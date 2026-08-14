from jungle.engine.apply import HALFMOVE_DRAW, apply_move, terminal_after
from jungle.engine.game import Game, GameConfig
from jungle.engine.position import empty_position
from jungle.engine.types import Move, Outcome, Piece, PieceType, Result, Side, Square


def test_wipe_wins() -> None:
    pos = empty_position(Side.GOLD)
    pos.place(Square.parse("a1"), Piece(Side.GOLD, PieceType.DOG))
    pos.place(Square.parse("a2"), Piece(Side.SCARLET, PieceType.CAT))
    nxt = apply_move(pos, Move.parse("a1a2"))
    result, outcome = terminal_after(nxt, Move.parse("a1a2"), [nxt.hash])
    assert result is Result.GOLD_WINS
    assert outcome is Outcome.WIPE


def test_no_moves_loses() -> None:
    # Scarlet rat boxed so it has no legal squares: gold dog adjacent blocking
    # and edges. Simpler: scarlet has a piece that cannot move into own den
    # and every other square is off-board or water or friend — use a lion
    # with no empty land. Isolated scarlet rat on a9 (corner) with gold
    # occupying a8 and b9, and a9 is not water. Rat can try a8/b9 — occupied
    # by stronger gold pieces, cannot capture. Own den is d9, not adjacent.
    pos = empty_position(Side.SCARLET)
    pos.place(Square.parse("a9"), Piece(Side.SCARLET, PieceType.CAT))
    pos.place(Square.parse("a8"), Piece(Side.GOLD, PieceType.ELEPHANT))
    pos.place(Square.parse("b9"), Piece(Side.GOLD, PieceType.ELEPHANT))
    result, outcome = terminal_after(pos, None, [pos.hash])
    assert result is Result.GOLD_WINS
    assert outcome is Outcome.NO_MOVES


def test_threefold_draw() -> None:
    game = Game()
    # Oscillate gold lion g1-g2 and scarlet lion a9-a8 twice more after returning.
    cycle = ["g1g2", "a9a8", "g2g1", "a8a9"]
    for _ in range(3):
        for u in cycle:
            if game.result is not Result.ONGOING:
                break
            game.play(Move.parse(u))
    assert game.result is Result.DRAW
    assert game.outcome is Outcome.REPETITION


def test_fifty_move_draw() -> None:
    pos = empty_position(Side.GOLD)
    pos.place(Square.parse("a1"), Piece(Side.GOLD, PieceType.CAT))
    pos.place(Square.parse("g9"), Piece(Side.SCARLET, PieceType.CAT))
    pos.halfmove = HALFMOVE_DRAW
    result, outcome = terminal_after(pos, None, [pos.hash])
    assert result is Result.DRAW
    assert outcome is Outcome.FIFTY


def test_human_first_gold_to_move() -> None:
    game = Game(GameConfig(human_first=True))
    assert game.position.to_move is Side.GOLD
    assert game.config.human_side is Side.GOLD


def test_ai_first_scarlet_to_move() -> None:
    game = Game(GameConfig(human_first=False))
    assert game.position.to_move is Side.SCARLET
    assert game.config.human_side is Side.GOLD
    assert game.config.is_ai_side(Side.SCARLET)
    assert not game.config.is_ai_side(Side.GOLD)
