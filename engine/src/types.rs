pub const FILES: i32 = 7;
pub const RANKS: i32 = 9;
pub const NSQUARES: usize = 63;
pub const MAX_PIECES: usize = 16;
pub const MAX_MOVES: usize = 48;
pub const MAX_PLY: usize = 64;

pub const GOLD: u8 = 0;
pub const SCARLET: u8 = 1;

pub const RAT: u8 = 1;
pub const CAT: u8 = 2;
pub const DOG: u8 = 3;
pub const WOLF: u8 = 4;
pub const LEOPARD: u8 = 5;
pub const TIGER: u8 = 6;
pub const LION: u8 = 7;
pub const ELEPHANT: u8 = 8;

pub const MATE: i32 = 100_000;
pub const HALFMOVE_DRAW: u8 = 100;

// Rat up (swim + elephant-killer). Elephant down (cannot take Rat).
// Lion up (unique three-block jump). Tiger slightly above leopard-class.
pub const MATERIAL: [i32; 9] = [0, 230, 230, 330, 430, 560, 820, 1000, 720];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Move {
    pub from: u8,
    pub to: u8,
    pub jump: bool,
}

impl Move {
    pub const NONE: Move = Move {
        from: 0xff,
        to: 0xff,
        jump: false,
    };

    pub fn is_none(self) -> bool {
        self.from == 0xff
    }

    pub fn pack(self) -> u16 {
        if self.is_none() {
            return 0xffff;
        }
        u16::from(self.from) | (u16::from(self.to) << 6) | (u16::from(self.jump) << 12)
    }

    pub fn unpack(v: u16) -> Move {
        if v == 0xffff {
            return Move::NONE;
        }
        Move {
            from: (v & 63) as u8,
            to: ((v >> 6) & 63) as u8,
            jump: ((v >> 12) & 1) != 0,
        }
    }

    pub fn uci(self) -> String {
        if self.is_none() {
            return "0000".to_string();
        }
        let mut s = sq_name(self.from);
        s.push_str(&sq_name(self.to));
        if self.jump {
            s.push('j');
        }
        s
    }

    pub fn parse(text: &str) -> Option<Move> {
        let t = text.trim().to_ascii_lowercase();
        let (body, jump) = if t.ends_with('j') {
            (&t[..t.len() - 1], true)
        } else {
            (t.as_str(), false)
        };
        if body.len() < 4 {
            return None;
        }
        let from = parse_sq(&body[..2])?;
        let to = parse_sq(&body[2..4])?;
        Some(Move { from, to, jump })
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Piece {
    pub color: u8,
    pub ptype: u8,
    pub sq: u8,
    pub alive: bool,
}

impl Piece {
    pub const EMPTY: Piece = Piece {
        color: 0,
        ptype: 0,
        sq: 0,
        alive: false,
    };
}

pub fn sq(file: i32, rank: i32) -> u8 {
    (rank * FILES + file) as u8
}

pub fn file_of(s: u8) -> i32 {
    i32::from(s) % FILES
}

pub fn rank_of(s: u8) -> i32 {
    i32::from(s) / FILES
}

pub fn on_board(file: i32, rank: i32) -> bool {
    (0..FILES).contains(&file) && (0..RANKS).contains(&rank)
}

pub fn sq_name(s: u8) -> String {
    format!("{}{}", (b'a' + file_of(s) as u8) as char, rank_of(s) + 1)
}

pub fn parse_sq(name: &str) -> Option<u8> {
    let b = name.as_bytes();
    if b.len() < 2 {
        return None;
    }
    let file = i32::from(b[0]) - i32::from(b'a');
    let rank = i32::from(b[1]) - i32::from(b'1');
    if on_board(file, rank) {
        Some(sq(file, rank))
    } else {
        None
    }
}

pub fn fen_char(color: u8, ptype: u8) -> char {
    let ch = match ptype {
        RAT => 'R',
        CAT => 'C',
        DOG => 'D',
        WOLF => 'W',
        LEOPARD => 'P',
        TIGER => 'T',
        LION => 'L',
        ELEPHANT => 'E',
        _ => '?',
    };
    if color == GOLD {
        ch
    } else {
        ch.to_ascii_lowercase()
    }
}

pub fn parse_fen_char(ch: char) -> Option<(u8, u8)> {
    let color = if ch.is_ascii_uppercase() {
        GOLD
    } else {
        SCARLET
    };
    let ptype = match ch.to_ascii_uppercase() {
        'R' => RAT,
        'C' => CAT,
        'D' => DOG,
        'W' => WOLF,
        'P' => LEOPARD,
        'T' => TIGER,
        'L' => LION,
        'E' => ELEPHANT,
        _ => return None,
    };
    Some((color, ptype))
}

pub struct MoveList {
    pub moves: [Move; MAX_MOVES],
    pub len: usize,
}

impl MoveList {
    pub fn new() -> Self {
        Self {
            moves: [Move::NONE; MAX_MOVES],
            len: 0,
        }
    }

    pub fn push(&mut self, m: Move) {
        if self.len < MAX_MOVES {
            self.moves[self.len] = m;
            self.len += 1;
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = Move> + '_ {
        self.moves[..self.len].iter().copied()
    }

    pub fn as_slice(&self) -> &[Move] {
        &self.moves[..self.len]
    }
}

impl Default for MoveList {
    fn default() -> Self {
        Self::new()
    }
}
