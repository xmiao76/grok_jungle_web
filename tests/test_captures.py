from jungle.engine.apply import apply_move
from jungle.engine.movegen import legal_moves
from jungle.engine.types import Move, PieceType, Side, Square  # PieceType used for names + captures

from tests.conftest import pos_of


def _uci(pos):
    return {m.uci() for m in legal_moves(pos)}


def test_higher_captures_lower() -> None:
    pos = pos_of(("a1", Side.GOLD, PieceType.TIGER), ("a2", Side.SCARLET, PieceType.CAT))
    assert "a1a2" in _uci(pos)


def test_lower_cannot_capture_higher() -> None:
    pos = pos_of(("a1", Side.GOLD, PieceType.CAT), ("a2", Side.SCARLET, PieceType.TIGER))
    assert "a1a2" not in _uci(pos)


def test_equal_ranks_can_capture() -> None:
    pos = pos_of(("a1", Side.GOLD, PieceType.LION), ("a2", Side.SCARLET, PieceType.LION))
    assert "a1a2" in _uci(pos)


def test_rat_captures_elephant_from_land() -> None:
    pos = pos_of(("a1", Side.GOLD, PieceType.RAT), ("a2", Side.SCARLET, PieceType.ELEPHANT))
    assert "a1a2" in _uci(pos)


def test_elephant_cannot_capture_rat_on_land() -> None:
    pos = pos_of(("a1", Side.GOLD, PieceType.ELEPHANT), ("a2", Side.SCARLET, PieceType.RAT))
    assert "a1a2" not in _uci(pos)


def test_elephant_cannot_capture_rat_even_when_adjacent_higher() -> None:
    pos = pos_of(("d5", Side.GOLD, PieceType.ELEPHANT), ("d4", Side.SCARLET, PieceType.RAT))
    assert "d5d4" not in _uci(pos)
    assert "d5d6" in _uci(pos)


def test_elephant_can_capture_trapped_rat() -> None:
    # Scarlet rat on Gold's trap c1 — rank 0, any Gold piece including Elephant may take it.
    pos = pos_of(
        ("c2", Side.GOLD, PieceType.ELEPHANT),
        ("c1", Side.SCARLET, PieceType.RAT),
    )
    assert "c2c1" in _uci(pos)


def test_rat_cannot_capture_elephant_from_water() -> None:
    pos = pos_of(("b4", Side.GOLD, PieceType.RAT), ("a4", Side.SCARLET, PieceType.ELEPHANT))
    assert "b4a4" not in _uci(pos)
    # but the rat may step onto other land / water
    assert "b4b5" in _uci(pos)
    assert "b4c4" in _uci(pos)


def test_piece_short_names() -> None:
    assert PieceType.RAT.short_name == "Rat"
    assert PieceType.ELEPHANT.short_name == "Elephant"
    assert PieceType.LEOPARD.short_name == "Leopard"


def test_capture_removes_piece() -> None:
    pos = pos_of(("a1", Side.GOLD, PieceType.DOG), ("a2", Side.SCARLET, PieceType.CAT))
    nxt = apply_move(pos, Move.parse("a1a2"))
    assert nxt.piece_at(Square.parse("a2")).ptype is PieceType.DOG
    assert nxt.counts[Side.SCARLET] == 0
    assert nxt.halfmove == 0
