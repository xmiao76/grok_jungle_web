use crate::board::*;
use crate::eval::evaluate;
use crate::movegen::{den_adjacent, den_adjacent_sq, gen_moves, is_den_entry};
use crate::pos::Pos;
use crate::types::*;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
#[cfg(feature = "clock")]
use web_time::{Duration, Instant};

pub static STOP: AtomicBool = AtomicBool::new(false);

const EXACT: u8 = 0;
const LOWER: u8 = 1;
const UPPER: u8 = 2;

#[derive(Clone, Copy)]
pub struct Limits {
    pub move_time_ms: u64,
    pub max_depth: i32,
    pub hard_cap_ms: u64,
    pub noise: f32,
    pub node_limit: u64,
    pub use_book: bool,
    pub use_tb: bool,
    pub seed: u32,
}

impl Limits {
    pub fn timed(ms: u64, max_depth: i32) -> Self {
        Self {
            move_time_ms: ms,
            max_depth,
            hard_cap_ms: (ms as f64 * 2.2) as u64,
            noise: 0.0,
            node_limit: 0,
            use_book: false,
            use_tb: true,
            seed: 0,
        }
    }
}

#[derive(Clone, Copy, Default)]
struct TTEntry {
    key: u32,
    score: i32,
    mv: u16,
    depth: u8,
    flag: u8,
}

pub struct Searcher {
    tt: Vec<TTEntry>,
    tt_mask: usize,
    killers: [[Move; 2]; MAX_PLY],
    history: [[[i32; NSQUARES]; NSQUARES]; 2],
    pub nodes: u64,
    best: Move,
    node_limit: u64,
    use_clock: bool,
    #[cfg(feature = "clock")]
    deadline: Instant,
    #[cfg(feature = "clock")]
    soft: Instant,
    use_tb: bool,
    pub root_depth: i32,
}

impl Searcher {
    pub fn new(tt_mb: usize) -> Self {
        let bytes = tt_mb.max(1) * 1024 * 1024;
        let n = (bytes / std::mem::size_of::<TTEntry>()).next_power_of_two();
        Self {
            tt: vec![TTEntry::default(); n],
            tt_mask: n - 1,
            killers: [[Move::NONE; 2]; MAX_PLY],
            history: [[[0; NSQUARES]; NSQUARES]; 2],
            nodes: 0,
            best: Move::NONE,
            node_limit: 0,
            use_clock: false,
            #[cfg(feature = "clock")]
            deadline: Instant::now(),
            #[cfg(feature = "clock")]
            soft: Instant::now(),
            use_tb: true,
            root_depth: 0,
        }
    }

    pub fn new_default() -> Self {
        Self::new(16)
    }

    pub fn clear_tables(&mut self) {
        self.tt.fill(TTEntry::default());
        self.killers = [[Move::NONE; 2]; MAX_PLY];
        self.history = [[[0; NSQUARES]; NSQUARES]; 2];
        self.nodes = 0;
        self.best = Move::NONE;
        self.root_depth = 0;
    }

    pub fn resize_tt(&mut self, tt_mb: usize) {
        let bytes = tt_mb.max(1) * 1024 * 1024;
        let n = (bytes / std::mem::size_of::<TTEntry>()).next_power_of_two();
        if n != self.tt.len() {
            self.tt = vec![TTEntry::default(); n];
            self.tt_mask = n - 1;
        }
    }

    pub fn global() -> &'static Mutex<Searcher> {
        static S: OnceLock<Mutex<Searcher>> = OnceLock::new();
        S.get_or_init(|| Mutex::new(Searcher::new_default()))
    }

    fn tt_probe(
        &self,
        hash: u64,
        depth: i32,
        ply: i32,
        alpha: i32,
        beta: i32,
    ) -> (Option<i32>, Move) {
        let idx = (hash as usize) & self.tt_mask;
        let key = (hash >> 32) as u32;
        let e = self.tt[idx];
        if e.key != key || e.depth == 0 {
            return (None, Move::NONE);
        }
        let mv = Move::unpack(e.mv);
        if i32::from(e.depth) > depth {
            let s = tt_from_stored(e.score, ply);
            match e.flag {
                EXACT => return (Some(s), mv),
                LOWER if s >= beta => return (Some(s), mv),
                UPPER if s <= alpha => return (Some(s), mv),
                _ => {}
            }
        }
        (None, mv)
    }

    fn tt_store(&mut self, hash: u64, depth: i32, ply: i32, score: i32, flag: u8, mv: Move) {
        let idx = (hash as usize) & self.tt_mask;
        let key = (hash >> 32) as u32;
        let e = &mut self.tt[idx];
        let nd = (depth + 1).clamp(1, 255) as u8;
        if e.depth != 0 && e.depth > nd && e.key == key {
            return;
        }
        *e = TTEntry {
            key,
            score: tt_to_stored(score, ply),
            mv: mv.pack(),
            depth: nd,
            flag,
        };
    }

    fn timed_out(&self) -> bool {
        if STOP.load(Ordering::Relaxed) {
            return true;
        }
        #[cfg(feature = "clock")]
        if self.use_clock && (self.nodes & 2047) == 0 && Instant::now() >= self.deadline {
            return true;
        }
        false
    }

    fn take_node(&mut self) -> bool {
        if self.node_limit > 0 && self.nodes >= self.node_limit {
            return false;
        }
        self.nodes += 1;
        true
    }

    pub fn search(&mut self, pos: &mut Pos, limits: Limits, history: &[u64]) -> Move {
        STOP.store(false, Ordering::Relaxed);
        self.nodes = 0;
        self.use_tb = limits.use_tb;
        self.node_limit = limits.node_limit;
        self.killers = [[Move::NONE; 2]; MAX_PLY];
        self.use_clock = limits.move_time_ms > 0 || limits.hard_cap_ms > 0;
        #[cfg(feature = "clock")]
        if self.use_clock {
            let now = Instant::now();
            let soft_ms = if limits.move_time_ms > 0 {
                limits.move_time_ms
            } else {
                limits.hard_cap_ms
            };
            let hard_ms = if limits.hard_cap_ms > 0 {
                limits.hard_cap_ms
            } else {
                soft_ms
            };
            self.soft = now + Duration::from_millis(soft_ms.max(1));
            self.deadline = now + Duration::from_millis(hard_ms.max(1));
        }

        if limits.use_book {
            if let Some(mv) = crate::book::probe(pos, limits.seed) {
                self.nodes = 1;
                self.best = mv;
                return mv;
            }
        }
        if limits.use_tb {
            if let Some(mv) = crate::tb::best_move(pos) {
                self.nodes = 1;
                self.best = mv;
                return mv;
            }
        }

        let mut root_moves = MoveList::new();
        gen_moves(pos, &mut root_moves, false);
        if root_moves.len == 0 {
            self.nodes = 1;
            return Move::NONE;
        }
        if root_moves.len == 1 || (0..root_moves.len).any(|i| is_den_entry(pos, root_moves.moves[i]))
        {
            self.nodes = 1;
            if root_moves.len == 1 {
                self.best = root_moves.moves[0];
            } else {
                for i in 0..root_moves.len {
                    if is_den_entry(pos, root_moves.moves[i]) {
                        self.best = root_moves.moves[i];
                        break;
                    }
                }
            }
            return self.best;
        }
        self.order(pos, &mut root_moves, Move::NONE, 0);
        self.best = root_moves.moves[0];

        let extra = if pos.remaining() <= 6 { 1 } else { 0 };
        let max_d = (limits.max_depth + extra).min(MAX_PLY as i32);
        let mut prev = 0i32;
        let mut path: Vec<u64> = history.to_vec();
        if path.is_empty() {
            path.push(pos.hash);
        }
        let mut completed_best = self.best;
        self.root_depth = 0;

        for depth in 1..=max_d {
            #[cfg(feature = "clock")]
            if self.use_clock && depth > 1 && Instant::now() >= self.soft {
                break;
            }
            let window = if depth >= 3 { 48 } else { MATE };
            let alpha = prev - window;
            let beta = prev + window;
            let mut score =
                match self.root(pos, depth, alpha, beta, &mut path, &mut root_moves, limits) {
                    Some(s) => s,
                    None => {
                        self.best = completed_best;
                        break;
                    }
                };
            if score <= alpha || score >= beta {
                match self.root(pos, depth, -MATE, MATE, &mut path, &mut root_moves, limits) {
                    Some(s) => score = s,
                    None => {
                        self.best = completed_best;
                        break;
                    }
                }
            }
            prev = score;
            completed_best = self.best;
            self.root_depth = depth;
            #[cfg(feature = "clock")]
            if self.use_clock && Instant::now() >= self.soft {
                break;
            }
            if score >= MATE - 64 || score <= -MATE + 64 {
                break;
            }
            if self.node_limit > 0 && self.nodes >= self.node_limit {
                break;
            }
        }
        self.best
    }

    #[allow(clippy::too_many_arguments)]
    fn root(
        &mut self,
        pos: &mut Pos,
        depth: i32,
        mut alpha: i32,
        beta: i32,
        path: &mut Vec<u64>,
        moves: &mut MoveList,
        limits: Limits,
    ) -> Option<i32> {
        if !self.take_node() {
            return None;
        }
        let tt_mv = self.tt_probe(pos.hash, depth, 0, alpha, beta).1;
        self.order(pos, moves, tt_mv, 0);
        let mut best = -MATE;
        let mut scored: Vec<(i32, Move)> = Vec::with_capacity(moves.len);
        for i in 0..moves.len {
            let mv = moves.moves[i];
            if mv.is_none() {
                continue;
            }
            let u = pos.make(mv);
            path.push(pos.hash);
            let child = self.alphabeta(pos, depth - 1, -beta, -alpha, path, 1);
            path.pop();
            pos.unmake(mv, u);
            let score = -child?;
            scored.push((score, mv));
            if score > best {
                best = score;
                self.best = mv;
            }
            if score > alpha {
                alpha = score;
            }
        }
        scored.sort_by(|a, b| b.0.cmp(&a.0));
        if let Some(&(s, m)) = scored.first() {
            self.best = m;
            best = s;
            if limits.noise > 0.0 && scored.len() > 1 {
                let floor = scored[0].0 - 50;
                let top: Vec<Move> = scored
                    .iter()
                    .filter(|(sc, _)| *sc >= floor)
                    .take(3)
                    .map(|(_, m)| *m)
                    .collect();
                if !top.is_empty()
                    && xorshift(pos.hash ^ self.nodes ^ u64::from(limits.seed)) as f32
                        / (u64::MAX as f32)
                        < limits.noise
                {
                    let idx = (xorshift(
                        pos.hash
                            .wrapping_add(self.nodes.wrapping_mul(17))
                            ^ u64::from(limits.seed),
                    ) as usize)
                        % top.len();
                    self.best = top[idx];
                }
            }
        }
        Some(best)
    }

    fn order(&self, pos: &Pos, moves: &mut MoveList, tt_mv: Move, ply: usize) {
        let killers = self.killers[ply.min(MAX_PLY - 1)];
        let mut keys = [0i64; MAX_MOVES];
        for i in 0..moves.len {
            let m = moves.moves[i];
            let is_hash = !tt_mv.is_none()
                && m.from == tt_mv.from
                && m.to == tt_mv.to
                && m.jump == tt_mv.jump;
            let den = i64::from(is_den_entry(pos, m));
            let cap = i64::from(mvv_lva(pos, m));
            let see_v = if pos.board[m.to as usize] >= 0 || is_den_entry(pos, m) {
                i64::from(see(pos, m))
            } else {
                0
            };
            let adj_bonus = if den_adjacent(pos, m) { 3_000 } else { 0 };
            let is_killer = (!killers[0].is_none() && m == killers[0])
                || (!killers[1].is_none() && m == killers[1]);
            let hist = if (m.from as usize) < NSQUARES && (m.to as usize) < NSQUARES {
                i64::from(self.history[pos.to_move as usize][m.from as usize][m.to as usize])
            } else {
                0
            };
            keys[i] = (if is_hash { 1_000_000_000 } else { 0 })
                + den * 50_000_000
                + see_v * 20_000
                + adj_bonus
                + cap * 1_000
                + if is_killer { 5_000 } else { 0 }
                + hist;
        }
        for i in 1..moves.len {
            let km = keys[i];
            let mm = moves.moves[i];
            let mut j = i;
            while j > 0 && keys[j - 1] < km {
                keys[j] = keys[j - 1];
                moves.moves[j] = moves.moves[j - 1];
                j -= 1;
            }
            keys[j] = km;
            moves.moves[j] = mm;
        }
    }

    fn alphabeta(
        &mut self,
        pos: &mut Pos,
        depth: i32,
        mut alpha: i32,
        beta: i32,
        path: &mut Vec<u64>,
        ply: i32,
    ) -> Option<i32> {
        if !self.take_node() || self.timed_out() {
            return None;
        }

        if let Some(s) = terminal_score(pos, path, ply) {
            return Some(s);
        }
        if self.use_tb {
            if let Some(s) = crate::tb::score(pos, ply) {
                return Some(s);
            }
        }

        let (tt_hit, tt_mv) = self.tt_probe(pos.hash, depth, ply, alpha, beta);
        if let Some(s) = tt_hit {
            return Some(s);
        }

        let mut local_depth = depth;
        if tt_mv.is_none() && depth >= 4 && ply > 0 {
            local_depth -= 1;
        }

        if local_depth <= 0 || ply as usize >= MAX_PLY - 1 {
            return self.quiesce(pos, alpha, beta, path, ply);
        }

        let stand = evaluate(pos);
        if local_depth <= 2 && stand - 150 * local_depth >= beta && beta < MATE - 400 {
            return Some(stand);
        }

        if local_depth >= 3
            && ply > 0
            && beta < MATE - 400
            && pos.counts[pos.to_move as usize] >= 3
            && pos.counts[(pos.to_move ^ 1) as usize] >= 2
            && stand >= beta
            && !crate::movegen::has_den_entry(pos)
        {
            pos.make_null();
            path.push(pos.hash);
            let r = 2 + local_depth / 4;
            let score = self.alphabeta(pos, local_depth - 1 - r, -beta, -beta + 1, path, ply + 1);
            path.pop();
            pos.unmake_null();
            if let Some(s) = score {
                if -s >= beta {
                    return Some(-s);
                }
            } else {
                return None;
            }
        }

        let mut moves = MoveList::new();
        gen_moves(pos, &mut moves, false);
        if moves.len == 0 {
            return Some(-MATE + ply);
        }

        self.order(pos, &mut moves, tt_mv, ply as usize);
        let orig_alpha = alpha;
        let mut best = -MATE;
        let mut best_move = Move::NONE;

        for i in 0..moves.len {
            let mv = moves.moves[i];
            if mv.is_none() || (mv.from as usize) >= NSQUARES || (mv.to as usize) >= NSQUARES {
                continue;
            }
            let is_cap = pos.board[mv.to as usize] >= 0;
            let den_in = is_den_entry(pos, mv);
            let approaching = approaching_den(pos, mv);
            let ext = if den_in || (approaching && local_depth < 12) {
                1
            } else {
                0
            };
            let mut reduction = 0;
            if local_depth >= 3 && i >= 3 && !is_cap && ext == 0 && !approaching {
                let li = ((i + 1) as f64).ln();
                let ld = (local_depth as f64).ln();
                reduction = (0.75 + ld * li / 2.25) as i32;
                reduction = reduction.clamp(1, local_depth - 1);
            }
            if is_cap && local_depth >= 2 && i >= 2 && see(pos, mv) < 0 && !den_in {
                reduction += 1;
            }

            let u = pos.make(mv);
            path.push(pos.hash);
            let new_depth = local_depth - 1 + ext - reduction;
            let mut score;
            if i == 0 {
                score = self.alphabeta(pos, new_depth, -beta, -alpha, path, ply + 1);
            } else {
                score = self.alphabeta(pos, new_depth, -alpha - 1, -alpha, path, ply + 1);
                if let Some(s) = score {
                    if alpha < -s && -s < beta {
                        score = self.alphabeta(
                            pos,
                            local_depth - 1 + ext,
                            -beta,
                            -alpha,
                            path,
                            ply + 1,
                        );
                    }
                }
            }
            if reduction > 0 {
                if let Some(s) = score {
                    if -s > alpha {
                        score = self.alphabeta(
                            pos,
                            local_depth - 1 + ext,
                            -beta,
                            -alpha,
                            path,
                            ply + 1,
                        );
                    }
                }
            }
            path.pop();
            pos.unmake(mv, u);
            let score = -score?;
            if score > best {
                best = score;
                best_move = mv;
            }
            if score > alpha {
                alpha = score;
            }
            if alpha >= beta {
                if !is_cap && (ply as usize) < MAX_PLY {
                    let k = &mut self.killers[ply as usize];
                    if k[0] != mv {
                        k[1] = k[0];
                        k[0] = mv;
                    }
                    if (mv.from as usize) < NSQUARES && (mv.to as usize) < NSQUARES {
                        let h = &mut self.history[pos.to_move as usize][mv.from as usize]
                            [mv.to as usize];
                        let bonus = local_depth * local_depth;
                        *h += bonus - *h * bonus / 8192;
                        if *h > 16_000 {
                            *h = 16_000;
                        }
                    }
                }
                break;
            }
        }

        let flag = if best <= orig_alpha {
            UPPER
        } else if best >= beta {
            LOWER
        } else {
            EXACT
        };
        self.tt_store(pos.hash, local_depth, ply, best, flag, best_move);
        Some(best)
    }

    fn quiesce(
        &mut self,
        pos: &mut Pos,
        mut alpha: i32,
        beta: i32,
        path: &mut Vec<u64>,
        ply: i32,
    ) -> Option<i32> {
        if !self.take_node() || self.timed_out() {
            return None;
        }
        if ply > 28 {
            return Some(evaluate(pos));
        }
        if let Some(s) = terminal_score(pos, path, ply) {
            return Some(s);
        }
        if self.use_tb {
            if let Some(s) = crate::tb::score(pos, ply) {
                return Some(s);
            }
        }
        let stand = evaluate(pos);
        if stand >= beta {
            return Some(stand);
        }
        if stand > alpha {
            alpha = stand;
        }
        let mut moves = MoveList::new();
        gen_moves(pos, &mut moves, true);
        self.order(pos, &mut moves, Move::NONE, (ply as usize).min(MAX_PLY - 1));
        for i in 0..moves.len {
            let mv = moves.moves[i];
            if mv.is_none() || (mv.to as usize) >= NSQUARES {
                continue;
            }
            let den_in = is_den_entry(pos, mv);
            if !den_in && see(pos, mv) < 0 {
                continue;
            }
            if !den_in {
                let vic = match pos.piece_at(mv.to) {
                    Some(v) => MATERIAL[v.ptype as usize],
                    None => 0,
                };
                if stand + vic + 80 <= alpha {
                    continue;
                }
            }
            let u = pos.make(mv);
            path.push(pos.hash);
            let child = self.quiesce(pos, -beta, -alpha, path, ply + 1);
            path.pop();
            pos.unmake(mv, u);
            let score = -child?;
            if score >= beta {
                return Some(score);
            }
            if score > alpha {
                alpha = score;
            }
        }
        Some(alpha)
    }
}

fn tt_to_stored(score: i32, ply: i32) -> i32 {
    if score >= MATE - MAX_PLY as i32 {
        score + ply
    } else if score <= -MATE + MAX_PLY as i32 {
        score - ply
    } else {
        score
    }
}

fn tt_from_stored(score: i32, ply: i32) -> i32 {
    if score >= MATE - MAX_PLY as i32 {
        score - ply
    } else if score <= -MATE + MAX_PLY as i32 {
        score + ply
    } else {
        score
    }
}

fn approaching_den(pos: &Pos, mv: Move) -> bool {
    if !den_adjacent(pos, mv) {
        return false;
    }
    match pos.piece_at(mv.from) {
        Some(p) => !den_adjacent_sq(p.color, mv.from),
        None => false,
    }
}

fn terminal_score(pos: &Pos, path: &[u64], ply: i32) -> Option<i32> {
    let verdict = crate::rules::adjudicate(pos, path);
    if verdict.is_playing() {
        return None;
    }
    match verdict.reason {
        "repetition" | "no-capture" => Some(0),
        _ => {
            let winner = verdict.winner?;
            let score = MATE - ply;
            Some(if pos.to_move == winner { score } else { -score })
        }
    }
}

fn mvv_lva(pos: &Pos, mv: Move) -> i32 {
    let Some(att) = pos.piece_at(mv.from) else {
        return 0;
    };
    let Some(vic) = pos.piece_at(mv.to) else {
        return if is_den_entry(pos, mv) { 10_000 } else { 0 };
    };
    MATERIAL[vic.ptype as usize] * 16 - MATERIAL[att.ptype as usize] + if mv.jump { 50 } else { 0 }
}

pub fn see(pos: &Pos, mv: Move) -> i32 {
    if is_den_entry(pos, mv) {
        return 20_000;
    }
    let Some(attacker) = pos.piece_at(mv.from) else {
        return 0;
    };
    let victim_val = match pos.piece_at(mv.to) {
        Some(v) => MATERIAL[v.ptype as usize],
        None => return 0,
    };
    let mut occ = pos.occ ^ bit(mv.from);
    let mut rats = pos.rats;
    if attacker.ptype == RAT {
        rats &= !bit(mv.from);
    }
    if let Some(v) = pos.piece_at(mv.to) {
        if v.ptype == RAT {
            rats &= !bit(mv.to);
        }
    }
    let mut piece_on = attacker;
    piece_on.sq = mv.to;
    let mut stm = attacker.color ^ 1;
    let mut gain = [0i32; 16];
    gain[0] = victim_val;
    let mut d = 1usize;
    while d < 16 {
        let Some((id, att)) = least_attacker(pos, stm, mv.to, occ, rats) else {
            break;
        };
        if !pos.can_capture(att, att.sq, piece_on, mv.to) {
            occ &= !bit(att.sq);
            continue;
        }
        gain[d] = MATERIAL[piece_on.ptype as usize] - gain[d - 1];
        occ &= !bit(att.sq);
        if att.ptype == RAT {
            rats &= !bit(att.sq);
        }
        piece_on = att;
        piece_on.sq = mv.to;
        stm ^= 1;
        d += 1;
        let _ = id;
    }
    for i in (1..d).rev() {
        gain[i - 1] = -(-gain[i - 1]).min(gain[i]);
    }
    gain[0]
}

fn least_attacker(pos: &Pos, side: u8, dst: u8, occ: u64, rats: u64) -> Option<(usize, Piece)> {
    let t = tables();
    let mut best: Option<(usize, Piece)> = None;
    let mut best_v = i32::MAX;
    for id in 0..MAX_PIECES {
        let p = pos.pieces[id];
        if !p.alive || p.color != side {
            continue;
        }
        if occ & bit(p.sq) == 0 {
            continue;
        }
        let attacks = {
            let mut ok = false;
            for &n in &t.nei[p.sq as usize] {
                if n >= 0 && n as u8 == dst {
                    ok = true;
                    break;
                }
            }
            if !ok && (p.ptype == LION || p.ptype == TIGER) {
                let horiz = t.horiz_jumps[p.sq as usize].iter();
                let verts = if p.ptype == LION {
                    t.vert_jumps[p.sq as usize].iter()
                } else {
                    [].iter()
                };
                for &(to, waters) in horiz.chain(verts) {
                    if to == dst && (rats & waters) == 0 {
                        ok = true;
                        break;
                    }
                }
            }
            ok
        };
        if !attacks {
            continue;
        }
        let v = MATERIAL[p.ptype as usize];
        if v < best_v {
            best_v = v;
            best = Some((id, p));
        }
    }
    best
}

fn xorshift(mut x: u64) -> u64 {
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    x
}

pub struct SearchResult {
    pub mv: Move,
    pub nodes: u64,
    pub depth: i32,
    pub score: i32,
}

/// Hard uses the whole budget. Medium uses one third. Easy caps at 200 nodes.
pub fn arena_node_limit(level: &str, budget: u32) -> u64 {
    let budget = u64::from(budget.max(1));
    match level {
        "easy" => budget.min(200),
        "medium" => (budget / 3).max(1),
        _ => budget,
    }
}

pub fn best_move(pos: &mut Pos, limits: Limits, history: &[u64]) -> SearchResult {
    let mut s = Searcher::global().lock().unwrap_or_else(|p| p.into_inner());
    let mv = s.search(pos, limits, history);
    let mv = fallback_legal(pos, mv);
    SearchResult {
        mv,
        nodes: s.nodes,
        depth: s.root_depth,
        score: 0,
    }
}

fn fallback_legal(pos: &Pos, mv: Move) -> Move {
    if !mv.is_none() {
        return mv;
    }
    let mut list = MoveList::new();
    gen_moves(pos, &mut list, false);
    if list.len > 0 {
        list.moves[0]
    } else {
        Move::NONE
    }
}
