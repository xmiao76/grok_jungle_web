from __future__ import annotations

from jungle.engine.board import DEN_GOLD, DEN_SCARLET, enemy_den_index, is_water, trap_owner
from jungle.engine.position import Position, can_capture
from jungle.engine.types import FILES, NSQUARES, ORTHO, PieceType, Side, Square

# Rat is priced up: it kills the Elephant and swims. Elephant is priced down:
# it cannot capture a Rat.
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

MATE = 100_000
JUMP_BANKS = {r * FILES + f for r in (2, 6) for f in (1, 2, 4, 5)}
HORIZ_BANKS = {r * FILES + f for r in (3, 4, 5) for f in (0, 3, 6)}
RIVER_BANKS = {r * FILES + f for r in (3, 4, 5) for f in (0, 3, 6)} | {
    r * FILES + f for r in (2, 6) for f in (1, 2, 4, 5)
}


def _manhattan(a: int, b: int) -> int:
    return abs(a % FILES - b % FILES) + abs(a // FILES - b // FILES)


def _neighbors(idx: int) -> list[int]:
    sq = Square.from_index(idx)
    out: list[int] = []
    for df, dr in ORTHO:
        nxt = sq.step(df, dr)
        if nxt is not None:
            out.append(nxt.index())
    return out


_NEI = [_neighbors(i) for i in range(NSQUARES)]
_GOLD_APPROACH = [i for i in _NEI[DEN_SCARLET]]
_SCARLET_APPROACH = [i for i in _NEI[DEN_GOLD]]


def _build_approach() -> dict[tuple[Side, PieceType], list[int]]:
    tables: dict[tuple[Side, PieceType], list[int]] = {}
    for side in Side:
        den = enemy_den_index(side)
        for ptype in PieceType:
            arr = [0] * NSQUARES
            rush = 16 if ptype in (PieceType.RAT, PieceType.CAT, PieceType.DOG) else 11
            for idx in range(NSQUARES):
                dist = _manhattan(idx, den)
                arr[idx] = (16 - dist) * rush
                if dist == 1:
                    arr[idx] += 140
                if dist == 2:
                    arr[idx] += 50
                if ptype is PieceType.LION and idx in JUMP_BANKS:
                    arr[idx] += 42
                if ptype is PieceType.LION and idx in HORIZ_BANKS:
                    arr[idx] += 32
                if ptype is PieceType.TIGER and idx in HORIZ_BANKS:
                    arr[idx] += 40
                if ptype is PieceType.RAT and idx in RIVER_BANKS:
                    arr[idx] += 22
            tables[(side, ptype)] = arr
    return tables


_APPROACH = _build_approach()


def _cheap_mobility(pos: Position, side: Side) -> int:
    n = 0
    for idx, piece in pos.pieces(side):
        for nxt in _NEI[idx]:
            if pos.board[nxt] is None:
                n += 1
    return n


def _den_threat(pos: Position, side: Side) -> int:
    den = enemy_den_index(side)
    if pos.board[den] is not None:
        return 0
    ds = Square.from_index(den)
    for df, dr in ORTHO:
        adj = ds.step(df, dr)
        if adj is None:
            continue
        piece = pos.board[adj.index()]
        if piece is not None and piece.side is side:
            return 380
    return 0


def _approach_danger(pos: Position, owner: Side) -> int:
    """Danger to *owner*'s den from enemy pieces sitting on the approach."""
    den = DEN_GOLD if owner is Side.GOLD else DEN_SCARLET
    approach = _SCARLET_APPROACH if owner is Side.GOLD else _GOLD_APPROACH
    danger = 0
    for idx in approach:
        piece = pos.board[idx]
        if piece is None or piece.side is owner:
            continue
        recapture = False
        for nidx in _NEI[idx]:
            friend = pos.board[nidx]
            if friend is not None and friend.side is owner:
                recapture = True
                break
        danger += 40 if recapture else 130
    # Extra if the den itself is empty and no guard is adjacent.
    if pos.board[den] is None:
        guards = 0
        for nidx in _NEI[den]:
            p = pos.board[nidx]
            if p is not None and p.side is owner:
                guards += 1
        if guards == 0:
            danger += 35
    return danger


def _hanging_penalty(pos: Position, idx: int, piece) -> int:
    friends = 0
    threat = 0
    for nidx in _NEI[idx]:
        other = pos.board[nidx]
        if other is None:
            continue
        if other.side is piece.side:
            friends += 1
            continue
        if can_capture(pos, other, nidx, piece, idx):
            threat = max(threat, MATERIAL[piece.ptype] - MATERIAL[other.ptype] // 2)
    if threat and friends == 0:
        return min(threat, MATERIAL[piece.ptype] * 6 // 10)
    if threat and friends == 1:
        return min(threat, MATERIAL[piece.ptype] // 8)
    return 0


def evaluate(pos: Position) -> int:
    """Score from the side to move."""
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
            val += 36
        owner = trap_owner(idx)
        if owner is not None and owner is not piece.side:
            val -= 160
        elif owner is piece.side:
            val += 18
        if piece.ptype is PieceType.RAT:
            for nidx in _NEI[idx]:
                other = pos.board[nidx]
                if (
                    other is not None
                    and other.side is not piece.side
                    and other.ptype is PieceType.ELEPHANT
                    and not is_water(idx)
                ):
                    val += 130
        if piece.ptype is PieceType.ELEPHANT and not is_water(idx):
            for nidx in _NEI[idx]:
                other = pos.board[nidx]
                if (
                    other is not None
                    and other.side is not piece.side
                    and other.ptype is PieceType.RAT
                    and not is_water(nidx)
                ):
                    val -= 110
        val -= _hanging_penalty(pos, idx, piece)
        if piece.side is Side.GOLD:
            score += val
        else:
            score -= val

    score += _cheap_mobility(pos, Side.GOLD) * 3
    score -= _cheap_mobility(pos, Side.SCARLET) * 3
    gold_view = score + _den_threat(pos, Side.GOLD) * (4 if pos.to_move is Side.GOLD else 1)
    gold_view -= _den_threat(pos, Side.SCARLET) * (4 if pos.to_move is Side.SCARLET else 1)
    score = gold_view
    score -= _approach_danger(pos, Side.GOLD)
    score += _approach_danger(pos, Side.SCARLET)

    for side, den in ((Side.GOLD, DEN_GOLD), (Side.SCARLET, DEN_SCARLET)):
        guard = 0
        for nidx in _NEI[den]:
            piece = pos.board[nidx]
            if piece is not None and piece.side is side:
                guard += 22
        if side is Side.GOLD:
            score += guard
        else:
            score -= guard

    return score if pos.to_move is Side.GOLD else -score


def result_score(winner: Side | None, ply: int) -> int:
    if winner is None:
        return 0
    return MATE - ply
