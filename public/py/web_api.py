"""JSON bridge between the JS Web Worker and the Jungle engine/AI.

Every public function returns a JSON string with the envelope
{"ok": bool, "data": ..., "error": str | None} so the JS side never has
to hold PyProxy objects — only plain strings cross the boundary.

The module owns a single game session (one worker == one game at a time),
mirroring the desktop MainWindow's wiring of Game + Searcher.
"""

from __future__ import annotations

import json
import sys
from collections import Counter

from jungle import __version__
from jungle.ai.search import Limits, Searcher
from jungle.engine.game import Game, GameConfig
from jungle.engine.position import START_PLACEMENT
from jungle.engine.types import Move, Outcome, PieceType, Result, Side, Square

# Browser budgets: Pyodide is slower than CPython. search.py is untouched.
WEB_LIMITS = {
    "easy": Limits(150, max_depth=2, noise=0.30, hard_cap_ms=330),
    "normal": Limits(1400, max_depth=10, noise=0.0, hard_cap_ms=2800),
    "strong": Limits(3500, max_depth=14, noise=0.0, hard_cap_ms=5500),
}

_TT_CAP = 50_000


class WebSearcher(Searcher):
    def clear_if_huge(self) -> None:
        if len(self.tt) > _TT_CAP:
            self.tt.clear()


class _Session:
    def __init__(self) -> None:
        self.game = Game(GameConfig())
        self.searcher = WebSearcher()

    def start(self, mode: str, human_first: bool, difficulty: str) -> None:
        self.game.reset(
            GameConfig(mode=mode, human_first=human_first, difficulty=difficulty)
        )


_session = _Session()


def _ok(data: dict) -> str:
    return json.dumps({"ok": True, "data": data, "error": None})


def _err(message: str) -> str:
    return json.dumps({"ok": False, "data": None, "error": message})


def _side_name(side: Side) -> str:
    return "gold" if side is Side.GOLD else "scarlet"


def _move_dict(move: Move) -> dict:
    return {
        "src": move.src.name(),
        "dst": move.dst.name(),
        "isJump": bool(move.is_jump),
        "uci": move.uci(),
    }


def _find_move(game: Game, src: str, dst: str) -> Move | None:
    src = src.strip().lower()
    dst = dst.strip().lower()
    for move in game.legal():
        if move.src.name() == src and move.dst.name() == dst:
            return move
    return None


def _captured_lists(game: Game) -> dict[str, list[str]]:
    start: Counter[tuple[Side, PieceType]] = Counter()
    now: Counter[tuple[Side, PieceType]] = Counter()
    for _, side, ptype in START_PLACEMENT:
        start[(side, ptype)] += 1
    for _, piece in game.position.pieces():
        now[(piece.side, piece.ptype)] += 1
    out: dict[str, list[str]] = {"gold": [], "scarlet": []}
    for side in Side:
        missing: list[str] = []
        for ptype in PieceType:
            lost = start[(side, ptype)] - now[(side, ptype)]
            missing.extend([ptype.noun] * lost)
        out[_side_name(side)] = missing
    return out


def _state_dict(game: Game) -> dict:
    board = []
    for idx, piece in game.position.pieces():
        sq = Square.from_index(idx)
        board.append(
            {
                "sq": sq.name(),
                "side": _side_name(piece.side),
                "type": piece.ptype.noun.lower(),
                "rank": int(piece.ptype),
            }
        )
    last = _move_dict(game.moves[-1]) if game.moves else None
    legal = [] if game.result is not Result.ONGOING else [_move_dict(m) for m in game.legal()]
    return {
        "board": board,
        "turn": _side_name(game.position.to_move),
        "ply": len(game.moves),
        "legalMoves": legal,
        "lastMove": last,
        "result": game.result.value,
        "outcome": game.outcome.value if game.outcome is not None else None,
        "terminal": game.result is not Result.ONGOING,
        "fen": game.fen(),
        "history": [m.uci() for m in game.moves],
        "captured": _captured_lists(game),
        "mode": game.config.mode,
        "humanFirst": game.config.human_first,
        "humanSide": _side_name(game.config.human_side),
        "difficulty": game.config.difficulty,
    }


def _commit(game: Game, move: Move) -> dict:
    captured_piece = game.position.piece_at(move.dst)
    captured = None
    if captured_piece is not None:
        captured = {
            "side": _side_name(captured_piece.side),
            "type": captured_piece.ptype.noun.lower(),
            "rank": int(captured_piece.ptype),
        }
    game.play(move)
    return {"move": _move_dict(move), "captured": captured, "state": _state_dict(game)}


def engine_info() -> str:
    return _ok(
        {
            "engineVersion": __version__,
            "python": sys.version.split()[0],
        }
    )


def new_game(mode: str = "human_ai", human_first: bool = True, difficulty: str = "normal") -> str:
    mode = str(mode)
    difficulty = str(difficulty)
    if mode not in ("human_ai", "ai_ai"):
        return _err(f"invalid mode: {mode}")
    if difficulty not in WEB_LIMITS:
        return _err(f"invalid difficulty: {difficulty}")
    _session.start(mode, bool(human_first), difficulty)
    return _ok({"state": _state_dict(_session.game)})


def get_state() -> str:
    return _ok({"state": _state_dict(_session.game)})


def apply_move(src: str, dst: str) -> str:
    game = _session.game
    if game.result is not Result.ONGOING:
        return _err("game is over")
    move = _find_move(game, str(src), str(dst))
    if move is None:
        return _err(f"illegal move: {src}->{dst}")
    return _ok(_commit(game, move))


def ai_move(time_budget_ms: int | None = None) -> str:
    game = _session.game
    if game.result is not Result.ONGOING:
        return _err("game is over")
    legal = game.legal()
    if not legal:
        return _err("no legal moves")
    limits = WEB_LIMITS[game.config.difficulty]
    if time_budget_ms is not None:
        budget = int(time_budget_ms)
        limits = Limits(budget, max_depth=limits.max_depth, noise=limits.noise)
    move = _session.searcher.search(game.position, limits, game.hashes)
    if not any(m.src == move.src and m.dst == move.dst for m in legal):
        return _err(f"AI proposed illegal move {move.uci()}")
    return _ok(_commit(game, move))


def undo() -> str:
    game = _session.game
    if not game.moves:
        return _err("nothing to undo")
    plies = 1
    if game.config.mode == "human_ai" and len(game.moves) >= 2:
        plies = 2
    game.undo(plies)
    return _ok({"state": _state_dict(game)})


def resign() -> str:
    game = _session.game
    if game.result is not Result.ONGOING:
        return _err("game is over")
    side = game.config.human_side if game.config.mode == "human_ai" else game.position.to_move
    game.resign(side)
    return _ok({"state": _state_dict(game)})


def replay_moves(moves_json: str) -> str:
    try:
        moves = json.loads(moves_json)
    except (TypeError, ValueError) as exc:
        return _err(f"bad moves payload: {exc}")
    game = _session.game
    applied = 0
    for entry in moves:
        if game.result is not Result.ONGOING:
            break
        if isinstance(entry, str):
            src, dst = entry[:2], entry[2:4]
        else:
            src, dst = entry[0], entry[1]
        move = _find_move(game, str(src), str(dst))
        if move is None:
            return _err(f"illegal move at ply {applied}: {src}->{dst}")
        game.play(move)
        applied += 1
    return _ok({"applied": applied, "state": _state_dict(game)})


# Silence unused import if a caller introspects outcomes.
_ = (Outcome,)
