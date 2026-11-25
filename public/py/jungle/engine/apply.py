from __future__ import annotations

from jungle.engine.board import DEN_GOLD, DEN_SCARLET
from jungle.engine.movegen import legal_moves
from jungle.engine.position import Position
from jungle.engine.types import Move, Outcome, Result, Side
from jungle.engine.zobrist import side_key

HALFMOVE_DRAW = 100


def inspect_terminal(pos: Position, hashes: list[int]) -> tuple[Result, Outcome | None]:
    """Den, elimination, immobilization, repetition, then the 100-ply draw."""
    occ = pos.board[DEN_SCARLET]
    if occ is not None and occ.side is Side.GOLD:
        return Result.GOLD_WINS, Outcome.DEN
    occ = pos.board[DEN_GOLD]
    if occ is not None and occ.side is Side.SCARLET:
        return Result.SCARLET_WINS, Outcome.DEN
    if pos.counts[Side.GOLD] == 0 and pos.counts[Side.SCARLET] > 0:
        return Result.SCARLET_WINS, Outcome.ELIMINATION
    if pos.counts[Side.SCARLET] == 0 and pos.counts[Side.GOLD] > 0:
        return Result.GOLD_WINS, Outcome.ELIMINATION
    if not legal_moves(pos):
        if pos.to_move is Side.GOLD:
            return Result.SCARLET_WINS, Outcome.IMMOBILIZATION
        return Result.GOLD_WINS, Outcome.IMMOBILIZATION
    if hashes.count(pos.hash) >= 3:
        return Result.DRAW, Outcome.REPETITION
    if pos.halfmove >= HALFMOVE_DRAW:
        return Result.DRAW, Outcome.NO_CAPTURE
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
    del last_move
    return inspect_terminal(pos, history_hashes)
