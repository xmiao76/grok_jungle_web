from __future__ import annotations

from jungle.engine.types import FILES, NSQUARES, RANKS, Side, Square, Terrain

# Gold den d1, traps c1 d2 e1. Scarlet den d9, traps c9 d8 e9.
# Rivers: b4-c6 and e4-f6.

_WATER = {
    Square.parse(n).index()
    for n in ("b4", "c4", "b5", "c5", "b6", "c6", "e4", "f4", "e5", "f5", "e6", "f6")
}
_TRAP_GOLD = {Square.parse(n).index() for n in ("c1", "d2", "e1")}
_TRAP_SCARLET = {Square.parse(n).index() for n in ("c9", "d8", "e9")}
DEN_GOLD = Square.parse("d1").index()
DEN_SCARLET = Square.parse("d9").index()


def _build_terrain() -> tuple[Terrain, ...]:
    cells = [Terrain.LAND] * NSQUARES
    for i in _WATER:
        cells[i] = Terrain.WATER
    for i in _TRAP_GOLD:
        cells[i] = Terrain.TRAP_GOLD
    for i in _TRAP_SCARLET:
        cells[i] = Terrain.TRAP_SCARLET
    cells[DEN_GOLD] = Terrain.DEN_GOLD
    cells[DEN_SCARLET] = Terrain.DEN_SCARLET
    return tuple(cells)


TERRAIN: tuple[Terrain, ...] = _build_terrain()


def terrain_at(sq: Square | int) -> Terrain:
    idx = sq.index() if isinstance(sq, Square) else sq
    return TERRAIN[idx]


def is_water(sq: Square | int) -> bool:
    return terrain_at(sq) is Terrain.WATER


def is_den(sq: Square | int, side: Side | None = None) -> bool:
    t = terrain_at(sq)
    if side is Side.GOLD:
        return t is Terrain.DEN_GOLD
    if side is Side.SCARLET:
        return t is Terrain.DEN_SCARLET
    return t in (Terrain.DEN_GOLD, Terrain.DEN_SCARLET)


def is_trap(sq: Square | int, owner: Side | None = None) -> bool:
    t = terrain_at(sq)
    if owner is Side.GOLD:
        return t is Terrain.TRAP_GOLD
    if owner is Side.SCARLET:
        return t is Terrain.TRAP_SCARLET
    return t in (Terrain.TRAP_GOLD, Terrain.TRAP_SCARLET)


def own_den_index(side: Side) -> int:
    return DEN_GOLD if side is Side.GOLD else DEN_SCARLET


def enemy_den_index(side: Side) -> int:
    return DEN_SCARLET if side is Side.GOLD else DEN_GOLD


def trap_owner(sq: Square | int) -> Side | None:
    t = terrain_at(sq)
    if t is Terrain.TRAP_GOLD:
        return Side.GOLD
    if t is Terrain.TRAP_SCARLET:
        return Side.SCARLET
    return None


# Vertical jumps: files b,c,e,f (1,2,4,5), rank 3 <-> rank 7 (indices 2 <-> 6).
# Horizontal jumps: ranks 4,5,6 (3,4,5), a<->d and d<->g.

def _idx(file: int, rank: int) -> int:
    return rank * FILES + file


def _waters_between(a: int, b: int) -> tuple[int, ...]:
    fa, ra = a % FILES, a // FILES
    fb, rb = b % FILES, b // FILES
    squares: list[int] = []
    if fa == fb:
        step = 1 if rb > ra else -1
        for r in range(ra + step, rb, step):
            squares.append(_idx(fa, r))
    elif ra == rb:
        step = 1 if fb > fa else -1
        for f in range(fa + step, fb, step):
            squares.append(_idx(f, ra))
    else:
        raise ValueError("jump is not orthogonal")
    return tuple(squares)


def _pair(src: int, dst: int) -> tuple[int, int, tuple[int, ...]]:
    return src, dst, _waters_between(src, dst)


VERTICAL_JUMPS: tuple[tuple[int, int, tuple[int, ...]], ...] = tuple(
    jump
    for file in (1, 2, 4, 5)
    for jump in (
        _pair(_idx(file, 2), _idx(file, 6)),
        _pair(_idx(file, 6), _idx(file, 2)),
    )
)

HORIZONTAL_JUMPS: tuple[tuple[int, int, tuple[int, ...]], ...] = tuple(
    jump
    for rank in (3, 4, 5)
    for a, b in ((0, 3), (3, 6))
    for jump in (_pair(_idx(a, rank), _idx(b, rank)), _pair(_idx(b, rank), _idx(a, rank)))
)

# 2-block = horizontal (2 water). 3-block = vertical (3 water).
TWO_BLOCK_JUMPS = HORIZONTAL_JUMPS
THREE_BLOCK_JUMPS = VERTICAL_JUMPS
assert all(len(w) == 3 for _, _, w in VERTICAL_JUMPS)
assert all(len(w) == 2 for _, _, w in HORIZONTAL_JUMPS)
assert RANKS == 9 and FILES == 7
