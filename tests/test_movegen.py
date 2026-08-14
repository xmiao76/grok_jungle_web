from jungle.engine.apply import apply_move
from jungle.engine.movegen import legal_moves, perft
from jungle.engine.position import start_position
from jungle.engine.types import Move, Side, Square


def _uci_set(pos=None):
    pos = pos or start_position()
    return {m.uci() for m in legal_moves(pos)}


def test_startpos_gold_move_count() -> None:
    moves = legal_moves(start_position())
    # Wolf/leopard cannot step onto the river at c4/e4.
    assert len(moves) == 24


def test_startpos_scarlet_move_count() -> None:
    pos = start_position(Side.SCARLET)
    assert len(legal_moves(pos)) == 24


def test_tiger_opening_steps() -> None:
    uci = _uci_set()
    assert "a1a2" in uci
    assert "a1b1" in uci
    assert "a1a0" not in uci
    assert "a1a1" not in uci


def test_no_diagonal() -> None:
    uci = _uci_set()
    assert "a1b2" not in uci
    assert "b2a1" not in uci
    assert "b2c3" not in uci


def test_cannot_enter_own_den() -> None:
    # Gold lion on d2 (trap) must not step onto d1.
    from jungle.engine.position import empty_position
    from jungle.engine.types import Piece, PieceType

    pos = empty_position(Side.GOLD)
    pos.place(Square.parse("d2"), Piece(Side.GOLD, PieceType.LION))
    uci = {m.uci() for m in legal_moves(pos)}
    assert "d2d1" not in uci
    assert "d2d3" in uci
    assert "d2c2" in uci
    assert "d2e2" in uci


def test_can_enter_enemy_den() -> None:
    from jungle.engine.position import empty_position
    from jungle.engine.types import Piece, PieceType

    pos = empty_position(Side.GOLD)
    pos.place(Square.parse("d8"), Piece(Side.GOLD, PieceType.CAT))
    uci = {m.uci() for m in legal_moves(pos)}
    assert "d8d9" in uci


def test_cannot_land_on_friend() -> None:
    from jungle.engine.position import empty_position
    from jungle.engine.types import Piece, PieceType

    pos = empty_position(Side.GOLD)
    pos.place(Square.parse("a1"), Piece(Side.GOLD, PieceType.TIGER))
    pos.place(Square.parse("a2"), Piece(Side.GOLD, PieceType.CAT))
    uci = {m.uci() for m in legal_moves(pos)}
    assert "a1a2" not in uci


def test_perft_depth_2_matches_apply() -> None:
    n = perft(start_position(), 2)
    # 26 gold replies; each scarlet position also has 26 by symmetry on the opening.
    assert n == 24 * 24


def test_apply_unmake_via_copy() -> None:
    pos = start_position()
    fen = pos.hash
    move = Move.parse("a1a2")
    nxt = apply_move(pos, move)
    assert nxt.piece_at(Square.parse("a1")) is None
    assert nxt.piece_at(Square.parse("a2")) is not None
    assert nxt.to_move is Side.SCARLET
    assert pos.hash == fen
    assert pos.piece_at(Square.parse("a1")) is not None
