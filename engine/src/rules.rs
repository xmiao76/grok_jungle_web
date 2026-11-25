//! Terminal order for desktop-jungle-v2.
//!
//! `history` must include the current position hash. A position is the 63
//! squares plus the side to move (the zobrist key).

use crate::movegen::gen_moves;
use crate::pos::Pos;
use crate::types::*;
use crate::wire::{BLUE, RED};

pub const HALFMOVE_DRAW: u8 = 100;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Verdict {
    pub winner: Option<u8>,
    pub reason: &'static str,
}

impl Verdict {
    pub fn playing() -> Self {
        Self {
            winner: None,
            reason: "",
        }
    }

    pub fn is_playing(self) -> bool {
        self.reason.is_empty()
    }
}

fn won(winner: u8, reason: &'static str) -> Verdict {
    Verdict {
        winner: Some(winner),
        reason,
    }
}

fn drawn(reason: &'static str) -> Verdict {
    Verdict {
        winner: None,
        reason,
    }
}

/// Den, elimination, immobilization, repetition, then the 100-ply draw.
pub fn adjudicate(pos: &Pos, history: &[u64]) -> Verdict {
    if let Some(id) = pos.board.get(crate::board::DEN_SCARLET as usize).copied() {
        if id >= 0 {
            let p = pos.pieces[id as usize];
            if p.alive && p.color == RED {
                return won(RED, "den");
            }
        }
    }
    if let Some(id) = pos.board.get(crate::board::DEN_GOLD as usize).copied() {
        if id >= 0 {
            let p = pos.pieces[id as usize];
            if p.alive && p.color == BLUE {
                return won(BLUE, "den");
            }
        }
    }
    if pos.counts[RED as usize] == 0 && pos.counts[BLUE as usize] > 0 {
        return won(BLUE, "elimination");
    }
    if pos.counts[BLUE as usize] == 0 && pos.counts[RED as usize] > 0 {
        return won(RED, "elimination");
    }
    let mut list = MoveList::new();
    gen_moves(pos, &mut list, false);
    if list.len == 0 {
        let winner = if pos.to_move == RED { BLUE } else { RED };
        return won(winner, "immobilization");
    }
    let reps = history.iter().filter(|&&h| h == pos.hash).count();
    if reps >= 3 {
        return drawn("repetition");
    }
    if pos.halfmove >= HALFMOVE_DRAW {
        return drawn("no-capture");
    }
    Verdict::playing()
}

/// Apply one wire move. The history vector gains the resulting hash.
pub fn commit_wire(pos: &mut Pos, history: &mut Vec<u64>, src: u8, dst: u8) -> bool {
    let from = crate::wire::wire_to_engine(src);
    let to = crate::wire::wire_to_engine(dst);
    let mut list = MoveList::new();
    gen_moves(pos, &mut list, false);
    let Some(mv) = (0..list.len).map(|i| list.moves[i]).find(|m| m.from == from && m.to == to)
    else {
        return false;
    };
    pos.make(mv);
    history.push(pos.hash);
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wire::{pos_from_wire, WirePiece, BLUE, RED};

    fn place(pairs: &[(u8, u8, u8)], quiet: u8) -> (Pos, Vec<u64>) {
        let mut cells = [None; 63];
        for &(wire, color, kind) in pairs {
            cells[wire as usize] = Some(WirePiece { color, kind });
        }
        let pos = pos_from_wire(&cells, RED, quiet);
        let history = vec![pos.hash];
        (pos, history)
    }

    fn play(pairs: &[(u8, u8, u8)], quiet: u8, moves: &[(u8, u8)]) -> (Pos, Vec<u64>) {
        let (mut pos, mut history) = place(pairs, quiet);
        for &(src, dst) in moves {
            assert!(commit_wire(&mut pos, &mut history, src, dst), "{src}->{dst}");
        }
        (pos, history)
    }

    #[test]
    fn adjudication_table() {
        let (pos, h) = play(&[(2, RED, CAT), (0, BLUE, RAT)], 99, &[(2, 3)]);
        let v = adjudicate(&pos, &h);
        assert_eq!(pos.halfmove, 100);
        assert_eq!(v.reason, "den");
        assert_eq!(v.winner, Some(RED));
        assert_eq!(pos.to_move, BLUE);

        let (pos, h) = play(&[(42, RED, RAT), (43, BLUE, ELEPHANT)], 99, &[(42, 43)]);
        let v = adjudicate(&pos, &h);
        assert_eq!(pos.halfmove, 0);
        assert_eq!(v.reason, "elimination");
        assert_eq!(v.winner, Some(RED));

        let (pos, h) = play(
            &[(2, RED, DOG), (7, RED, CAT), (0, BLUE, RAT)],
            99,
            &[(2, 1)],
        );
        let v = adjudicate(&pos, &h);
        assert_eq!(pos.halfmove, 100);
        assert_eq!(v.reason, "immobilization");
        assert_eq!(v.winner, Some(RED));

        let (pos, h) = play(&[(42, RED, CAT), (6, BLUE, DOG)], 98, &[(42, 43)]);
        let v = adjudicate(&pos, &h);
        assert_eq!(pos.halfmove, 99);
        assert!(v.is_playing());

        let (pos, h) = play(&[(42, RED, CAT), (6, BLUE, DOG)], 99, &[(42, 43)]);
        let v = adjudicate(&pos, &h);
        assert_eq!(pos.halfmove, 100);
        assert_eq!(v.reason, "no-capture");

        let (pos, h) = play(
            &[(42, RED, WOLF), (43, BLUE, CAT), (6, BLUE, DOG)],
            99,
            &[(42, 43)],
        );
        let v = adjudicate(&pos, &h);
        assert_eq!(pos.halfmove, 0);
        assert!(v.is_playing());
    }

    #[test]
    fn repetition_before_no_capture() {
        let setup = &[(42, RED, CAT), (6, BLUE, DOG)];
        let seven = [(42, 43), (6, 5), (43, 42), (5, 6), (42, 43), (6, 5), (43, 42)];
        let (pos, h) = play(setup, 0, &seven);
        let v = adjudicate(&pos, &h);
        assert_eq!(pos.halfmove, 7);
        assert!(v.is_playing(), "{:?}", v);

        let mut eight = seven.to_vec();
        eight.push((5, 6));
        let (pos, h) = play(setup, 0, &eight);
        let v = adjudicate(&pos, &h);
        assert_eq!(pos.halfmove, 8);
        assert_eq!(v.reason, "repetition");
        assert_eq!(pos.to_move, RED);
        assert_eq!(h.len(), 9);

        let (pos, h) = play(setup, 92, &eight);
        let v = adjudicate(&pos, &h);
        assert_eq!(pos.halfmove, 100);
        assert_eq!(v.reason, "repetition");
    }

    #[test]
    fn rules_file_digest() {
        use sha2::{Digest, Sha256};
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../rules/desktop-jungle-v2.json"
        );
        let bytes = std::fs::read(path).expect(path);
        let digest = Sha256::digest(bytes);
        let hex: String = digest.iter().map(|b| format!("{b:02x}")).collect();
        assert_eq!(hex, crate::identity::RULES_DIGEST);
        assert_eq!(crate::identity::RULES_ID, "desktop-jungle-v2");
    }
}
