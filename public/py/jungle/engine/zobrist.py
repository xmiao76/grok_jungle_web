from __future__ import annotations

import random

from jungle.engine.types import NSQUARES, Piece, PieceType, Side

_rng = random.Random(0xD005_50_01)


def _rand() -> int:
    return _rng.getrandbits(64)


_PIECE = {
    (side, ptype, sq): _rand()
    for side in Side
    for ptype in PieceType
    for sq in range(NSQUARES)
}
_SIDE = _rand()


def piece_key(piece: Piece, square_index: int) -> int:
    return _PIECE[(piece.side, piece.ptype, square_index)]


def side_key() -> int:
    return _SIDE
