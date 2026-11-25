use crate::board::*;
use crate::pos::Pos;
use crate::types::*;

fn tactical_dst(pos: &Pos, piece: Piece, dst: u8) -> bool {
    if pos.board[dst as usize] >= 0 {
        return true;
    }
    dst == enemy_den(piece.color)
}

pub fn den_adjacent_sq(color: u8, dst: u8) -> bool {
    let den = enemy_den(color);
    (file_of(dst) - file_of(den)).abs() + (rank_of(dst) - rank_of(den)).abs() == 1
}

pub fn is_den_entry(pos: &Pos, mv: Move) -> bool {
    match pos.piece_at(mv.from) {
        Some(p) => mv.to == enemy_den(p.color),
        None => false,
    }
}

pub fn den_adjacent(pos: &Pos, mv: Move) -> bool {
    match pos.piece_at(mv.from) {
        Some(p) => {
            let den = enemy_den(p.color);
            (file_of(mv.to) - file_of(den)).abs() + (rank_of(mv.to) - rank_of(den)).abs() == 1
        }
        None => false,
    }
}

fn add_jumps(pos: &Pos, src: u8, piece: Piece, caps_only: bool, out: &mut MoveList) {
    let t = tables();
    let horiz = t.horiz_jumps[src as usize].iter();
    let verts = if piece.ptype == LION {
        t.vert_jumps[src as usize].iter()
    } else {
        [].iter()
    };
    let tiger_ok = piece.ptype == TIGER || piece.ptype == LION;
    if !tiger_ok {
        return;
    }
    for &(dst, waters) in horiz.chain(verts) {
        if pos.rats & waters != 0 {
            continue;
        }
        if !pos.can_land(piece, src, dst) {
            continue;
        }
        if caps_only && !tactical_dst(pos, piece, dst) {
            continue;
        }
        out.push(Move {
            from: src,
            to: dst,
            jump: true,
        });
    }
}

pub fn gen_moves(pos: &Pos, out: &mut MoveList, caps_only: bool) {
    let t = tables();
    let stm = pos.to_move;
    for id in 0..MAX_PIECES {
        let p = pos.pieces[id];
        if !p.alive || p.color != stm {
            continue;
        }
        let src = p.sq;
        for &n in &t.nei[src as usize] {
            if n < 0 {
                continue;
            }
            let dst = n as u8;
            if !pos.can_land(p, src, dst) {
                continue;
            }
            if caps_only && !tactical_dst(pos, p, dst) {
                continue;
            }
            out.push(Move {
                from: src,
                to: dst,
                jump: false,
            });
        }
        if p.ptype == LION || p.ptype == TIGER {
            add_jumps(pos, src, p, caps_only, out);
        }
    }
}

pub fn legal_uci(pos: &Pos) -> Vec<String> {
    let mut list = MoveList::new();
    gen_moves(pos, &mut list, false);
    list.iter().map(|m| m.uci()).collect()
}

pub fn perft(pos: &mut Pos, depth: u32) -> u64 {
    if depth == 0 {
        return 1;
    }
    let mut list = MoveList::new();
    gen_moves(pos, &mut list, false);
    if depth == 1 {
        return list.len as u64;
    }
    let mut total = 0u64;
    for i in 0..list.len {
        let mv = list.moves[i];
        let u = pos.make(mv);
        total += perft(pos, depth - 1);
        pos.unmake(mv, u);
    }
    total
}

pub fn has_move(pos: &Pos) -> bool {
    let mut list = MoveList::new();
    gen_moves(pos, &mut list, false);
    list.len > 0
}

pub fn has_den_entry(pos: &Pos) -> bool {
    let den = enemy_den(pos.to_move);
    if pos.board[den as usize] >= 0 {
        return false;
    }
    let t = tables();
    for &nb in &t.nei[den as usize] {
        if nb < 0 {
            continue;
        }
        if let Some(p) = pos.piece_at(nb as u8) {
            if p.color == pos.to_move && pos.can_land(p, p.sq, den) {
                return true;
            }
        }
    }
    false
}
