from __future__ import annotations

from dataclasses import dataclass

from jungle.engine.board import DEN_GOLD, DEN_SCARLET, is_den, is_trap, is_water, trap_owner
from jungle.engine.types import (
    FEN_TO_TYPE,
    FILES,
    NSQUARES,
    RANKS,
    Piece,
    PieceType,
    Side,
    Square,
)
from jungle.engine.zobrist import piece_key, side_key

START_PLACEMENT: tuple[tuple[str, Side, PieceType], ...] = (
    ("a1", Side.GOLD, PieceType.TIGER),
    ("g1", Side.GOLD, PieceType.LION),
    ("b2", Side.GOLD, PieceType.CAT),
    ("f2", Side.GOLD, PieceType.DOG),
    ("a3", Side.GOLD, PieceType.ELEPHANT),
    ("c3", Side.GOLD, PieceType.WOLF),
    ("e3", Side.GOLD, PieceType.LEOPARD),
    ("g3", Side.GOLD, PieceType.RAT),
    ("a9", Side.SCARLET, PieceType.LION),
    ("g9", Side.SCARLET, PieceType.TIGER),
    ("b8", Side.SCARLET, PieceType.DOG),
    ("f8", Side.SCARLET, PieceType.CAT),
    ("a7", Side.SCARLET, PieceType.RAT),
    ("c7", Side.SCARLET, PieceType.LEOPARD),
    ("e7", Side.SCARLET, PieceType.WOLF),
    ("g7", Side.SCARLET, PieceType.ELEPHANT),
)


@dataclass
class Position:
    board: list[Piece | None]
    to_move: Side
    halfmove: int
    counts: list[int]
    hash: int

    def copy(self) -> Position:
        return Position(self.board[:], self.to_move, self.halfmove, self.counts[:], self.hash)

    def piece_at(self, sq: Square | int) -> Piece | None:
        idx = sq.index() if isinstance(sq, Square) else sq
        return self.board[idx]

    def place(self, sq: Square | int, piece: Piece | None) -> None:
        idx = sq.index() if isinstance(sq, Square) else sq
        old = self.board[idx]
        if old is not None:
            self.hash ^= piece_key(old, idx)
            self.counts[old.side] -= 1
        self.board[idx] = piece
        if piece is not None:
            self.hash ^= piece_key(piece, idx)
            self.counts[piece.side] += 1

    def pieces(self, side: Side | None = None) -> list[tuple[int, Piece]]:
        out: list[tuple[int, Piece]] = []
        for i, p in enumerate(self.board):
            if p is not None and (side is None or p.side is side):
                out.append((i, p))
        return out

    def effective_rank(self, piece: Piece, square_index: int) -> int:
        owner = trap_owner(square_index)
        if owner is not None and owner is not piece.side:
            return 0
        return int(piece.ptype)


def empty_position(to_move: Side = Side.GOLD) -> Position:
    h = side_key() if to_move is Side.SCARLET else 0
    return Position([None] * NSQUARES, to_move, 0, [0, 0], h)


def start_position(to_move: Side = Side.GOLD) -> Position:
    pos = empty_position(to_move)
    for name, side, ptype in START_PLACEMENT:
        pos.place(Square.parse(name), Piece(side, ptype))
    return pos


def compute_hash(board: list[Piece | None], to_move: Side) -> int:
    h = side_key() if to_move is Side.SCARLET else 0
    for i, p in enumerate(board):
        if p is not None:
            h ^= piece_key(p, i)
    return h


def dump_fen(pos: Position) -> str:
    ranks: list[str] = []
    for rank in range(RANKS - 1, -1, -1):
        empty = 0
        chars: list[str] = []
        for file in range(FILES):
            piece = pos.board[rank * FILES + file]
            if piece is None:
                empty += 1
            else:
                if empty:
                    chars.append(str(empty))
                    empty = 0
                chars.append(piece.fen_char())
        if empty:
            chars.append(str(empty))
        ranks.append("".join(chars))
    stm = "g" if pos.to_move is Side.GOLD else "s"
    return "/".join(ranks) + f" {stm}"


def parse_fen(fen: str) -> Position:
    body, _, stm = fen.strip().partition(" ")
    to_move = Side.SCARLET if stm[:1].lower() == "s" else Side.GOLD
    rows = body.split("/")
    if len(rows) != RANKS:
        raise ValueError(f"expected {RANKS} ranks, got {len(rows)}")
    pos = empty_position(to_move)
    for rank_from_top, row in enumerate(rows):
        rank = RANKS - 1 - rank_from_top
        file = 0
        for ch in row:
            if ch.isdigit():
                file += int(ch)
                continue
            upper = ch.upper()
            if upper not in FEN_TO_TYPE:
                raise ValueError(f"bad FEN piece {ch!r}")
            if file >= FILES:
                raise ValueError("file overflow")
            side = Side.GOLD if ch.isupper() else Side.SCARLET
            pos.place(rank * FILES + file, Piece(side, FEN_TO_TYPE[upper]))
            file += 1
        if file != FILES:
            raise ValueError(f"rank {rank + 1} has {file} files")
    return pos


def can_capture(pos: Position, attacker: Piece, src: int, defender: Piece, dst: int) -> bool:
    if attacker.side is defender.side:
        return False
    if is_water(dst):
        if attacker.ptype is not PieceType.RAT or not is_water(src):
            return False
    # A swimming Rat may only capture in water (another swimming Rat).
    if attacker.ptype is PieceType.RAT and is_water(src) and not is_water(dst):
        return False
    if attacker.ptype is PieceType.RAT and defender.ptype is PieceType.ELEPHANT:
        return not is_water(src) and not is_water(dst)
    # Elephant never captures a Rat, except a Rat sitting on the Elephant's trap
    # (trap rank 0 — any defender may take it).
    if attacker.ptype is PieceType.ELEPHANT and defender.ptype is PieceType.RAT:
        return pos.effective_rank(defender, dst) == 0 and not is_water(dst)
    return pos.effective_rank(attacker, src) >= pos.effective_rank(defender, dst)


def can_land(pos: Position, piece: Piece, src: int, dst: int) -> bool:
    if is_den(dst, piece.side):
        return False
    if is_water(dst) and piece.ptype is not PieceType.RAT:
        return False
    occupant = pos.board[dst]
    if occupant is None:
        return True
    return can_capture(pos, piece, src, occupant, dst)


# Re-export helpers used by movegen/game
__all__ = [
    "Position",
    "START_PLACEMENT",
    "empty_position",
    "start_position",
    "compute_hash",
    "dump_fen",
    "parse_fen",
    "can_capture",
    "can_land",
    "DEN_GOLD",
    "DEN_SCARLET",
    "is_den",
    "is_trap",
    "is_water",
]
