from jungle.ai.eval import evaluate
from jungle.engine.types import PieceType, Side

from tests.conftest import pos_of


def test_rat_beside_elephant_is_better_than_isolated() -> None:
    threat = pos_of(
        ("d4", Side.GOLD, PieceType.RAT),
        ("d5", Side.SCARLET, PieceType.ELEPHANT),
    )
    quiet = pos_of(
        ("a1", Side.GOLD, PieceType.RAT),
        ("d5", Side.SCARLET, PieceType.ELEPHANT),
    )
    assert evaluate(threat) > evaluate(quiet)


def test_undefended_den_approach_is_dangerous() -> None:
    # Scarlet piece on d2 (gold trap, next to gold den) with no gold recapture.
    danger = pos_of(
        ("d2", Side.SCARLET, PieceType.DOG),
        ("a9", Side.GOLD, PieceType.CAT),
        to_move=Side.GOLD,
    )
    safe = pos_of(
        ("d2", Side.SCARLET, PieceType.DOG),
        ("c2", Side.GOLD, PieceType.CAT),
        to_move=Side.GOLD,
    )
    assert evaluate(danger) < evaluate(safe)
