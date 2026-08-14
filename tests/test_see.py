from jungle.ai.see import see_capture
from jungle.engine.types import Move, PieceType, Side

from tests.conftest import pos_of


def test_see_free_capture_is_positive() -> None:
    pos = pos_of(("a4", Side.GOLD, PieceType.TIGER), ("a5", Side.SCARLET, PieceType.WOLF))
    assert see_capture(pos, Move.parse("a4a5")) > 0


def test_see_equal_trade_near_zero() -> None:
    pos = pos_of(
        ("a4", Side.GOLD, PieceType.DOG),
        ("a5", Side.SCARLET, PieceType.DOG),
        ("b5", Side.SCARLET, PieceType.WOLF),
    )
    # Gold dog takes scarlet dog; scarlet cat recaptures.
    score = see_capture(pos, Move.parse("a4a5"))
    assert score <= 0


def test_see_elephant_takes_land_rat_is_bad() -> None:
    # Elephant cannot take a Rat on land — move is illegal so SEE is 0.
    pos = pos_of(
        ("d5", Side.GOLD, PieceType.ELEPHANT),
        ("d4", Side.SCARLET, PieceType.RAT),
    )
    assert see_capture(pos, Move.parse("d5d4")) == 0
