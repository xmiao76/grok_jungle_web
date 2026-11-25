use crate::board::*;
use crate::pos::Pos;
use crate::types::*;

pub fn evaluate(pos: &Pos) -> i32 {
    let remaining = pos.remaining();
    let end_w = if remaining >= 12 {
        0
    } else if remaining <= 6 {
        55
    } else {
        (12 - remaining) * 55 / 6
    };
    let mut score = pos.material_pst + pos.approach + pos.approach * end_w / 100;

    score += mobility(pos, GOLD) * 5;
    score -= mobility(pos, SCARLET) * 5;
    score += rat_elephant(pos);
    score += race_term(pos);
    score += den_threat(pos, GOLD) * if pos.to_move == GOLD { 4 } else { 1 };
    score -= den_threat(pos, SCARLET) * if pos.to_move == SCARLET { 4 } else { 1 };
    score += den_guard(pos, GOLD);
    score -= den_guard(pos, SCARLET);
    score += jump_block_penalty(pos);
    score += hanging_term(pos);
    score -= approach_danger(pos, GOLD);
    score += approach_danger(pos, SCARLET);

    if pos.to_move == GOLD {
        score
    } else {
        -score
    }
}

fn mobility(pos: &Pos, side: u8) -> i32 {
    let t = tables();
    let mut n = 0i32;
    for id in 0..MAX_PIECES {
        let p = pos.pieces[id];
        if !p.alive || p.color != side || (p.sq as usize) >= NSQUARES {
            continue;
        }
        for &nb in &t.nei[p.sq as usize] {
            if nb < 0 {
                continue;
            }
            if pos.can_land(p, p.sq, nb as u8) {
                n += 1;
            }
        }
        if p.ptype == LION || p.ptype == TIGER {
            let horiz = t.horiz_jumps[p.sq as usize].iter();
            let verts = if p.ptype == LION {
                t.vert_jumps[p.sq as usize].iter()
            } else {
                [].iter()
            };
            for &(dst, waters) in horiz.chain(verts) {
                if pos.rats & waters == 0 && pos.can_land(p, p.sq, dst) {
                    n += 1;
                }
            }
        }
    }
    n
}

fn rat_elephant(pos: &Pos) -> i32 {
    let t = tables();
    let mut s = 0i32;
    for id in 0..MAX_PIECES {
        let p = pos.pieces[id];
        if !p.alive || p.ptype != RAT || (p.sq as usize) >= NSQUARES || is_water_sq(p.sq) {
            continue;
        }
        for &nb in &t.nei[p.sq as usize] {
            if nb < 0 {
                continue;
            }
            if let Some(o) = pos.piece_at(nb as u8) {
                if o.color != p.color && o.ptype == ELEPHANT {
                    s += if p.color == GOLD { 90 } else { -90 };
                }
            }
        }
    }
    s
}

fn race_term(pos: &Pos) -> i32 {
    let gd = min_den_dist(pos, GOLD);
    let sd = min_den_dist(pos, SCARLET);
    if gd >= 90 && sd >= 90 {
        return 0;
    }
    let g = gd * 2 + i32::from(pos.to_move != GOLD);
    let s = sd * 2 + i32::from(pos.to_move != SCARLET);
    // Smaller is closer. Gold closer => positive.
    (s - g) * 28
}

fn min_den_dist(pos: &Pos, side: u8) -> i32 {
    let t = tables();
    let mut best = 99i32;
    for id in 0..MAX_PIECES {
        let p = pos.pieces[id];
        if !p.alive || p.color != side || (p.sq as usize) >= NSQUARES {
            continue;
        }
        let pt = p.ptype as usize;
        if pt >= 9 {
            continue;
        }
        let d = i32::from(t.den_dist[side as usize][pt][p.sq as usize]);
        if d < best {
            best = d;
        }
    }
    best
}

fn den_threat(pos: &Pos, side: u8) -> i32 {
    let den = enemy_den(side);
    if pos.board[den as usize] >= 0 {
        return 0;
    }
    let t = tables();
    for &nb in &t.nei[den as usize] {
        if nb < 0 {
            continue;
        }
        if let Some(p) = pos.piece_at(nb as u8) {
            if p.color == side {
                return 220;
            }
        }
    }
    0
}

fn den_guard(pos: &Pos, side: u8) -> i32 {
    let den = own_den(side);
    let t = tables();
    let mut g = 0i32;
    for &nb in &t.nei[den as usize] {
        if nb < 0 {
            continue;
        }
        if let Some(p) = pos.piece_at(nb as u8) {
            if p.color == side {
                g += 18;
            }
        }
    }
    g
}

fn jump_block_penalty(pos: &Pos) -> i32 {
    let t = tables();
    let mut s = 0i32;
    for id in 0..MAX_PIECES {
        let p = pos.pieces[id];
        if !p.alive || (p.ptype != LION && p.ptype != TIGER) {
            continue;
        }
        let mut blocked = 0i32;
        let mut total = 0i32;
        let horiz = t.horiz_jumps[p.sq as usize].iter();
        let verts = if p.ptype == LION {
            t.vert_jumps[p.sq as usize].iter()
        } else {
            [].iter()
        };
        for &(_, waters) in horiz.chain(verts) {
            total += 1;
            if pos.rats & waters != 0 {
                blocked += 1;
            }
        }
        if total > 0 && blocked > 0 {
            let pen = 16 * blocked;
            s += if p.color == GOLD { -pen } else { pen };
        }
    }
    s
}

fn attacks_square(pos: &Pos, attacker: Piece, dst: u8) -> bool {
    let t = tables();
    if (attacker.sq as usize) >= NSQUARES {
        return false;
    }
    for &n in &t.nei[attacker.sq as usize] {
        if n >= 0 && n as u8 == dst {
            return true;
        }
    }
    if attacker.ptype != LION && attacker.ptype != TIGER {
        return false;
    }
    let horiz = t.horiz_jumps[attacker.sq as usize].iter();
    let verts = if attacker.ptype == LION {
        t.vert_jumps[attacker.sq as usize].iter()
    } else {
        [].iter()
    };
    for &(to, waters) in horiz.chain(verts) {
        if to == dst && pos.rats & waters == 0 {
            return true;
        }
    }
    false
}

fn hanging_term(pos: &Pos) -> i32 {
    let mut s = 0i32;
    for id in 0..MAX_PIECES {
        let p = pos.pieces[id];
        if !p.alive || (p.sq as usize) >= NSQUARES {
            continue;
        }
        let mut friends = 0i32;
        let mut threat = 0i32;
        for oid in 0..MAX_PIECES {
            let other = pos.pieces[oid];
            if !other.alive || other.sq == p.sq || !attacks_square(pos, other, p.sq) {
                continue;
            }
            if other.color == p.color {
                friends += 1;
                continue;
            }
            if pos.can_capture(other, other.sq, p, p.sq) {
                let gain = MATERIAL[p.ptype as usize] - MATERIAL[other.ptype as usize] / 2;
                if gain > threat {
                    threat = gain;
                }
            }
        }
        if threat <= 0 {
            continue;
        }
        let pen = if friends == 0 {
            threat.min(MATERIAL[p.ptype as usize] * 6 / 10)
        } else if friends == 1 {
            threat.min(MATERIAL[p.ptype as usize] / 8)
        } else {
            0
        };
        if pen != 0 {
            s += if p.color == GOLD { -pen } else { pen };
        }
    }
    s
}

fn approach_danger(pos: &Pos, owner: u8) -> i32 {
    let den = own_den(owner);
    let t = tables();
    let mut danger = 0i32;
    for &nb in &t.nei[den as usize] {
        if nb < 0 {
            continue;
        }
        let Some(piece) = pos.piece_at(nb as u8) else {
            continue;
        };
        if piece.color == owner {
            continue;
        }
        let mut recapture = false;
        for &n2 in &t.nei[nb as usize] {
            if n2 < 0 {
                continue;
            }
            if let Some(friend) = pos.piece_at(n2 as u8) {
                if friend.color == owner {
                    recapture = true;
                    break;
                }
            }
        }
        danger += if recapture { 40 } else { 130 };
    }
    if pos.board[den as usize] < 0 {
        let mut guards = 0i32;
        for &nidx in &t.nei[den as usize] {
            if nidx < 0 {
                continue;
            }
            if let Some(p) = pos.piece_at(nidx as u8) {
                if p.color == owner {
                    guards += 1;
                }
            }
        }
        if guards == 0 {
            danger += 35;
        }
    }
    danger
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hanging_own_piece_is_worse() {
        // Scarlet tiger attacks gold wolf; extra gold dog is far vs adjacent.
        let hanging = Pos::parse_fen("7/7/7/7/t6/W6/7/7/6D g").unwrap();
        let defended = Pos::parse_fen("7/7/7/7/t6/WD5/7/7/7 g").unwrap();
        assert!(
            evaluate(&hanging) < evaluate(&defended),
            "hanging={} defended={}",
            evaluate(&hanging),
            evaluate(&defended)
        );
    }

    #[test]
    fn hanging_to_tiger_jump_is_worse() {
        // Scarlet tiger on d4 jumps to gold wolf on a4.
        let hanging = Pos::parse_fen("7/7/7/7/7/W2t3/7/7/6D g").unwrap();
        let defended = Pos::parse_fen("7/7/7/7/7/WD1t3/7/7/7 g").unwrap();
        assert!(
            evaluate(&hanging) < evaluate(&defended),
            "jump hanging={} defended={}",
            evaluate(&hanging),
            evaluate(&defended)
        );
    }
}
