from __future__ import annotations

from dataclasses import dataclass
from enum import Enum, IntEnum

FILES = 7
RANKS = 9
NSQUARES = FILES * RANKS


class Side(IntEnum):
    GOLD = 0
    SCARLET = 1

    def other(self) -> Side:
        return Side.SCARLET if self is Side.GOLD else Side.GOLD

    @property
    def label(self) -> str:
        return "Gold" if self is Side.GOLD else "Scarlet"


class PieceType(IntEnum):
    RAT = 1
    CAT = 2
    DOG = 3
    WOLF = 4
    LEOPARD = 5
    TIGER = 6
    LION = 7
    ELEPHANT = 8

    @property
    def fen_char(self) -> str:
        return {
            PieceType.RAT: "R",
            PieceType.CAT: "C",
            PieceType.DOG: "D",
            PieceType.WOLF: "W",
            PieceType.LEOPARD: "P",
            PieceType.TIGER: "T",
            PieceType.LION: "L",
            PieceType.ELEPHANT: "E",
        }[self]

    @property
    def noun(self) -> str:
        return {
            PieceType.RAT: "Rat",
            PieceType.CAT: "Cat",
            PieceType.DOG: "Dog",
            PieceType.WOLF: "Wolf",
            PieceType.LEOPARD: "Leopard",
            PieceType.TIGER: "Tiger",
            PieceType.LION: "Lion",
            PieceType.ELEPHANT: "Elephant",
        }[self]

    @property
    def short_name(self) -> str:
        return self.noun


FEN_TO_TYPE = {pt.fen_char: pt for pt in PieceType}


class Terrain(IntEnum):
    LAND = 0
    WATER = 1
    TRAP_GOLD = 2
    TRAP_SCARLET = 3
    DEN_GOLD = 4
    DEN_SCARLET = 5


class Result(Enum):
    ONGOING = "ongoing"
    GOLD_WINS = "gold_wins"
    SCARLET_WINS = "scarlet_wins"
    DRAW = "draw"


class Outcome(Enum):
    DEN = "den"
    ELIMINATION = "elimination"
    IMMOBILIZATION = "immobilization"
    RESIGN = "resign"
    REPETITION = "repetition"
    NO_CAPTURE = "no-capture"


@dataclass(frozen=True, slots=True)
class Square:
    file: int
    rank: int

    def __post_init__(self) -> None:
        if not (0 <= self.file < FILES and 0 <= self.rank < RANKS):
            raise ValueError(f"off-board square {self.file},{self.rank}")

    def index(self) -> int:
        return self.rank * FILES + self.file

    @staticmethod
    def from_index(idx: int) -> Square:
        return Square(idx % FILES, idx // FILES)

    @staticmethod
    def parse(name: str) -> Square:
        name = name.strip().lower()
        if len(name) < 2:
            raise ValueError(name)
        return Square(ord(name[0]) - ord("a"), int(name[1:]) - 1)

    def name(self) -> str:
        return f"{chr(ord('a') + self.file)}{self.rank + 1}"

    def step(self, df: int, dr: int) -> Square | None:
        f, r = self.file + df, self.rank + dr
        if 0 <= f < FILES and 0 <= r < RANKS:
            return Square(f, r)
        return None


@dataclass(frozen=True, slots=True)
class Piece:
    side: Side
    ptype: PieceType

    def fen_char(self) -> str:
        ch = self.ptype.fen_char
        return ch if self.side is Side.GOLD else ch.lower()


@dataclass(frozen=True, slots=True)
class Move:
    src: Square
    dst: Square
    is_jump: bool = False

    def uci(self) -> str:
        tag = "j" if self.is_jump else ""
        return f"{self.src.name()}{self.dst.name()}{tag}"

    @staticmethod
    def parse(text: str) -> Move:
        text = text.strip().lower()
        is_jump = text.endswith("j")
        if is_jump:
            text = text[:-1]
        if len(text) < 4:
            raise ValueError(text)
        # files are one letter; ranks are one digit on this 7x9 board
        return Move(Square.parse(text[:2]), Square.parse(text[2:4]), is_jump=is_jump)


ORTHO = ((1, 0), (-1, 0), (0, 1), (0, -1))
