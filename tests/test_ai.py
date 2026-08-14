import time

from jungle.ai.search import Limits, best_move
from jungle.engine.apply import apply_move
from jungle.engine.game import Game, GameConfig
from jungle.engine.movegen import legal_moves
from jungle.engine.position import start_position
from jungle.engine.types import PieceType, Result, Side

from tests.conftest import pos_of


def test_ai_move_is_legal() -> None:
    pos = start_position()
    move = best_move(pos, Limits(move_time_ms=50, max_depth=3))
    assert any(m.src == move.src and m.dst == move.dst for m in legal_moves(pos))


def test_ai_takes_open_den() -> None:
    pos = pos_of(("d8", Side.GOLD, PieceType.DOG), ("a9", Side.SCARLET, PieceType.CAT))
    move = best_move(pos, Limits(move_time_ms=200, max_depth=4))
    assert move.src.name() == "d8" and move.dst.name() == "d9"


def test_ai_captures_free_piece() -> None:
    pos = pos_of(("a4", Side.GOLD, PieceType.TIGER), ("a5", Side.SCARLET, PieceType.WOLF))
    move = best_move(pos, Limits(move_time_ms=200, max_depth=4))
    assert move.dst.name() == "a5"


def test_ai_respects_time_cap() -> None:
    pos = start_position()
    t0 = time.perf_counter()
    best_move(pos, Limits(move_time_ms=40, max_depth=12, hard_cap_ms=200))
    assert time.perf_counter() - t0 < 0.6


def test_applied_ai_move_changes_side() -> None:
    pos = start_position()
    move = best_move(pos, Limits(move_time_ms=40, max_depth=3))
    nxt = apply_move(pos, move)
    assert nxt.to_move is Side.SCARLET
