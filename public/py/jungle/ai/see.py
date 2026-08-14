"""Static exchange evaluation for Jungle captures.

Uses engine can_capture — no rule fork. Returns net material for the side
that just played `move`, assuming both sides recapture with the cheapest
legal attacker.
"""

from __future__ import annotations

from jungle.ai.eval import MATERIAL
from jungle.engine.board import HORIZONTAL_JUMPS, VERTICAL_JUMPS
from jungle.engine.position import Position, can_capture
from jungle.engine.types import ORTHO, Piece, PieceType, Square


def _material(piece: Piece) -> int:
    return MATERIAL[piece.ptype]


def _attackers(pos: Position, dst: int, side, removed: set[int]) -> list[tuple[int, Piece]]:
    found: list[tuple[int, Piece]] = []
    target = Square.from_index(dst)
    for df, dr in ORTHO:
        src_sq = target.step(df, dr)
        if src_sq is None:
            continue
        src = src_sq.index()
        if src in removed:
            continue
        piece = pos.board[src]
        if piece is None or piece.side is not side:
            continue
        occupant = pos.board[dst]
        if occupant is None:
            continue
        if can_capture(pos, piece, src, occupant, dst):
            found.append((src, piece))
    for src, piece in pos.pieces(side):
        if src in removed or piece.ptype not in (PieceType.LION, PieceType.TIGER):
            continue
        tables = [HORIZONTAL_JUMPS]
        if piece.ptype is PieceType.LION:
            tables.append(VERTICAL_JUMPS)
        for table in tables:
            for a, b, waters in table:
                if a != src or b != dst:
                    continue
                if any(
                    idx not in removed
                    and pos.board[idx] is not None
                    and pos.board[idx].ptype is PieceType.RAT
                    for idx in waters
                ):
                    continue
                occupant = pos.board[dst]
                if occupant is None:
                    continue
                if can_capture(pos, piece, src, occupant, dst):
                    found.append((src, piece))
    found.sort(key=lambda t: int(t[1].ptype))
    return found


def see_capture(pos: Position, move) -> int:
    """Net material for the mover after an optimal recapture sequence."""
    src, dst = move.src.index(), move.dst.index()
    attacker = pos.board[src]
    victim = pos.board[dst]
    if attacker is None or victim is None:
        return 0
    if not can_capture(pos, attacker, src, victim, dst):
        return 0
    gains = [_material(victim)]
    removed = {src}
    on_square = attacker
    side = attacker.side.other()
    # Pretend the attacker now occupies dst for recapture legality.
    board_save = pos.board[dst]
    pos.board[dst] = attacker
    try:
        while True:
            cands = _attackers(pos, dst, side, removed)
            if not cands:
                break
            nxt_src, nxt = cands[0]
            gains.append(_material(on_square))
            removed.add(nxt_src)
            on_square = nxt
            pos.board[dst] = nxt
            side = side.other()
    finally:
        pos.board[dst] = board_save

    # Negamax the gain list from the end.
    score = 0
    for g in reversed(gains):
        score = g - max(0, score)
    return score
