from jungle.engine.position import Position, empty_position
from jungle.engine.types import Piece, PieceType, Side, Square


def place(pos: Position, name: str, side: Side, ptype: PieceType) -> None:
    pos.place(Square.parse(name), Piece(side, ptype))


def pos_of(*entries: tuple[str, Side, PieceType], to_move: Side = Side.GOLD) -> Position:
    pos = empty_position(to_move)
    for name, side, ptype in entries:
        place(pos, name, side, ptype)
    return pos
