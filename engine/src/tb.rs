//! Two-piece (1 vs 1) distance-to-mate. Generated once, then probed in search.

use crate::board::*;
use crate::movegen::{gen_moves, is_den_entry};
use crate::pos::Pos;
use crate::types::*;
use std::sync::OnceLock;

const N: usize = 8 * 63 * 8 * 63 * 2;
const UNK: i8 = 127;

static TABLE: OnceLock<Vec<i8>> = OnceLock::new();
const EMBEDDED: &[u8] = include_bytes!("tb2.bin");

fn idx(gt: u8, gsq: u8, st: u8, ssq: u8, stm: u8) -> usize {
    (((((gt - 1) as usize) * 63 + gsq as usize) * 8 + (st - 1) as usize) * 63 + ssq as usize) * 2
        + stm as usize
}

fn two_alive(pos: &Pos) -> Option<(u8, u8, u8, u8)> {
    if pos.counts[0] != 1 || pos.counts[1] != 1 {
        return None;
    }
    let mut g = None;
    let mut s = None;
    for p in &pos.pieces {
        if !p.alive {
            continue;
        }
        if p.color == GOLD {
            g = Some((p.ptype, p.sq));
        } else {
            s = Some((p.ptype, p.sq));
        }
    }
    Some((g?.0, g?.1, s?.0, s?.1))
}

fn child_key(pos: &Pos) -> Option<(u8, u8, u8, u8, u8)> {
    let (gt, gsq, st, ssq) = two_alive(pos)?;
    Some((gt, gsq, st, ssq, pos.to_move))
}

fn build() -> Vec<i8> {
    let mut t = vec![UNK; N];
    // Immediate terminals and one-ply wins.
    for gt in RAT..=ELEPHANT {
        for gsq in 0..NSQUARES as u8 {
            if gsq == DEN_GOLD {
                continue;
            }
            for st in RAT..=ELEPHANT {
                for ssq in 0..NSQUARES as u8 {
                    if ssq == DEN_SCARLET || ssq == gsq {
                        continue;
                    }
                    for stm in [GOLD, SCARLET] {
                        let i = idx(gt, gsq, st, ssq, stm);
                        if gsq == DEN_SCARLET {
                            t[i] = if stm == GOLD { 1 } else { -1 };
                            continue;
                        }
                        if ssq == DEN_GOLD {
                            t[i] = if stm == SCARLET { 1 } else { -1 };
                            continue;
                        }
                        let mut pos = Pos::empty(stm);
                        pos.place(gsq, GOLD, gt);
                        pos.place(ssq, SCARLET, st);
                        let mut list = MoveList::new();
                        gen_moves(&pos, &mut list, false);
                        if list.len == 0 {
                            t[i] = -1;
                            continue;
                        }
                        let mut win1 = false;
                        for k in 0..list.len {
                            let mv = list.moves[k];
                            if is_den_entry(&pos, mv) || pos.board[mv.to as usize] >= 0 {
                                win1 = true;
                                break;
                            }
                        }
                        if win1 {
                            t[i] = 1;
                        }
                    }
                }
            }
        }
    }

    let mut changed = true;
    let mut guard = 0u32;
    while changed && guard < 80 {
        changed = false;
        guard += 1;
        for gt in RAT..=ELEPHANT {
            for gsq in 0..NSQUARES as u8 {
                if gsq == DEN_GOLD {
                    continue;
                }
                for st in RAT..=ELEPHANT {
                    for ssq in 0..NSQUARES as u8 {
                        if ssq == DEN_SCARLET || ssq == gsq {
                            continue;
                        }
                        for stm in [GOLD, SCARLET] {
                            let i = idx(gt, gsq, st, ssq, stm);
                            if t[i] != UNK {
                                continue;
                            }
                            if gsq == DEN_SCARLET || ssq == DEN_GOLD {
                                continue;
                            }
                            let mut pos = Pos::empty(stm);
                            pos.place(gsq, GOLD, gt);
                            pos.place(ssq, SCARLET, st);
                            let mut list = MoveList::new();
                            gen_moves(&pos, &mut list, false);
                            if list.len == 0 {
                                t[i] = -1;
                                changed = true;
                                continue;
                            }
                            let mut best_win: Option<i8> = None;
                            let mut worst_loss: Option<i8> = None;
                            let mut saw_draw = false;
                            let mut saw_unk = false;
                            for k in 0..list.len {
                                let mv = list.moves[k];
                                if is_den_entry(&pos, mv) || pos.board[mv.to as usize] >= 0 {
                                    best_win = Some(1);
                                    continue;
                                }
                                let u = pos.make(mv);
                                let child = match child_key(&pos) {
                                    Some(key) => t[idx(key.0, key.1, key.2, key.3, key.4)],
                                    None => {
                                        pos.unmake(mv, u);
                                        best_win = Some(1);
                                        continue;
                                    }
                                };
                                pos.unmake(mv, u);
                                if child == UNK {
                                    saw_unk = true;
                                    continue;
                                }
                                if child == 0 {
                                    saw_draw = true;
                                    continue;
                                }
                                if child < 0 {
                                    // opponent loses => we win
                                    let w = 1 + (-child);
                                    best_win = Some(best_win.map_or(w, |b| b.min(w)));
                                } else {
                                    let l = -(1 + child);
                                    worst_loss = Some(worst_loss.map_or(l, |b| b.min(l)));
                                    // more negative = faster loss; we want the slowest loss (closest to 0)
                                    // wait: loss of -3 is better than -1 for the losing side (last longer).
                                    // child > 0 is opponent win in `child` plies.
                                    // our score = -(1+child), e.g. child=1 => -2, child=5 => -6.
                                    // Slowest loss is the one closest to 0 (e.g. -6 is slower than -2).
                                    // So we want min (most negative)? No: -6 < -2, slower mate is more negative.
                                    // We want to delay mate: pick the most negative?
                                    // Mate in 5 plies for them = we lose in 6 = -6.
                                    // Mate in 1 for them = we lose in 2 = -2.
                                    // We prefer -6 over -2, i.e. min (more negative).
                                }
                            }
                            if let Some(w) = best_win {
                                t[i] = w;
                                changed = true;
                            } else if !saw_unk && !saw_draw {
                                if let Some(l) = worst_loss {
                                    t[i] = l;
                                    changed = true;
                                }
                            } else if !saw_unk && saw_draw && worst_loss.is_some() {
                                t[i] = 0;
                                changed = true;
                            } else if !saw_unk && saw_draw && worst_loss.is_none() {
                                t[i] = 0;
                                changed = true;
                            }
                        }
                    }
                }
            }
        }
    }
    for v in &mut t {
        if *v == UNK {
            *v = 0;
        }
    }
    t
}

fn table() -> &'static [i8] {
    TABLE.get_or_init(|| {
        if EMBEDDED.len() == N {
            EMBEDDED.iter().map(|&b| b as i8).collect()
        } else {
            build()
        }
    })
}

pub fn warm() {
    let _ = table();
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(not(target_arch = "wasm32"))]
pub fn write_bin(path: &str) -> std::io::Result<()> {
    let t = build();
    let bytes: Vec<u8> = t.iter().map(|&v| v as u8).collect();
    std::fs::write(path, bytes)
}

/// Distance-to-mate from the side to move. `None` if not a 1-vs-1 ending.
/// Positive = stm wins in that many plies, negative = stm loses, 0 = draw.
pub fn probe(pos: &Pos) -> Option<i32> {
    let (gt, gsq, st, ssq) = two_alive(pos)?;
    if gsq == ssq || gsq >= 63 || ssq >= 63 {
        return None;
    }
    if !(RAT..=ELEPHANT).contains(&gt) || !(RAT..=ELEPHANT).contains(&st) {
        return None;
    }
    let i = idx(gt, gsq, st, ssq, pos.to_move);
    let t = table();
    t.get(i).copied().map(i32::from)
}

pub fn score(pos: &Pos, ply: i32) -> Option<i32> {
    let dtm = probe(pos)?;
    if dtm > 0 {
        Some(MATE - ply - dtm)
    } else if dtm < 0 {
        Some(-MATE + ply + dtm.abs())
    } else {
        Some(0)
    }
}

/// Best move in a 1-vs-1 ending, if the table knows one.
pub fn best_move(pos: &Pos) -> Option<Move> {
    let _ = two_alive(pos)?;
    let mut list = MoveList::new();
    gen_moves(pos, &mut list, false);
    if list.len == 0 {
        return None;
    }
    let mut best = list.moves[0];
    let mut best_sc = i32::MIN;
    let mut tmp = pos.clone();
    for i in 0..list.len {
        let mv = list.moves[i];
        if is_den_entry(pos, mv) || pos.board[mv.to as usize] >= 0 {
            return Some(mv);
        }
        let u = tmp.make(mv);
        let sc = match score(&tmp, 1) {
            Some(s) => -s,
            None => 0,
        };
        tmp.unmake(mv, u);
        if sc > best_sc {
            best_sc = sc;
            best = mv;
        }
    }
    Some(best)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "builds the full 2-piece table (~seconds in release)"]
    fn den_in_one_is_mate_in_one() {
        let pos = Pos::parse_fen("c6/3D3/7/7/7/7/7/7/7 g").unwrap();
        let dtm = probe(&pos).expect("2-piece");
        assert!(dtm > 0, "gold should be winning, got {dtm}");
        let mv = best_move(&pos).unwrap();
        assert_eq!(mv.uci(), "d8d9");
    }
}
