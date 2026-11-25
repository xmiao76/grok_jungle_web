//! Wire squares are row-major from the top-left. Engine squares are
//! rank-major from the bottom. Red is the engine's gold side (bottom).

use crate::movegen::gen_moves;
use crate::pos::Pos;
use crate::types::*;

pub const RED: u8 = GOLD;
pub const BLUE: u8 = SCARLET;

pub fn engine_to_wire(sq: u8) -> u8 {
    let file = sq % 7;
    let rank = sq / 7;
    (8 - rank) * 7 + file
}

pub fn wire_to_engine(wire: u8) -> u8 {
    let col = wire % 7;
    let row = wire / 7;
    let rank = 8 - row;
    rank * 7 + col
}

pub fn side_wire(color: u8) -> &'static str {
    if color == RED {
        "red"
    } else {
        "blue"
    }
}

#[derive(Clone, Copy, Debug)]
pub struct WirePiece {
    pub color: u8,
    pub kind: u8,
}

pub fn pos_from_wire(cells: &[Option<WirePiece>], to_move: u8, halfmove: u8) -> Pos {
    let mut pos = Pos::empty(to_move);
    if cells.len() != NSQUARES {
        return pos;
    }
    for (wire, cell) in cells.iter().enumerate() {
        if let Some(p) = cell {
            pos.place(wire_to_engine(wire as u8), p.color, p.kind);
        }
    }
    pos.halfmove = halfmove;
    pos
}

/// Legal moves as `(wire src, wire dst, engine move)`, sorted by `(src, dst)`.
pub fn wire_legal(pos: &Pos) -> Vec<(u8, u8, Move)> {
    let mut list = MoveList::new();
    if pos.counts[pos.to_move as usize] > 0 {
        gen_moves(pos, &mut list, false);
    }
    let mut out = Vec::with_capacity(list.len);
    for i in 0..list.len {
        let mv = list.moves[i];
        out.push((engine_to_wire(mv.from), engine_to_wire(mv.to), mv));
    }
    out.sort_by_key(|(src, dst, _)| (*src, *dst));
    out
}

pub fn opening(red_first: bool) -> Pos {
    let mut pos = Pos::start();
    if !red_first {
        pos.to_move = BLUE;
        pos.hash ^= crate::pos::side_key();
    }
    pos
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dens_and_opening_squares() {
        assert_eq!(engine_to_wire(3), 59);
        assert_eq!(engine_to_wire(59), 3);
        assert_eq!(wire_to_engine(59), 3);
        let pos = Pos::start();
        let mut elephant = None;
        let mut lion = None;
        for p in &pos.pieces {
            if !p.alive || p.color != RED {
                continue;
            }
            if p.ptype == ELEPHANT {
                elephant = Some(engine_to_wire(p.sq));
            }
            if p.ptype == LION {
                lion = Some(engine_to_wire(p.sq));
            }
        }
        assert_eq!(elephant, Some(42));
        assert_eq!(lion, Some(62));
    }

    #[test]
    fn opening_handshake() {
        let red = wire_legal(&opening(true));
        assert_eq!(red.len(), 24);
        assert_eq!((red[0].0, red[0].1), (42, 35));
        assert_eq!((red[23].0, red[23].1), (62, 61));
        let blue = wire_legal(&opening(false));
        assert_eq!(blue.len(), 24);
        assert_eq!((blue[0].0, blue[0].1), (0, 1));
    }

    fn piece(color: u8, kind: u8) -> WirePiece {
        WirePiece { color, kind }
    }

    fn sparse(pairs: &[(u8, u8, u8)]) -> Pos {
        let mut cells = [None; 63];
        for &(wire, color, kind) in pairs {
            cells[wire as usize] = Some(piece(color, kind));
        }
        pos_from_wire(&cells, RED, 0)
    }

    fn has(pos: &Pos, src: u8, dst: u8) -> bool {
        wire_legal(pos).iter().any(|(s, d, _)| *s == src && *d == dst)
    }

    #[test]
    fn portable_legal_cases() {
        let cases: &[(&str, &[(u8, u8, u8)], u8, u8, bool)] = &[
            ("lion-short", &[(21, RED, LION), (0, BLUE, RAT)], 21, 24, true),
            ("lion-long", &[(15, RED, LION), (0, BLUE, RAT)], 15, 43, true),
            ("tiger-short", &[(21, RED, TIGER), (0, BLUE, RAT)], 21, 24, true),
            ("tiger-long", &[(15, RED, TIGER), (0, BLUE, RAT)], 15, 43, false),
            (
                "jump-own-rat",
                &[(15, RED, LION), (29, RED, RAT), (0, BLUE, CAT)],
                15,
                43,
                false,
            ),
            (
                "jump-enemy-rat",
                &[(21, RED, TIGER), (23, BLUE, RAT)],
                21,
                24,
                false,
            ),
            (
                "jump-capture",
                &[(21, RED, TIGER), (24, BLUE, DOG)],
                21,
                24,
                true,
            ),
            (
                "jump-stronger",
                &[(21, RED, TIGER), (24, BLUE, ELEPHANT)],
                21,
                24,
                false,
            ),
            ("rat-swims", &[(21, RED, RAT), (0, BLUE, CAT)], 21, 22, true),
            ("cat-cannot-swim", &[(21, RED, CAT), (0, BLUE, RAT)], 21, 22, false),
            (
                "water-to-land",
                &[(22, RED, RAT), (21, BLUE, ELEPHANT)],
                22,
                21,
                false,
            ),
            (
                "land-to-water",
                &[(21, RED, RAT), (22, BLUE, RAT)],
                21,
                22,
                false,
            ),
            (
                "water-capture",
                &[(22, RED, RAT), (23, BLUE, RAT)],
                22,
                23,
                true,
            ),
            (
                "rat-elephant",
                &[(42, RED, RAT), (43, BLUE, ELEPHANT)],
                42,
                43,
                true,
            ),
            (
                "elephant-rat",
                &[(42, RED, ELEPHANT), (43, BLUE, RAT)],
                42,
                43,
                false,
            ),
            ("equal-rank", &[(42, RED, DOG), (43, BLUE, DOG)], 42, 43, true),
            (
                "trapped-elephant",
                &[(51, RED, CAT), (52, BLUE, ELEPHANT)],
                51,
                52,
                true,
            ),
            (
                "trapped-rat",
                &[(51, RED, ELEPHANT), (52, BLUE, RAT)],
                51,
                52,
                true,
            ),
            (
                "trapped-attacker",
                &[(10, RED, TIGER), (11, BLUE, CAT)],
                10,
                11,
                false,
            ),
            (
                "leave-trap-empty",
                &[(10, RED, TIGER), (0, BLUE, CAT)],
                10,
                11,
                true,
            ),
            ("own-den", &[(58, RED, CAT), (0, BLUE, RAT)], 58, 59, false),
            ("enemy-den", &[(2, RED, CAT), (0, BLUE, RAT)], 2, 3, true),
            (
                "friendly-occupied",
                &[(42, RED, CAT), (43, RED, ELEPHANT), (0, BLUE, RAT)],
                42,
                43,
                false,
            ),
        ];
        for (name, pairs, src, dst, expect) in cases {
            let pos = sparse(pairs);
            assert_eq!(has(&pos, *src, *dst), *expect, "{name}");
        }
    }
}
