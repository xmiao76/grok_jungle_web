from jungle.engine.game import Game, GameConfig
from jungle.engine.types import Move, Side


def test_flip_does_not_mutate_state() -> None:
    game = Game(GameConfig(human_first=True))
    game.play(Move.parse("a1a2"))
    before = game.snapshot_key()
    to_move = game.position.to_move
    game.flip_view()
    assert game.view.flipped is True
    assert game.snapshot_key() == before
    assert game.position.to_move is to_move
    game.flip_view()
    assert game.view.flipped is False
    assert game.snapshot_key() == before


def test_flip_independent_of_first_move() -> None:
    game = Game(GameConfig(human_first=False))
    key = game.snapshot_key()
    assert game.position.to_move is Side.SCARLET
    game.flip_view()
    assert game.snapshot_key() == key
    assert game.position.to_move is Side.SCARLET
    assert game.config.human_side is Side.GOLD


def test_undo_restores_position() -> None:
    game = Game()
    start = game.snapshot_key()
    game.play(Move.parse("g3g4"))
    game.undo()
    assert game.snapshot_key() == start
    assert game.position.to_move is Side.GOLD
