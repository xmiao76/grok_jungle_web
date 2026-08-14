from __future__ import annotations

from jungle.engine.board import HORIZONTAL_JUMPS, VERTICAL_JUMPS
from jungle.engine.position import Position, can_land
from jungle.engine.types import FILES, Move, ORTHO, PieceType, Square


def _rat_blocks(pos: Position, waters: tuple[int, ...]) -> bool:
    for idx in waters:
        piece = pos.board[idx]
        if piece is not None and piece.ptype is PieceType.RAT:
            return True
    return False


def _add_jumps(pos: Position, src: int, piece_type: PieceType, out: list[Move]) -> None:
    # Two-block jump = across 2 water squares (horizontal a↔d, d↔g).
    # Three-block jump = across 3 water squares (vertical rank 3↔7).
    # Tiger: two-block only. Lion: both.
    if piece_type is PieceType.LION:
        tables = [HORIZONTAL_JUMPS, VERTICAL_JUMPS]
    elif piece_type is PieceType.TIGER:
        tables = [HORIZONTAL_JUMPS]
    else:
        return
    piece = pos.board[src]
    assert piece is not None
    for table in tables:
        for a, b, waters in table:
            if a != src:
                continue
            if _rat_blocks(pos, waters):
                continue
            if can_land(pos, piece, src, b):
                out.append(Move(Square.from_index(src), Square.from_index(b), is_jump=True))


def legal_moves(pos: Position) -> list[Move]:
    moves: list[Move] = []
    for src, piece in pos.pieces(pos.to_move):
        sq = Square.from_index(src)
        for df, dr in ORTHO:
            dst_sq = sq.step(df, dr)
            if dst_sq is None:
                continue
            dst = dst_sq.index()
            if can_land(pos, piece, src, dst):
                moves.append(Move(sq, dst_sq, is_jump=False))
        if piece.ptype in (PieceType.LION, PieceType.TIGER):
            _add_jumps(pos, src, piece.ptype, moves)
    return moves


def is_legal(pos: Position, move: Move) -> bool:
    return any(
        m.src == move.src and m.dst == move.dst and m.is_jump == move.is_jump
        for m in legal_moves(pos)
    )


def perft(pos: Position, depth: int) -> int:
    if depth <= 0:
        return 1
    moves = legal_moves(pos)
    if depth == 1:
        return len(moves)
    from jungle.engine.apply import apply_move

    total = 0
    for move in moves:
        total += perft(apply_move(pos, move), depth - 1)
    return total


# Silence unused import if FILES is wanted by tests via this module
_ = FILES
