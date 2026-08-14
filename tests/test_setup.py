from jungle.engine.board import TERRAIN, is_den, is_trap, is_water
from jungle.engine.position import dump_fen, parse_fen, start_position
from jungle.engine.types import FILES, NSQUARES, RANKS, PieceType, Side, Square, Terrain


def test_board_dimensions() -> None:
    assert FILES == 7 and RANKS == 9 and NSQUARES == 63
    assert len(TERRAIN) == 63


def test_terrain_counts() -> None:
    waters = sum(1 for t in TERRAIN if t is Terrain.WATER)
    dens = sum(1 for t in TERRAIN if t in (Terrain.DEN_GOLD, Terrain.DEN_SCARLET))
    traps = sum(1 for t in TERRAIN if t in (Terrain.TRAP_GOLD, Terrain.TRAP_SCARLET))
    assert waters == 12
    assert dens == 2
    assert traps == 6


def test_named_terrain() -> None:
    assert is_den(Square.parse("d1"), Side.GOLD)
    assert is_den(Square.parse("d9"), Side.SCARLET)
    assert is_trap(Square.parse("c1"), Side.GOLD)
    assert is_trap(Square.parse("d2"), Side.GOLD)
    assert is_trap(Square.parse("e1"), Side.GOLD)
    assert is_trap(Square.parse("c9"), Side.SCARLET)
    assert is_trap(Square.parse("d8"), Side.SCARLET)
    assert is_trap(Square.parse("e9"), Side.SCARLET)
    for name in ("b4", "c4", "b5", "c5", "b6", "c6", "e4", "f4", "e5", "f5", "e6", "f6"):
        assert is_water(Square.parse(name))
    assert not is_water(Square.parse("a4"))
    assert not is_water(Square.parse("d5"))
    assert not is_water(Square.parse("g6"))


def test_start_has_sixteen_pieces() -> None:
    pos = start_position()
    assert pos.counts == [8, 8]
    assert pos.to_move is Side.GOLD


def test_start_squares() -> None:
    pos = start_position()
    expect = {
        "a1": (Side.GOLD, PieceType.TIGER),
        "g1": (Side.GOLD, PieceType.LION),
        "b2": (Side.GOLD, PieceType.CAT),
        "f2": (Side.GOLD, PieceType.DOG),
        "a3": (Side.GOLD, PieceType.ELEPHANT),
        "c3": (Side.GOLD, PieceType.WOLF),
        "e3": (Side.GOLD, PieceType.LEOPARD),
        "g3": (Side.GOLD, PieceType.RAT),
        "a9": (Side.SCARLET, PieceType.LION),
        "g9": (Side.SCARLET, PieceType.TIGER),
        "b8": (Side.SCARLET, PieceType.DOG),
        "f8": (Side.SCARLET, PieceType.CAT),
        "a7": (Side.SCARLET, PieceType.RAT),
        "c7": (Side.SCARLET, PieceType.LEOPARD),
        "e7": (Side.SCARLET, PieceType.WOLF),
        "g7": (Side.SCARLET, PieceType.ELEPHANT),
    }
    for name, (side, ptype) in expect.items():
        piece = pos.piece_at(Square.parse(name))
        assert piece is not None
        assert piece.side is side and piece.ptype is ptype


def test_start_fen_roundtrip() -> None:
    pos = start_position()
    fen = dump_fen(pos)
    assert fen == "l5t/1d3c1/r1p1w1e/7/7/7/E1W1P1R/1C3D1/T5L g"
    again = parse_fen(fen)
    assert dump_fen(again) == fen
    assert again.hash == pos.hash
    assert again.counts == pos.counts
