from jungle.engine.apply import apply_move, terminal_after
from jungle.engine.movegen import legal_moves
from jungle.engine.types import Move, Outcome, PieceType, Result, Side, Square

from tests.conftest import pos_of


def _uci(pos):
    return {m.uci() for m in legal_moves(pos)}


def test_enemy_trap_reduces_rank_to_zero() -> None:
    # Gold elephant on scarlet trap c9 can be taken by scarlet rat on c8.
    pos = pos_of(
        ("c9", Side.GOLD, PieceType.ELEPHANT),
        ("c8", Side.SCARLET, PieceType.RAT),
        to_move=Side.SCARLET,
    )
    assert "c8c9" in _uci(pos)


def test_own_trap_does_not_reduce_rank() -> None:
    pos = pos_of(
        ("c1", Side.GOLD, PieceType.ELEPHANT),
        ("c2", Side.SCARLET, PieceType.CAT),
        to_move=Side.SCARLET,
    )
    assert "c2c1" not in _uci(pos)


def test_trapped_piece_cannot_capture_higher() -> None:
    # Gold lion on scarlet trap has rank 0 and cannot take a free scarlet cat.
    pos = pos_of(
        ("c9", Side.GOLD, PieceType.LION),
        ("c8", Side.SCARLET, PieceType.CAT),
    )
    assert "c9c8" not in _uci(pos)


def test_entering_enemy_den_wins() -> None:
    pos = pos_of(("d8", Side.GOLD, PieceType.DOG))
    nxt = apply_move(pos, Move.parse("d8d9"))
    result, outcome = terminal_after(nxt, Move.parse("d8d9"), [nxt.hash])
    assert result is Result.GOLD_WINS
    assert outcome is Outcome.DEN


def test_own_den_still_forbidden_when_empty() -> None:
    pos = pos_of(("d2", Side.GOLD, PieceType.DOG))
    assert "d2d1" not in _uci(pos)
    assert pos.piece_at(Square.parse("d1")) is None
