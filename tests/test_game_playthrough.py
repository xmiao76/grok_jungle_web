import random

import pytest

from jungle.ai.search import Limits, best_move
from jungle.engine.game import Game, GameConfig
from jungle.engine.movegen import legal_moves
from jungle.engine.types import Result


def _random_game(rng: random.Random, max_plies: int = 400) -> Game:
    game = Game(GameConfig(human_first=rng.choice([True, False])))
    while game.result is Result.ONGOING and len(game.moves) < max_plies:
        moves = legal_moves(game.position)
        assert moves, "ongoing game must have a move"
        game.play(rng.choice(moves))
    return game


@pytest.mark.slow
def test_random_games_terminate() -> None:
    rng = random.Random(42)
    for i in range(80):
        game = _random_game(rng)
        assert game.result is not Result.ONGOING or len(game.moves) >= 400
        if game.result is not Result.ONGOING:
            assert game.outcome is not None


def test_short_random_games_stay_legal() -> None:
    rng = random.Random(7)
    for _ in range(12):
        game = Game()
        for _ply in range(30):
            if game.result is not Result.ONGOING:
                break
            moves = game.legal()
            assert moves
            game.play(rng.choice(moves))
            assert game.position.counts[0] + game.position.counts[1] <= 16


@pytest.mark.slow
def test_ai_vs_ai_completes() -> None:
    limits = Limits(move_time_ms=30, max_depth=3)
    game = Game(GameConfig(mode="ai_ai", human_first=True))
    while game.result is Result.ONGOING and len(game.moves) < 200:
        move = best_move(game.position, limits, game.hashes)
        assert any(m.src == move.src and m.dst == move.dst for m in game.legal())
        game.play(move)
    assert game.result is not Result.ONGOING or len(game.moves) >= 200
