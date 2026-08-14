from jungle.engine.movegen import legal_moves
from jungle.engine.types import PieceType, Side

from tests.conftest import pos_of


def _uci(pos):
    return {m.uci() for m in legal_moves(pos)}


def test_lion_vertical_jump() -> None:
    pos = pos_of(("b3", Side.GOLD, PieceType.LION))
    uci = _uci(pos)
    assert "b3b7j" in uci
    assert "b3b4" not in uci  # water, not a step


def test_lion_horizontal_jump() -> None:
    pos = pos_of(("a4", Side.GOLD, PieceType.LION))
    uci = _uci(pos)
    assert "a4d4j" in uci
    assert "a4b4" not in uci


def test_tiger_two_block_only() -> None:
    # Two-block = horizontal over 2 water (a4→d4). Three-block = vertical over 3 water (b3→b7).
    pos = pos_of(("a4", Side.GOLD, PieceType.TIGER))
    uci = _uci(pos)
    assert "a4d4j" in uci
    assert "a4b4" not in uci

    pos2 = pos_of(("b3", Side.GOLD, PieceType.TIGER))
    uci2 = _uci(pos2)
    assert "b3b7j" not in uci2
    assert "b3b4" not in uci2


def test_scarlet_lion_both_ways() -> None:
    pos = pos_of(("g6", Side.SCARLET, PieceType.LION), to_move=Side.SCARLET)
    uci = _uci(pos)
    assert "g6d6j" in uci
    assert "g6g3" not in {m.uci().replace("j", "") for m in legal_moves(pos) if m.src.name() == "g6" and m.dst.name() == "g3" and not m.is_jump}
    # vertical from g6 is not a river file — g is land. Horizontal only here.
    pos_v = pos_of(("f7", Side.SCARLET, PieceType.LION), to_move=Side.SCARLET)
    assert "f7f3j" in _uci(pos_v)


def test_rat_blocks_own_and_enemy() -> None:
    pos = pos_of(
        ("b3", Side.GOLD, PieceType.LION),
        ("b5", Side.GOLD, PieceType.RAT),
    )
    assert "b3b7j" not in _uci(pos)

    pos2 = pos_of(
        ("b3", Side.GOLD, PieceType.LION),
        ("b6", Side.SCARLET, PieceType.RAT),
    )
    assert "b3b7j" not in _uci(pos2)


def test_jump_capture_on_bank() -> None:
    pos = pos_of(
        ("b3", Side.GOLD, PieceType.LION),
        ("b7", Side.SCARLET, PieceType.WOLF),
    )
    assert "b3b7j" in _uci(pos)

    pos2 = pos_of(
        ("a4", Side.GOLD, PieceType.TIGER),
        ("d4", Side.SCARLET, PieceType.WOLF),
    )
    assert "a4d4j" in _uci(pos2)

    pos3 = pos_of(
        ("b3", Side.GOLD, PieceType.TIGER),
        ("b7", Side.SCARLET, PieceType.WOLF),
    )
    assert "b3b7j" not in _uci(pos3)


def test_jump_blocked_by_friend_on_bank() -> None:
    pos = pos_of(
        ("b3", Side.GOLD, PieceType.LION),
        ("b7", Side.GOLD, PieceType.CAT),
    )
    assert "b3b7j" not in _uci(pos)


def test_all_horizontal_lion_banks() -> None:
    for rank in (4, 5, 6):
        pos = pos_of((f"a{rank}", Side.GOLD, PieceType.LION))
        assert f"a{rank}d{rank}j" in _uci(pos)
        pos = pos_of((f"d{rank}", Side.GOLD, PieceType.LION))
        uci = _uci(pos)
        assert f"d{rank}a{rank}j" in uci
        assert f"d{rank}g{rank}j" in uci


def test_tiger_two_block_every_river_rank() -> None:
    for rank in (4, 5, 6):
        pos = pos_of((f"a{rank}", Side.GOLD, PieceType.TIGER))
        assert f"a{rank}d{rank}j" in _uci(pos)


def test_tiger_cannot_three_block_any_river_file() -> None:
    for file in ("b", "c", "e", "f"):
        pos = pos_of((f"{file}3", Side.GOLD, PieceType.TIGER))
        assert f"{file}3{file}7j" not in _uci(pos)


def test_lion_both_two_and_three_block() -> None:
    two = pos_of(("d5", Side.GOLD, PieceType.LION))
    uci = _uci(two)
    assert "d5a5j" in uci
    assert "d5g5j" in uci
    three = pos_of(("c3", Side.GOLD, PieceType.LION))
    assert "c3c7j" in _uci(three)
