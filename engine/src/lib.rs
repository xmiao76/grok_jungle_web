pub mod board;
pub mod book;
pub mod eval;
pub mod movegen;
pub mod pos;
pub mod search;
pub mod session;
pub mod tb;
pub mod types;

use crate::movegen::perft as rust_perft;
use crate::pos::Pos;
use crate::search::Limits;

pub fn perft_fen(fen: &str, depth: u32) -> Result<u64, String> {
    let mut pos = Pos::parse_fen(fen)?;
    Ok(rust_perft(&mut pos, depth))
}

#[allow(clippy::too_many_arguments)]
pub fn search_fen(
    fen: &str,
    history_fens: &[String],
    move_time_ms: u64,
    max_depth: i32,
    hard_cap_ms: u64,
    noise: f32,
    node_limit: u64,
    use_book: bool,
    use_tb: bool,
) -> Result<(String, u64, i32), String> {
    let mut pos = Pos::parse_fen(fen)?;
    let mut history = Vec::with_capacity(history_fens.len().max(1));
    if history_fens.is_empty() {
        history.push(pos.hash);
    } else {
        for hfen in history_fens {
            history.push(Pos::parse_fen(hfen)?.hash);
        }
    }
    let limits = Limits {
        move_time_ms,
        max_depth,
        hard_cap_ms,
        noise,
        node_limit,
        use_book,
        use_tb,
    };
    let rec = crate::search::best_move(&mut pos, limits, &history);
    Ok((rec.mv.uci(), rec.nodes, rec.depth))
}

#[cfg(feature = "wasm")]
pub mod wasm;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::movegen::{gen_moves, perft};
    use crate::search::Limits;

    #[test]
    fn start_fen_roundtrip() {
        let pos = Pos::start();
        assert_eq!(pos.fen(), "l5t/1d3c1/r1p1w1e/7/7/7/E1W1P1R/1C3D1/T5L g");
        let again = Pos::parse_fen(&pos.fen()).unwrap();
        assert_eq!(again.fen(), pos.fen());
        assert_eq!(again.counts, [8, 8]);
    }

    #[test]
    fn perft_start() {
        let mut pos = Pos::start();
        assert_eq!(perft(&mut pos, 1), 24);
        assert_eq!(perft(&mut pos, 2), 24 * 24);
        assert_eq!(perft(&mut pos, 3), 12_240);
        assert_eq!(perft(&mut pos, 4), 260_099);
        assert_eq!(perft(&mut pos, 5), 5_111_620);
    }

    #[test]
    fn make_unmake_restores() {
        let mut pos = Pos::start();
        let hash = pos.hash;
        let fen = pos.fen();
        let mut list = crate::types::MoveList::new();
        gen_moves(&pos, &mut list, false);
        for i in 0..list.len {
            let mv = list.moves[i];
            let u = pos.make(mv);
            assert_ne!(pos.fen(), fen);
            pos.unmake(mv, u);
            assert_eq!(pos.hash, hash);
            assert_eq!(pos.fen(), fen);
            assert_eq!(pos.counts, [8, 8]);
        }
    }

    #[test]
    fn lion_jumps_tiger_does_not_vertical() {
        let pos = Pos::parse_fen("7/7/7/7/7/7/1L5/7/7 g").unwrap();
        let uci = crate::movegen::legal_uci(&pos);
        assert!(uci.iter().any(|m| m == "b3b7j"), "{uci:?}");
        let tiger = Pos::parse_fen("7/7/7/7/7/7/1T5/7/7 g").unwrap();
        let tuci = crate::movegen::legal_uci(&tiger);
        assert!(!tuci.iter().any(|m| m == "b3b7j"), "{tuci:?}");
    }

    #[test]
    fn rat_takes_elephant() {
        let pos = Pos::parse_fen("7/7/7/7/7/7/7/e6/R6 g").unwrap();
        let uci = crate::movegen::legal_uci(&pos);
        assert!(uci.iter().any(|m| m == "a1a2"), "{uci:?}");
    }

    #[test]
    fn elephant_not_rat() {
        let pos = Pos::parse_fen("7/7/7/7/7/7/7/r6/E6 g").unwrap();
        let uci = crate::movegen::legal_uci(&pos);
        assert!(!uci.iter().any(|m| m == "a1a2"), "{uci:?}");
    }

    #[test]
    fn den_in_one() {
        let mut pos = Pos::parse_fen("c6/3D3/7/7/7/7/7/7/7 g").unwrap();
        let h = pos.hash;
        let rec = crate::search::best_move(&mut pos, Limits::timed(200, 6), &[h]);
        assert_eq!(rec.mv.uci(), "d8d9");
    }

    #[test]
    fn short_searches_do_not_panic() {
        let fens = [
            "l5t/1d3c1/r1p1w1e/7/7/7/E1W1P1R/1C3D1/T5L g",
            "c6/3D3/7/7/7/7/7/7/7 g",
            "7/7/7/7/w6/T6/7/7/7 g",
            "l5t/1d3c1/r1p1w1e/7/7/7/E1W1P1R/1C3D1/1T4L s",
        ];
        for fen in fens {
            let mut pos = Pos::parse_fen(fen).unwrap();
            let h = pos.hash;
            let _ = crate::search::best_move(&mut pos, Limits::timed(8, 20), &[h]);
            assert_eq!(pos.fen(), fen);
        }
    }
}
