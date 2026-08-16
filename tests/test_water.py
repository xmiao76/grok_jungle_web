from jungle.engine.movegen import legal_moves
from jungle.engine.types import PieceType, Side

from tests.conftest import pos_of


def _uci(pos):
    return {m.uci() for m in legal_moves(pos)}


def test_only_rat_enters_water() -> None:
    pos = pos_of(("a4", Side.GOLD, PieceType.DOG))
    assert "a4b4" not in _uci(pos)
    rat = pos_of(("a4", Side.GOLD, PieceType.RAT))
    assert "a4b4" in _uci(rat)


def test_rat_can_leave_water() -> None:
    pos = pos_of(("b4", Side.GOLD, PieceType.RAT))
    uci = _uci(pos)
    assert "b4a4" in uci
    assert "b4b5" in uci


def test_water_rat_immune_to_land_piece() -> None:
    pos = pos_of(("a4", Side.GOLD, PieceType.LION), ("b4", Side.SCARLET, PieceType.RAT))
    assert "a4b4" not in _uci(pos)


def test_water_rat_captures_water_rat() -> None:
    pos = pos_of(("b4", Side.GOLD, PieceType.RAT), ("c4", Side.SCARLET, PieceType.RAT))
    assert "b4c4" in _uci(pos)


def test_land_rat_cannot_capture_water_rat() -> None:
    pos = pos_of(("a4", Side.GOLD, PieceType.RAT), ("b4", Side.SCARLET, PieceType.RAT))
    assert "a4b4" not in _uci(pos)


def test_water_rat_cannot_capture_land_rat() -> None:
    pos = pos_of(("b4", Side.GOLD, PieceType.RAT), ("a4", Side.SCARLET, PieceType.RAT))
    assert "b4a4" not in _uci(pos)
    # Leaving the river onto empty land is still legal.
    assert "b4b3" in _uci(pos)


def test_water_rat_cannot_capture_land_cat() -> None:
    pos = pos_of(("b4", Side.GOLD, PieceType.RAT), ("a4", Side.SCARLET, PieceType.CAT))
    assert "b4a4" not in _uci(pos)


def test_lion_cannot_step_on_water() -> None:
    pos = pos_of(("a5", Side.GOLD, PieceType.LION))
    uci = _uci(pos)
    assert "a5b5" not in uci
    assert "a5a4" in uci
    assert "a5a6" in uci
