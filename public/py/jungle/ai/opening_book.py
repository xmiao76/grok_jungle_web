"""Tiny hash-keyed opening book. Built by replaying legal UCI lines."""

from __future__ import annotations

from jungle.engine.apply import apply_move
from jungle.engine.movegen import legal_moves
from jungle.engine.position import Position, start_position
from jungle.engine.types import Move, Side

_BOOK: dict[int, Move] | None = None

# Principled development: rats toward water, big cats toward banks.
_LINES: tuple[tuple[str, ...], ...] = (
    ("g3g4", "a7a6", "g4g5", "a6a5"),
    ("a1a2", "g9g8", "g1g2", "a9a8"),
    ("g1g2", "a9a8", "e3e4", "c7c6"),
    ("g3g4", "g9g8", "a1a2", "a7a6"),
    ("e3e2", "e7e8", "g3g4", "a7a6"),
)


def _build() -> dict[int, Move]:
    book: dict[int, Move] = {}
    for line in _LINES:
        pos = start_position(Side.GOLD)
        for uci in line:
            want = Move.parse(uci)
            legal = legal_moves(pos)
            move = next(
                (m for m in legal if m.src == want.src and m.dst == want.dst),
                None,
            )
            if move is None:
                break
            book.setdefault(pos.hash, move)
            pos = apply_move(pos, move)
    return book


def lookup(pos: Position) -> Move | None:
    global _BOOK
    if _BOOK is None:
        _BOOK = _build()
    move = _BOOK.get(pos.hash)
    if move is None:
        return None
    legal = legal_moves(pos)
    if any(m.src == move.src and m.dst == move.dst for m in legal):
        return move
    return None
