use crate::board::*;
use crate::types::*;

#[derive(Clone, Debug)]
pub struct Pos {
    pub pieces: [Piece; MAX_PIECES],
    pub board: [i8; NSQUARES],
    pub occ: u64,
    pub occ_color: [u64; 2],
    pub rats: u64,
    pub to_move: u8,
    pub halfmove: u8,
    pub hash: u64,
    pub material_pst: i32,
    pub approach: i32,
    pub counts: [u8; 2],
}

#[derive(Clone, Copy, Debug)]
pub struct Undo {
    pub from: u8,
    pub to: u8,
    pub captured: i8,
    pub halfmove: u8,
    pub hash: u64,
    pub material_pst: i32,
    pub approach: i32,
}

static mut Z_PIECE: [[[u64; NSQUARES]; 9]; 2] = [[[0; NSQUARES]; 9]; 2];
static mut Z_SIDE: u64 = 0;
static Z_INIT: std::sync::Once = std::sync::Once::new();

fn splitmix(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

fn init_zobrist() {
    Z_INIT.call_once(|| {
        let mut state = 0xD005_5001u64;
        unsafe {
            for side in 0..2 {
                for pt in 1..9 {
                    for sq in 0..NSQUARES {
                        Z_PIECE[side][pt][sq] = splitmix(&mut state);
                    }
                }
            }
            Z_SIDE = splitmix(&mut state);
        }
    });
}

pub fn piece_key(color: u8, ptype: u8, sq: u8) -> u64 {
    init_zobrist();
    unsafe { Z_PIECE[color as usize][ptype as usize][sq as usize] }
}

pub fn side_key() -> u64 {
    init_zobrist();
    unsafe { Z_SIDE }
}

fn rush(ptype: u8) -> i32 {
    match ptype {
        RAT | CAT | DOG => 14,
        _ => 10,
    }
}

pub fn piece_terms(color: u8, ptype: u8, sq: u8) -> (i32, i32) {
    let t = tables();
    let mut mat = MATERIAL[ptype as usize];
    let dist = t.den_dist[color as usize][ptype as usize][sq as usize];
    let mut app = 0i32;
    if dist < 90 {
        app = (16 - i32::from(dist)) * rush(ptype);
        if dist == 1 {
            app += 120;
        } else if dist == 2 {
            app += 40;
        }
    }
    if ptype == LION && (t.jump_banks & bit(sq)) != 0 {
        app += 36;
    }
    if ptype == LION && (t.horiz_banks & bit(sq)) != 0 {
        app += 28;
    }
    if ptype == TIGER && (t.horiz_banks & bit(sq)) != 0 {
        app += 36;
    }
    if ptype == RAT && is_water_sq(sq) {
        mat += 28;
    }
    match trap_owner(sq) {
        Some(owner) if owner != color => mat -= 110,
        Some(owner) if owner == color => mat += 12,
        _ => {}
    }
    let sign = if color == GOLD { 1 } else { -1 };
    (sign * mat, sign * app)
}

impl Pos {
    pub fn empty(to_move: u8) -> Pos {
        init_zobrist();
        let mut hash = 0u64;
        if to_move == SCARLET {
            hash ^= side_key();
        }
        Pos {
            pieces: [Piece::EMPTY; MAX_PIECES],
            board: [-1; NSQUARES],
            occ: 0,
            occ_color: [0, 0],
            rats: 0,
            to_move,
            halfmove: 0,
            hash,
            material_pst: 0,
            approach: 0,
            counts: [0, 0],
        }
    }

    pub fn start() -> Pos {
        let mut pos = Pos::empty(GOLD);
        const PLACEMENT: [(&str, u8, u8); 16] = [
            ("a1", GOLD, TIGER),
            ("g1", GOLD, LION),
            ("b2", GOLD, CAT),
            ("f2", GOLD, DOG),
            ("a3", GOLD, ELEPHANT),
            ("c3", GOLD, WOLF),
            ("e3", GOLD, LEOPARD),
            ("g3", GOLD, RAT),
            ("a9", SCARLET, LION),
            ("g9", SCARLET, TIGER),
            ("b8", SCARLET, DOG),
            ("f8", SCARLET, CAT),
            ("a7", SCARLET, RAT),
            ("c7", SCARLET, LEOPARD),
            ("e7", SCARLET, WOLF),
            ("g7", SCARLET, ELEPHANT),
        ];
        for (i, (name, color, ptype)) in PLACEMENT.iter().enumerate() {
            pos.put_id(i, parse_sq(name).unwrap(), *color, *ptype);
        }
        pos
    }

    fn alloc_id(&self, color: u8) -> Option<usize> {
        let (lo, hi) = if color == GOLD { (0, 8) } else { (8, 16) };
        (lo..hi).find(|&i| !self.pieces[i].alive)
    }

    fn put_id(&mut self, id: usize, sq: u8, color: u8, ptype: u8) {
        self.pieces[id] = Piece {
            color,
            ptype,
            sq,
            alive: true,
        };
        self.board[sq as usize] = id as i8;
        self.occ |= bit(sq);
        self.occ_color[color as usize] |= bit(sq);
        if ptype == RAT {
            self.rats |= bit(sq);
        }
        self.hash ^= piece_key(color, ptype, sq);
        let (m, a) = piece_terms(color, ptype, sq);
        self.material_pst += m;
        self.approach += a;
        self.counts[color as usize] += 1;
    }

    pub fn place(&mut self, sq: u8, color: u8, ptype: u8) {
        if self.board[sq as usize] >= 0 {
            self.remove_at(sq);
        }
        if let Some(id) = self.alloc_id(color) {
            self.put_id(id, sq, color, ptype);
        }
    }

    pub fn remove_at(&mut self, sq: u8) {
        let id = self.board[sq as usize];
        if id < 0 {
            return;
        }
        self.kill(id as usize);
    }

    fn kill(&mut self, id: usize) {
        let p = self.pieces[id];
        if !p.alive {
            return;
        }
        self.board[p.sq as usize] = -1;
        self.occ &= !bit(p.sq);
        self.occ_color[p.color as usize] &= !bit(p.sq);
        if p.ptype == RAT {
            self.rats &= !bit(p.sq);
        }
        self.hash ^= piece_key(p.color, p.ptype, p.sq);
        let (m, a) = piece_terms(p.color, p.ptype, p.sq);
        self.material_pst -= m;
        self.approach -= a;
        self.counts[p.color as usize] -= 1;
        self.pieces[id].alive = false;
    }

    pub fn piece_at(&self, sq: u8) -> Option<Piece> {
        let id = self.board[sq as usize];
        if id < 0 {
            None
        } else {
            Some(self.pieces[id as usize])
        }
    }

    pub fn effective_rank(&self, piece: Piece, square: u8) -> i32 {
        if let Some(owner) = trap_owner(square) {
            if owner != piece.color {
                return 0;
            }
        }
        i32::from(piece.ptype)
    }

    pub fn can_capture(&self, attacker: Piece, src: u8, defender: Piece, dst: u8) -> bool {
        if attacker.color == defender.color {
            return false;
        }
        if is_water_sq(dst) && (attacker.ptype != RAT || !is_water_sq(src)) {
            return false;
        }
        // A swimming Rat may only capture in water (another swimming Rat).
        if attacker.ptype == RAT && is_water_sq(src) && !is_water_sq(dst) {
            return false;
        }
        if attacker.ptype == RAT && defender.ptype == ELEPHANT {
            return !is_water_sq(src) && !is_water_sq(dst);
        }
        if attacker.ptype == ELEPHANT && defender.ptype == RAT {
            return self.effective_rank(defender, dst) == 0 && !is_water_sq(dst);
        }
        self.effective_rank(attacker, src) >= self.effective_rank(defender, dst)
    }

    pub fn can_land(&self, piece: Piece, src: u8, dst: u8) -> bool {
        if dst == own_den(piece.color) {
            return false;
        }
        if is_water_sq(dst) && piece.ptype != RAT {
            return false;
        }
        match self.piece_at(dst) {
            None => true,
            Some(occ) => self.can_capture(piece, src, occ, dst),
        }
    }

    fn dummy_undo(&self) -> Undo {
        Undo {
            from: 0xff,
            to: 0xff,
            captured: -1,
            halfmove: self.halfmove,
            hash: self.hash,
            material_pst: self.material_pst,
            approach: self.approach,
        }
    }

    pub fn make(&mut self, mv: Move) -> Undo {
        let src = mv.from;
        let dst = mv.to;
        if (src as usize) >= NSQUARES || (dst as usize) >= NSQUARES {
            return self.dummy_undo();
        }
        let raw = self.board[src as usize];
        if raw < 0 || (raw as usize) >= MAX_PIECES {
            return self.dummy_undo();
        }
        let id = raw as usize;
        let captured = self.board[dst as usize];
        let undo = Undo {
            from: src,
            to: dst,
            captured,
            halfmove: self.halfmove,
            hash: self.hash,
            material_pst: self.material_pst,
            approach: self.approach,
        };
        if captured >= 0 {
            self.kill(captured as usize);
            self.halfmove = 0;
        } else {
            self.halfmove = self.halfmove.saturating_add(1);
        }
        let mut p = self.pieces[id];
        self.hash ^= piece_key(p.color, p.ptype, src);
        let (om, oa) = piece_terms(p.color, p.ptype, src);
        self.material_pst -= om;
        self.approach -= oa;
        self.board[src as usize] = -1;
        self.occ &= !bit(src);
        self.occ_color[p.color as usize] &= !bit(src);
        if p.ptype == RAT {
            self.rats &= !bit(src);
        }
        p.sq = dst;
        self.pieces[id] = p;
        self.board[dst as usize] = id as i8;
        self.occ |= bit(dst);
        self.occ_color[p.color as usize] |= bit(dst);
        if p.ptype == RAT {
            self.rats |= bit(dst);
        }
        self.hash ^= piece_key(p.color, p.ptype, dst);
        let (nm, na) = piece_terms(p.color, p.ptype, dst);
        self.material_pst += nm;
        self.approach += na;
        self.to_move ^= 1;
        self.hash ^= side_key();
        undo
    }

    pub fn unmake(&mut self, mv: Move, u: Undo) {
        if u.from == 0xff {
            return;
        }
        if (mv.to as usize) >= NSQUARES || (mv.from as usize) >= NSQUARES {
            return;
        }
        let raw = self.board[mv.to as usize];
        if raw < 0 || (raw as usize) >= MAX_PIECES {
            return;
        }
        let id = raw as usize;
        let mut p = self.pieces[id];
        self.board[mv.to as usize] = -1;
        self.occ &= !bit(mv.to);
        self.occ_color[p.color as usize] &= !bit(mv.to);
        if p.ptype == RAT {
            self.rats &= !bit(mv.to);
        }
        p.sq = mv.from;
        self.pieces[id] = p;
        self.board[mv.from as usize] = id as i8;
        self.occ |= bit(mv.from);
        self.occ_color[p.color as usize] |= bit(mv.from);
        if p.ptype == RAT {
            self.rats |= bit(mv.from);
        }
        if u.captured >= 0 {
            let cid = u.captured as usize;
            let c = self.pieces[cid];
            // captured piece fields except alive/sq are intact
            let sq = mv.to;
            self.pieces[cid].alive = true;
            self.pieces[cid].sq = sq;
            self.board[sq as usize] = u.captured;
            self.occ |= bit(sq);
            self.occ_color[c.color as usize] |= bit(sq);
            if c.ptype == RAT {
                self.rats |= bit(sq);
            }
            self.counts[c.color as usize] += 1;
        }
        self.to_move ^= 1;
        self.halfmove = u.halfmove;
        self.hash = u.hash;
        self.material_pst = u.material_pst;
        self.approach = u.approach;
    }

    pub fn make_null(&mut self) {
        self.to_move ^= 1;
        self.hash ^= side_key();
    }

    pub fn unmake_null(&mut self) {
        self.to_move ^= 1;
        self.hash ^= side_key();
    }

    pub fn fen(&self) -> String {
        let mut ranks = Vec::with_capacity(9);
        for rank in (0..RANKS).rev() {
            let mut empty = 0;
            let mut row = String::new();
            for file in 0..FILES {
                let s = sq(file, rank);
                if let Some(p) = self.piece_at(s) {
                    if empty > 0 {
                        row.push(char::from(b'0' + empty));
                        empty = 0;
                    }
                    row.push(fen_char(p.color, p.ptype));
                } else {
                    empty += 1;
                }
            }
            if empty > 0 {
                row.push(char::from(b'0' + empty));
            }
            ranks.push(row);
        }
        let stm = if self.to_move == GOLD { 'g' } else { 's' };
        format!("{} {stm}", ranks.join("/"))
    }

    pub fn parse_fen(fen: &str) -> Result<Pos, String> {
        let fen = fen.trim();
        let (body, stm) = match fen.split_once(' ') {
            Some((b, s)) => (b, s),
            None => (fen, "g"),
        };
        let to_move = if stm.starts_with('s') || stm.starts_with('S') {
            SCARLET
        } else {
            GOLD
        };
        let rows: Vec<&str> = body.split('/').collect();
        if rows.len() != RANKS as usize {
            return Err(format!("expected 9 ranks, got {}", rows.len()));
        }
        let mut pos = Pos::empty(to_move);
        for (rank_from_top, row) in rows.iter().enumerate() {
            let rank = RANKS - 1 - rank_from_top as i32;
            let mut file = 0i32;
            for ch in row.chars() {
                if ch.is_ascii_digit() {
                    file += i32::from(ch.to_digit(10).unwrap() as u8);
                    continue;
                }
                let (color, ptype) =
                    parse_fen_char(ch).ok_or_else(|| format!("bad FEN piece {ch}"))?;
                if file >= FILES {
                    return Err("file overflow".into());
                }
                pos.place(sq(file, rank), color, ptype);
                file += 1;
            }
            if file != FILES {
                return Err(format!("rank {} has {file} files", rank + 1));
            }
        }
        Ok(pos)
    }

    pub fn remaining(&self) -> i32 {
        i32::from(self.counts[0]) + i32::from(self.counts[1])
    }
}

pub fn dump_fen(pos: &Pos) -> String {
    pos.fen()
}
