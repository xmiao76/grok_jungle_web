"""Frozen desktop-style evaluator for the strength gate."""

from __future__ import annotations

from jungle.engine.board import DEN_GOLD, DEN_SCARLET, enemy_den_index, is_water, trap_owner
from jungle.engine.types import FILES, NSQUARES, ORTHO, PieceType, Side, Square

MATERIAL = {
    PieceType.RAT: 180,
    PieceType.CAT: 220,
    PieceType.DOG: 320,
    PieceType.WOLF: 420,
    PieceType.LEOPARD: 540,
    PieceType.TIGER: 780,
    PieceType.LION: 920,
    PieceType.ELEPHANT: 760,
}

JUMP_BANKS = {r * FILES + f for r in (2, 6) for f in (1, 2, 4, 5)}
HORIZ_BANKS = {r * FILES + f for r in (3, 4, 5) for f in (0, 3, 6)}


def _manhattan(a: int, b: int) -> int:
    return abs(a % FILES - b % FILES) + abs(a // FILES - b // FILES)


def _build_approach():
    tables = {}
    for side in Side:
        den = enemy_den_index(side)
        for ptype in PieceType:
            arr = [0] * NSQUARES
            rush = 14 if ptype in (PieceType.RAT, PieceType.CAT, PieceType.DOG) else 10
            for idx in range(NSQUARES):
                dist = _manhattan(idx, den)
                arr[idx] = (16 - dist) * rush
                if dist == 1:
                    arr[idx] += 120
                if dist == 2:
                    arr[idx] += 40
                if ptype is PieceType.LION and idx in JUMP_BANKS:
                    arr[idx] += 36
                if ptype is PieceType.LION and idx in HORIZ_BANKS:
                    arr[idx] += 28
                if ptype is PieceType.TIGER and idx in HORIZ_BANKS:
                    arr[idx] += 36
            tables[(side, ptype)] = arr
    return tables


_APPROACH = _build_approach()


def evaluate(pos) -> int:
    score = 0
    remaining = pos.counts[0] + pos.counts[1]
    endgame = remaining <= 8
    for idx, piece in enumerate(pos.board):
        if piece is None:
            continue
        val = MATERIAL[piece.ptype]
        approach = _APPROACH[(piece.side, piece.ptype)][idx]
        if endgame:
            approach = int(approach * 1.45)
        val += approach
        if piece.ptype is PieceType.RAT and is_water(idx):
            val += 28
        owner = trap_owner(idx)
        if owner is not None and owner is not piece.side:
            val -= 110
        elif owner is piece.side:
            val += 12
        if piece.ptype is PieceType.RAT:
            sq = Square.from_index(idx)
            for df, dr in ORTHO:
                nxt = sq.step(df, dr)
                if nxt is None:
                    continue
                other = pos.board[nxt.index()]
                if (
                    other is not None
                    and other.side is not piece.side
                    and other.ptype is PieceType.ELEPHANT
                    and not is_water(idx)
                ):
                    val += 90
        if piece.side is Side.GOLD:
            score += val
        else:
            score -= val
    for side, den in ((Side.GOLD, DEN_GOLD), (Side.SCARLET, DEN_SCARLET)):
        guard = 0
        ds = Square.from_index(den)
        for df, dr in ORTHO:
            adj = ds.step(df, dr)
            if adj is None:
                continue
            piece = pos.board[adj.index()]
            if piece is not None and piece.side is side:
                guard += 18
        if side is Side.GOLD:
            score += guard
        else:
            score -= guard
    return score if pos.to_move is Side.GOLD else -score
