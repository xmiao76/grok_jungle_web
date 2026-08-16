use crate::types::*;
use std::collections::VecDeque;
use std::sync::OnceLock;

pub const DEN_GOLD: u8 = 3; // d1
pub const DEN_SCARLET: u8 = 3 + 8 * 7; // d9

pub struct Tables {
    pub water: u64,
    pub trap_gold: u64,
    pub trap_scarlet: u64,
    pub den: [u64; 2],
    pub nei: [[i8; 4]; NSQUARES],
    pub horiz_jumps: [Vec<(u8, u64)>; NSQUARES],
    pub vert_jumps: [Vec<(u8, u64)>; NSQUARES],
    /// [side][ptype 1..=8][sq] = path-aware steps to enemy den (99 = unreachable)
    pub den_dist: [[[u8; NSQUARES]; 9]; 2],
    pub jump_banks: u64,
    pub horiz_banks: u64,
}

pub fn tables() -> &'static Tables {
    static T: OnceLock<Tables> = OnceLock::new();
    T.get_or_init(build)
}

pub fn enemy_den(side: u8) -> u8 {
    if side == GOLD {
        DEN_SCARLET
    } else {
        DEN_GOLD
    }
}

pub fn own_den(side: u8) -> u8 {
    if side == GOLD {
        DEN_GOLD
    } else {
        DEN_SCARLET
    }
}

pub fn bit(sq: u8) -> u64 {
    if sq >= 64 {
        0
    } else {
        1u64 << sq
    }
}

pub fn is_water_sq(sq: u8) -> bool {
    tables().water & bit(sq) != 0
}

pub fn trap_owner(sq: u8) -> Option<u8> {
    let t = tables();
    if t.trap_gold & bit(sq) != 0 {
        Some(GOLD)
    } else if t.trap_scarlet & bit(sq) != 0 {
        Some(SCARLET)
    } else {
        None
    }
}

fn waters_between(a: u8, b: u8) -> u64 {
    let (fa, ra) = (file_of(a), rank_of(a));
    let (fb, rb) = (file_of(b), rank_of(b));
    let mut mask = 0u64;
    if fa == fb {
        let step = if rb > ra { 1 } else { -1 };
        let mut r = ra + step;
        while r != rb {
            mask |= bit(sq(fa, r));
            r += step;
        }
    } else if ra == rb {
        let step = if fb > fa { 1 } else { -1 };
        let mut f = fa + step;
        while f != fb {
            mask |= bit(sq(f, ra));
            f += step;
        }
    }
    mask
}

fn parse_name(name: &str) -> u8 {
    parse_sq(name).expect(name)
}

fn build() -> Tables {
    let mut water = 0u64;
    for name in [
        "b4", "c4", "b5", "c5", "b6", "c6", "e4", "f4", "e5", "f5", "e6", "f6",
    ] {
        water |= bit(parse_name(name));
    }
    let mut trap_gold = 0u64;
    for name in ["c1", "d2", "e1"] {
        trap_gold |= bit(parse_name(name));
    }
    let mut trap_scarlet = 0u64;
    for name in ["c9", "d8", "e9"] {
        trap_scarlet |= bit(parse_name(name));
    }

    let mut nei = [[-1i8; 4]; NSQUARES];
    const DIRS: [(i32, i32); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];
    for s in 0..NSQUARES {
        let f = file_of(s as u8);
        let r = rank_of(s as u8);
        for (i, (df, dr)) in DIRS.iter().enumerate() {
            let nf = f + df;
            let nr = r + dr;
            if on_board(nf, nr) {
                nei[s][i] = sq(nf, nr) as i8;
            }
        }
    }

    let mut horiz_jumps: [Vec<(u8, u64)>; NSQUARES] = std::array::from_fn(|_| Vec::new());
    let mut vert_jumps: [Vec<(u8, u64)>; NSQUARES] = std::array::from_fn(|_| Vec::new());

    for file in [1, 2, 4, 5] {
        let a = sq(file, 2);
        let b = sq(file, 6);
        let mask = waters_between(a, b);
        debug_assert_eq!(mask.count_ones(), 3);
        vert_jumps[a as usize].push((b, mask));
        vert_jumps[b as usize].push((a, mask));
    }
    for rank in [3, 4, 5] {
        for (a_f, b_f) in [(0, 3), (3, 6)] {
            let a = sq(a_f, rank);
            let b = sq(b_f, rank);
            let mask = waters_between(a, b);
            debug_assert_eq!(mask.count_ones(), 2);
            horiz_jumps[a as usize].push((b, mask));
            horiz_jumps[b as usize].push((a, mask));
        }
    }

    let mut jump_banks = 0u64;
    for r in [2, 6] {
        for f in [1, 2, 4, 5] {
            jump_banks |= bit(sq(f, r));
        }
    }
    let mut horiz_banks = 0u64;
    for r in [3, 4, 5] {
        for f in [0, 3, 6] {
            horiz_banks |= bit(sq(f, r));
        }
    }

    let mut den_dist = [[[99u8; NSQUARES]; 9]; 2];
    for side in [GOLD, SCARLET] {
        let target = enemy_den(side);
        let forbidden = own_den(side);
        for ptype in RAT..=ELEPHANT {
            let mut adj: [Vec<u8>; NSQUARES] = std::array::from_fn(|_| Vec::new());
            for s in 0..NSQUARES as u8 {
                if s == forbidden {
                    continue;
                }
                for &n in &nei[s as usize] {
                    if n < 0 {
                        continue;
                    }
                    let dst = n as u8;
                    if dst == forbidden {
                        continue;
                    }
                    if (water & bit(dst)) != 0 && ptype != RAT {
                        continue;
                    }
                    adj[s as usize].push(dst);
                }
                if ptype == LION || ptype == TIGER {
                    for &(dst, _) in &horiz_jumps[s as usize] {
                        if dst != forbidden {
                            adj[s as usize].push(dst);
                        }
                    }
                }
                if ptype == LION {
                    for &(dst, _) in &vert_jumps[s as usize] {
                        if dst != forbidden {
                            adj[s as usize].push(dst);
                        }
                    }
                }
            }
            let mut dist = [99u8; NSQUARES];
            dist[target as usize] = 0;
            let mut q = VecDeque::new();
            q.push_back(target);
            while let Some(cur) = q.pop_front() {
                let d = dist[cur as usize];
                for from in 0..NSQUARES as u8 {
                    if dist[from as usize] != 99 {
                        continue;
                    }
                    if adj[from as usize].contains(&cur) {
                        dist[from as usize] = d + 1;
                        q.push_back(from);
                    }
                }
            }
            den_dist[side as usize][ptype as usize] = dist;
        }
    }

    Tables {
        water,
        trap_gold,
        trap_scarlet,
        den: [bit(DEN_GOLD), bit(DEN_SCARLET)],
        nei,
        horiz_jumps,
        vert_jumps,
        den_dist,
        jump_banks,
        horiz_banks,
    }
}
