from __future__ import annotations

from jungle.engine.board import DEN_GOLD, DEN_SCARLET, enemy_den_index
from jungle.engine.movegen import legal_moves
from jungle.engine.position import Position
from jungle.engine.types import Move, Outcome, Result, Side
from jungle.engine.zobrist import side_key

HALFMOVE_DRAW = 100


def inspect_terminal(pos: Position, hashes: list[int]) -> tuple[Result, Outcome | None]:
    """Terminal check that does not need the last move (used by search)."""
    occ = pos.board[DEN_SCARLET]
    if occ is not None and occ.side is Side.GOLD:
        return Result.GOLD_WINS, Outcome.DEN
    occ = pos.board[DEN_GOLD]
    if occ is not None and occ.side is Side.SCARLET:
        return Result.SCARLET_WINS, Outcome.DEN
    if pos.counts[Side.GOLD] == 0:
        return Result.SCARLET_WINS, Outcome.WIPE
    if pos.counts[Side.SCARLET] == 0:
        return Result.GOLD_WINS, Outcome.WIPE
    if hashes.count(pos.hash) >= 3:
        return Result.DRAW, Outcome.REPETITION
    if pos.halfmove >= HALFMOVE_DRAW:
        return Result.DRAW, Outcome.FIFTY
    return Result.ONGOING, None


def apply_move(pos: Position, move: Move) -> Position:
    nxt = pos.copy()
    src, dst = move.src.index(), move.dst.index()
    piece = nxt.board[src]
    if piece is None:
        raise ValueError(f"no piece on {move.src.name()}")
    captured = nxt.board[dst]
    nxt.place(src, None)
    nxt.place(dst, piece)
    nxt.halfmove = 0 if captured is not None else nxt.halfmove + 1
    nxt.to_move = piece.side.other()
    nxt.hash ^= side_key()
    return nxt


def terminal_after(pos: Position, last_move: Move | None, history_hashes: list[int]) -> tuple[Result, Outcome | None]:
    result, outcome = inspect_terminal(pos, history_hashes)
    if result is not Result.ONGOING:
        return result, outcome
    if last_move is not None:
        mover = pos.board[last_move.dst.index()]
        if mover is not None and last_move.dst.index() == enemy_den_index(mover.side):
            win = Result.GOLD_WINS if mover.side is Side.GOLD else Result.SCARLET_WINS
            return win, Outcome.DEN
    if not legal_moves(pos):
        win = Result.GOLD_WINS if pos.to_move is Side.SCARLET else Result.SCARLET_WINS
        return win, Outcome.NO_MOVES
    return Result.ONGOING, None
