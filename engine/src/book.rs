//! Tiny opening book. Keys are start-style FENs; values are weighted UCI moves.

use crate::pos::Pos;
use crate::types::Move;

struct Entry {
    fen: &'static str,
    moves: &'static [(&'static str, u32)],
}

// Weighted lines from self-play of this engine. Rat advance, bank development,
// and lion/tiger steps. Easy ignores the book (noise at the root instead).
const BOOK: &[Entry] = &[
    Entry {
        fen: "l5t/1d3c1/r1p1w1e/7/7/7/E1W1P1R/1C3D1/T5L g",
        moves: &[
            ("g3g4", 40),
            ("a1a2", 22),
            ("g1f1", 14),
            ("a3a4", 12),
            ("g1g2", 8),
            ("e3e4", 4),
        ],
    },
    // AI-first: Scarlet opens from the start array.
    Entry {
        fen: "l5t/1d3c1/r1p1w1e/7/7/7/E1W1P1R/1C3D1/T5L s",
        moves: &[
            ("a7a6", 40),
            ("g9g8", 22),
            ("a9b9", 14),
            ("g7g6", 12),
            ("a9a8", 8),
            ("c7c6", 4),
        ],
    },
    Entry {
        fen: "l5t/1d3c1/r1p1w1e/7/7/7/E1W1P1R/1C3D1/1T4L s",
        moves: &[
            ("a9a8", 30),
            ("a7a6", 28),
            ("g9f9", 20),
            ("g7g6", 14),
            ("g9g8", 8),
        ],
    },
    Entry {
        fen: "l5t/1d3c1/r1p1w1e/7/6R/7/E1W1P2/1C3D1/T5L s",
        moves: &[
            ("a7a6", 34),
            ("a9a8", 28),
            ("g9g8", 18),
            ("g7g6", 12),
            ("c7c6", 8),
        ],
    },
];

/// Pick a book move from `pos.hash` and `seed`. The same pair always
/// returns the same move, including across component instances.
pub fn probe(pos: &Pos, seed: u32) -> Option<Move> {
    let fen = pos.fen();
    let entry = BOOK.iter().find(|e| e.fen == fen)?;
    let total: u32 = entry.moves.iter().map(|(_, w)| *w).sum();
    if total == 0 {
        return None;
    }
    let mut r = pos
        .hash
        .wrapping_mul(0x9E37_79B9_7F4A_7C15)
        .wrapping_add(u64::from(seed) | 1)
        % u64::from(total);
    for &(uci, w) in entry.moves {
        if r < u64::from(w) {
            return Move::parse(uci);
        }
        r -= u64::from(w);
    }
    Move::parse(entry.moves[0].0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_is_in_book() {
        let pos = Pos::start();
        let mv = probe(&pos, 1).expect("start book");
        assert!(["g3g4", "a1a2", "g1f1", "a3a4", "g1g2", "e3e4"].contains(&mv.uci().as_str()));
    }

    #[test]
    fn scarlet_start_is_in_book() {
        let pos = Pos::parse_fen("l5t/1d3c1/r1p1w1e/7/7/7/E1W1P1R/1C3D1/T5L s").unwrap();
        let mv = probe(&pos, 1).expect("scarlet start book");
        assert!(["a7a6", "g9g8", "a9b9", "g7g6", "a9a8", "c7c6"].contains(&mv.uci().as_str()));
    }
}
