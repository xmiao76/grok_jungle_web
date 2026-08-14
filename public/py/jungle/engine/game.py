from __future__ import annotations

from dataclasses import dataclass, field

from jungle.engine.apply import apply_move, terminal_after
from jungle.engine.movegen import is_legal, legal_moves
from jungle.engine.position import Position, dump_fen, start_position
from jungle.engine.types import Move, Outcome, Result, Side


@dataclass
class GameConfig:
    """Who plays which role. Flip is display-only and lives on ViewState."""

    mode: str = "human_ai"  # human_ai | ai_ai
    human_first: bool = True
    difficulty: str = "normal"  # easy | normal | strong
    human_side: Side = Side.GOLD

    @property
    def first_side(self) -> Side:
        if self.human_first:
            return self.human_side
        return self.human_side.other()

    def is_ai_side(self, side: Side) -> bool:
        if self.mode == "ai_ai":
            return True
        return side is not self.human_side


@dataclass
class ViewState:
    """Display options that must never mutate GameState."""

    flipped: bool = False
    debug_coords: bool = False


@dataclass
class Game:
    config: GameConfig = field(default_factory=GameConfig)
    view: ViewState = field(default_factory=ViewState)
    position: Position = field(init=False)
    history: list[Position] = field(default_factory=list)
    moves: list[Move] = field(default_factory=list)
    hashes: list[int] = field(default_factory=list)
    result: Result = Result.ONGOING
    outcome: Outcome | None = None

    def __post_init__(self) -> None:
        if not self.history:
            self.reset()

    def reset(self, config: GameConfig | None = None) -> None:
        if config is not None:
            self.config = config
        self.position = start_position(self.config.first_side)
        self.history = [self.position.copy()]
        self.moves = []
        self.hashes = [self.position.hash]
        self.result = Result.ONGOING
        self.outcome = None
        self._refresh_terminal()

    def legal(self) -> list[Move]:
        if self.result is not Result.ONGOING:
            return []
        return legal_moves(self.position)

    def play(self, move: Move) -> None:
        if self.result is not Result.ONGOING:
            raise ValueError("game is over")
        if not is_legal(self.position, move):
            raise ValueError(f"illegal move {move.uci()}")
        self.position = apply_move(self.position, move)
        self.moves.append(move)
        self.history.append(self.position.copy())
        self.hashes.append(self.position.hash)
        self._refresh_terminal()

    def undo(self, plies: int = 1) -> None:
        plies = min(plies, len(self.moves))
        if plies <= 0:
            return
        keep = len(self.history) - plies
        self.history = self.history[:keep]
        self.moves = self.moves[: len(self.moves) - plies]
        self.hashes = self.hashes[:keep]
        self.position = self.history[-1].copy()
        self.result = Result.ONGOING
        self.outcome = None
        self._refresh_terminal()

    def resign(self, side: Side) -> None:
        if self.result is not Result.ONGOING:
            return
        self.result = Result.SCARLET_WINS if side is Side.GOLD else Result.GOLD_WINS
        self.outcome = Outcome.RESIGN

    def flip_view(self) -> None:
        self.view.flipped = not self.view.flipped

    def fen(self) -> str:
        return dump_fen(self.position)

    def snapshot_key(self) -> tuple:
        """Identity of game state used to prove flip does not mutate it."""
        return (
            tuple(p.fen_char() if p else "." for p in self.position.board),
            int(self.position.to_move),
            self.position.halfmove,
            self.position.hash,
            self.result.value,
        )

    def _refresh_terminal(self) -> None:
        last = self.moves[-1] if self.moves else None
        self.result, self.outcome = terminal_after(self.position, last, self.hashes)
