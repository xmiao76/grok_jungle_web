//! JSON game session matching the previous `web_api.py` envelope.

use crate::movegen::gen_moves;
use crate::pos::Pos;
use crate::rules::{self, Verdict};
use crate::search::{Limits, Searcher};
use crate::types::*;
use crate::wire::{self, BLUE, RED};
use serde_json::{json, Value};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Res {
    Ongoing,
    GoldWins,
    ScarletWins,
    Draw,
}

impl Res {
    fn as_str(self) -> &'static str {
        match self {
            Res::Ongoing => "ongoing",
            Res::GoldWins => "red_wins",
            Res::ScarletWins => "blue_wins",
            Res::Draw => "draw",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Out {
    Den,
    Elimination,
    Immobilization,
    Resign,
    Repetition,
    NoCapture,
}

impl Out {
    fn as_str(self) -> &'static str {
        match self {
            Out::Den => "den",
            Out::Elimination => "elimination",
            Out::Immobilization => "immobilization",
            Out::Resign => "resign",
            Out::Repetition => "repetition",
            Out::NoCapture => "no-capture",
        }
    }

    fn from_reason(reason: &str) -> Option<Self> {
        Some(match reason {
            "den" => Out::Den,
            "elimination" => Out::Elimination,
            "immobilization" => Out::Immobilization,
            "repetition" => Out::Repetition,
            "no-capture" => Out::NoCapture,
            _ => return None,
        })
    }
}

#[derive(Clone)]
struct Config {
    mode: String,
    human_first: bool,
    difficulty: String,
}

pub struct Game {
    config: Config,
    pos: Pos,
    history: Vec<Pos>,
    moves: Vec<Move>,
    hashes: Vec<u64>,
    result: Res,
    outcome: Option<Out>,
}

impl Game {
    fn new() -> Self {
        let mut g = Self {
            config: Config {
                mode: "human_ai".into(),
                human_first: true,
                difficulty: "medium".into(),
            },
            pos: Pos::start(),
            history: Vec::new(),
            moves: Vec::new(),
            hashes: Vec::new(),
            result: Res::Ongoing,
            outcome: None,
        };
        g.reset(None);
        g
    }

    fn reset(&mut self, cfg: Option<Config>) {
        if let Some(c) = cfg {
            self.config = c;
        }
        self.pos = wire::opening(self.config.human_first);
        self.history = vec![self.pos.clone()];
        self.moves.clear();
        self.hashes = vec![self.pos.hash];
        self.result = Res::Ongoing;
        self.outcome = None;
        self.refresh();
    }

    fn legal(&self) -> MoveList {
        let mut list = MoveList::new();
        if self.result != Res::Ongoing {
            return list;
        }
        gen_moves(&self.pos, &mut list, false);
        list
    }

    fn find_move(&self, src: &str, dst: &str) -> Option<Move> {
        let src = src.trim().to_ascii_lowercase();
        let dst = dst.trim().to_ascii_lowercase();
        let list = self.legal();
        for i in 0..list.len {
            let m = list.moves[i];
            if sq_name(m.from) == src && sq_name(m.to) == dst {
                return Some(m);
            }
        }
        None
    }

    fn play(&mut self, mv: Move) -> Result<(), String> {
        if self.result != Res::Ongoing {
            return Err("game is over".into());
        }
        let list = self.legal();
        if !list
            .iter()
            .any(|m| m.from == mv.from && m.to == mv.to && m.jump == mv.jump)
        {
            return Err(format!("illegal move {}", mv.uci()));
        }
        self.pos.make(mv);
        self.moves.push(mv);
        self.history.push(self.pos.clone());
        self.hashes.push(self.pos.hash);
        self.refresh();
        Ok(())
    }

    fn undo(&mut self, plies: usize) {
        let plies = plies.min(self.moves.len());
        if plies == 0 {
            return;
        }
        let keep = self.history.len() - plies;
        self.history.truncate(keep);
        self.moves.truncate(self.moves.len() - plies);
        self.hashes.truncate(keep);
        self.pos = self.history.last().cloned().unwrap_or_else(Pos::start);
        self.result = Res::Ongoing;
        self.outcome = None;
        self.refresh();
    }

    fn resign(&mut self, side: u8) {
        if self.result != Res::Ongoing {
            return;
        }
        self.result = if side == GOLD {
            Res::ScarletWins
        } else {
            Res::GoldWins
        };
        self.outcome = Some(Out::Resign);
    }

    fn apply_verdict(&mut self, verdict: Verdict) {
        if verdict.is_playing() {
            self.result = Res::Ongoing;
            self.outcome = None;
            return;
        }
        self.outcome = Out::from_reason(verdict.reason);
        self.result = match verdict.reason {
            "repetition" | "no-capture" => Res::Draw,
            _ => match verdict.winner {
                Some(RED) => Res::GoldWins,
                Some(BLUE) => Res::ScarletWins,
                _ => Res::Draw,
            },
        };
    }

    fn refresh(&mut self) {
        if self.outcome == Some(Out::Resign) {
            return;
        }
        self.apply_verdict(rules::adjudicate(&self.pos, &self.hashes));
    }
}

fn piece_noun(pt: u8) -> &'static str {
    match pt {
        RAT => "Rat",
        CAT => "Cat",
        DOG => "Dog",
        WOLF => "Wolf",
        LEOPARD => "Leopard",
        TIGER => "Tiger",
        LION => "Lion",
        ELEPHANT => "Elephant",
        _ => "?",
    }
}

fn piece_type_name(pt: u8) -> &'static str {
    match pt {
        RAT => "rat",
        CAT => "cat",
        DOG => "dog",
        WOLF => "wolf",
        LEOPARD => "leopard",
        TIGER => "tiger",
        LION => "lion",
        ELEPHANT => "elephant",
        _ => "?",
    }
}

fn side_name(side: u8) -> &'static str {
    wire::side_wire(side)
}

fn move_json(mv: Move) -> Value {
    json!({
        "src": sq_name(mv.from),
        "dst": sq_name(mv.to),
        "isJump": mv.jump,
        "uci": mv.uci(),
    })
}

fn captured_lists(pos: &Pos) -> Value {
    // Start has one of each type per side.
    let mut have = [[0u8; 9]; 2];
    for p in &pos.pieces {
        if p.alive && (p.ptype as usize) < 9 {
            have[p.color as usize][p.ptype as usize] += 1;
        }
    }
    let mut red = Vec::new();
    let mut blue = Vec::new();
    for pt in RAT..=ELEPHANT {
        let lost_r = 1u8.saturating_sub(have[RED as usize][pt as usize]);
        let lost_b = 1u8.saturating_sub(have[BLUE as usize][pt as usize]);
        for _ in 0..lost_r {
            red.push(piece_noun(pt));
        }
        for _ in 0..lost_b {
            blue.push(piece_noun(pt));
        }
    }
    json!({ "red": red, "blue": blue })
}

fn state_json(game: &Game) -> Value {
    let mut board = Vec::new();
    for p in &game.pos.pieces {
        if !p.alive {
            continue;
        }
        board.push(json!({
            "sq": sq_name(p.sq),
            "side": side_name(p.color),
            "type": piece_type_name(p.ptype),
            "rank": p.ptype,
        }));
    }
    let last = game.moves.last().copied().map(move_json);
    let legal: Vec<Value> = if game.result == Res::Ongoing {
        game.legal().iter().map(move_json).collect()
    } else {
        Vec::new()
    };
    json!({
        "board": board,
        "turn": side_name(game.pos.to_move),
        "ply": game.moves.len(),
        "legalMoves": legal,
        "lastMove": last,
        "result": game.result.as_str(),
        "outcome": game.outcome.map(Out::as_str),
        "terminal": game.result != Res::Ongoing,
        "fen": game.pos.fen(),
        "history": game.moves.iter().map(|m| m.uci()).collect::<Vec<_>>(),
        "captured": captured_lists(&game.pos),
        "mode": game.config.mode,
        "humanFirst": game.config.human_first,
        "humanSide": "red",
        "difficulty": game.config.difficulty,
        "noCapturePlies": game.pos.halfmove,
    })
}

fn undo_plies(game: &Game) -> usize {
    if game.moves.is_empty() {
        return 0;
    }
    if game.config.mode != "human_ai" {
        return 1;
    }
    let last = *game.moves.last().unwrap();
    let mover = game.pos.piece_at(last.to).map(|p| p.color);
    if mover == Some(RED) || game.moves.len() < 2 {
        1
    } else {
        2
    }
}

fn ok(data: Value) -> String {
    json!({ "ok": true, "data": data, "error": Value::Null }).to_string()
}

fn err(message: &str) -> String {
    json!({ "ok": false, "data": Value::Null, "error": message }).to_string()
}

/// Easy and medium spend less than the one-second Hard ceiling.
/// Easy is depth 2 with seeded noise and no book or tablebase.
/// Medium is a 400 ms search with book and tablebase.
/// Hard uses the full 850 ms so the reply, including worker messaging, lands within 1 s.
pub fn difficulty_limits(name: &str) -> Option<Limits> {
    Some(match name {
        "easy" => Limits {
            move_time_ms: 150,
            max_depth: 2,
            hard_cap_ms: 150,
            noise: 0.35,
            node_limit: 0,
            use_book: false,
            use_tb: false,
            seed: 1,
        },
        "medium" => Limits {
            move_time_ms: 400,
            max_depth: 24,
            hard_cap_ms: 400,
            noise: 0.0,
            node_limit: 0,
            use_book: true,
            use_tb: true,
            seed: 1,
        },
        "hard" => Limits {
            move_time_ms: 850,
            max_depth: 48,
            hard_cap_ms: 850,
            noise: 0.0,
            node_limit: 0,
            use_book: true,
            use_tb: true,
            seed: 1,
        },
        _ => return None,
    })
}

pub struct Session {
    game: Game,
    searcher: Searcher,
}

impl Session {
    pub fn new() -> Self {
        Self {
            game: Game::new(),
            searcher: Searcher::new_default(),
        }
    }

    pub fn set_tt_mb(&mut self, mb: usize) {
        self.searcher.resize_tt(mb);
    }

    pub fn engine_info() -> String {
        ok(json!({
            "title": crate::identity::title(),
            "gameVersion": crate::identity::GAME_VERSION,
            "engineVersion": crate::identity::ENGINE_VERSION,
            "runtime": crate::identity::RUNTIME,
            "model": crate::identity::MODEL,
            "codeAgent": crate::identity::CODE_AGENT,
            "modelPlatform": crate::identity::MODEL_PLATFORM,
            "githubUrl": crate::identity::GITHUB_URL,
            "rulesId": crate::identity::RULES_ID,
            "rulesDigest": crate::identity::RULES_DIGEST,
            "wasmFilename": crate::identity::wasm_filename(),
        }))
    }

    pub fn new_game(&mut self, mode: &str, human_first: bool, difficulty: &str) -> String {
        if mode != "human_ai" && mode != "ai_ai" {
            return err(&format!("invalid mode: {mode}"));
        }
        if difficulty_limits(difficulty).is_none() {
            return err(&format!("invalid difficulty: {difficulty}"));
        }
        self.game.reset(Some(Config {
            mode: mode.to_string(),
            human_first,
            difficulty: difficulty.to_string(),
        }));
        self.searcher.clear_tables();
        ok(json!({ "state": state_json(&self.game) }))
    }

    pub fn get_state(&self) -> String {
        ok(json!({ "state": state_json(&self.game) }))
    }

    fn commit(&mut self, mv: Move) -> String {
        let captured_piece = self.game.pos.piece_at(mv.to);
        let captured = captured_piece.map(|p| {
            json!({
                "side": side_name(p.color),
                "type": piece_type_name(p.ptype),
                "rank": p.ptype,
            })
        });
        if let Err(e) = self.game.play(mv) {
            return err(&e);
        }
        ok(json!({
            "move": move_json(mv),
            "captured": captured,
            "state": state_json(&self.game),
        }))
    }

    pub fn apply_move(&mut self, src: &str, dst: &str) -> String {
        if self.game.result != Res::Ongoing {
            return err("game is over");
        }
        match self.game.find_move(src, dst) {
            Some(mv) => self.commit(mv),
            None => err(&format!("illegal move: {src}->{dst}")),
        }
    }

    pub fn ai_move(&mut self, time_budget_ms: Option<u64>) -> String {
        if self.game.result != Res::Ongoing {
            return err("game is over");
        }
        let list = self.game.legal();
        if list.len == 0 {
            return err("no legal moves");
        }
        let Some(mut limits) = difficulty_limits(&self.game.config.difficulty) else {
            return err("invalid difficulty");
        };
        if let Some(ms) = time_budget_ms {
            if ms > 0 {
                limits.move_time_ms = ms;
                limits.hard_cap_ms = ms;
            }
        }
        let mv = self
            .searcher
            .search(&mut self.game.pos, limits, &self.game.hashes);
        // search() mutates a copy? No — it make/unmakes on the same pos, so pos is restored.
        // Verify legality against the pre-search list (pos should be unchanged).
        if !list
            .iter()
            .any(|m| m.from == mv.from && m.to == mv.to && m.jump == mv.jump)
        {
            return err(&format!("AI proposed illegal move {}", mv.uci()));
        }
        self.commit(mv)
    }

    pub fn undo(&mut self) -> String {
        if self.game.moves.is_empty() {
            return err("nothing to undo");
        }
        let plies = undo_plies(&self.game);
        self.game.undo(plies);
        ok(json!({ "state": state_json(&self.game) }))
    }

    pub fn resign(&mut self) -> String {
        if self.game.result != Res::Ongoing {
            return err("game is over");
        }
        let side = if self.game.config.mode == "human_ai" {
            GOLD
        } else {
            self.game.pos.to_move
        };
        self.game.resign(side);
        ok(json!({ "state": state_json(&self.game) }))
    }

    pub fn replay_moves(&mut self, moves_json: &str) -> String {
        let parsed: Value = match serde_json::from_str(moves_json) {
            Ok(v) => v,
            Err(e) => return err(&format!("bad moves payload: {e}")),
        };
        let Some(arr) = parsed.as_array() else {
            return err("bad moves payload: expected array");
        };
        let mut applied = 0usize;
        for entry in arr {
            if self.game.result != Res::Ongoing {
                break;
            }
            let (src, dst) = if let Some(s) = entry.as_str() {
                let b = s.as_bytes();
                if !s.is_ascii() || b.len() < 4 {
                    return err(&format!("illegal move at ply {applied}: {s}"));
                }
                (
                    String::from_utf8_lossy(&b[0..2]).into_owned(),
                    String::from_utf8_lossy(&b[2..4]).into_owned(),
                )
            } else if let Some(pair) = entry.as_array() {
                if pair.len() < 2 {
                    return err(&format!("illegal move at ply {applied}"));
                }
                (
                    pair[0].as_str().unwrap_or("").to_string(),
                    pair[1].as_str().unwrap_or("").to_string(),
                )
            } else {
                return err(&format!("illegal move at ply {applied}"));
            };
            match self.game.find_move(&src, &dst) {
                Some(mv) => {
                    if let Err(e) = self.game.play(mv) {
                        return err(&e);
                    }
                    applied += 1;
                }
                None => return err(&format!("illegal move at ply {applied}: {src}->{dst}")),
            }
        }
        ok(json!({ "applied": applied, "state": state_json(&self.game) }))
    }

    pub fn search_raw(
        &mut self,
        fen: &str,
        history_fens: &[String],
        ms: u64,
        depth: i32,
        use_book: bool,
        use_tb: bool,
    ) -> Result<(String, u64, i32), String> {
        let mut pos = Pos::parse_fen(fen)?;
        let mut hist = vec![pos.hash];
        for h in history_fens {
            let hh = Pos::parse_fen(h)?.hash;
            if hh != pos.hash {
                hist.push(hh);
            }
        }
        let limits = Limits {
            move_time_ms: ms,
            max_depth: depth,
            hard_cap_ms: ((ms as f64) * 2.2) as u64,
            noise: 0.0,
            node_limit: 0,
            use_book,
            use_tb,
            seed: 0,
        };
        let rec = crate::search::best_move(&mut pos, limits, &hist);
        Ok((rec.mv.uci(), rec.nodes, rec.depth))
    }

    /// Fresh-budget search. `nodes` is a ceiling on visited positions.
    /// Clock stays off so the result depends only on the position and the budget.
    pub fn search_nodes(
        &mut self,
        fen: &str,
        history_fens: &[String],
        nodes: u64,
        use_book: bool,
        use_tb: bool,
    ) -> Result<(String, u64, i32), String> {
        let mut pos = Pos::parse_fen(fen)?;
        let mut hist = vec![pos.hash];
        for h in history_fens {
            let hh = Pos::parse_fen(h)?.hash;
            if hh != pos.hash {
                hist.push(hh);
            }
        }
        let limits = Limits {
            move_time_ms: 0,
            max_depth: 64,
            hard_cap_ms: 0,
            noise: 0.0,
            node_limit: nodes.max(1),
            use_book,
            use_tb,
            seed: 1,
        };
        let rec = crate::search::best_move(&mut pos, limits, &hist);
        Ok((rec.mv.uci(), rec.nodes, rec.depth))
    }
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_ok(s: &str) -> Value {
        let v: Value = serde_json::from_str(s).unwrap();
        assert!(v["ok"].as_bool().unwrap(), "{s}");
        v["data"].clone()
    }

    #[test]
    fn start_has_24_moves() {
        let mut s = Session::new();
        let data = parse_ok(&s.new_game("human_ai", true, "easy"));
        assert_eq!(data["state"]["legalMoves"].as_array().unwrap().len(), 24);
        assert_eq!(data["state"]["turn"], "red");
        assert!(data["state"]["fen"].as_str().unwrap().ends_with(" g"));
    }

    #[test]
    fn ai_first_scarlet_opens() {
        let mut s = Session::new();
        let data = parse_ok(&s.new_game("human_ai", false, "easy"));
        assert_eq!(data["state"]["turn"], "blue");
        assert_eq!(data["state"]["humanFirst"], false);
    }

    #[test]
    fn illegal_rejected() {
        let mut s = Session::new();
        s.new_game("human_ai", true, "easy");
        let v: Value = serde_json::from_str(&s.apply_move("a1", "d5")).unwrap();
        assert!(!v["ok"].as_bool().unwrap());
    }

    #[test]
    fn apply_and_undo() {
        let mut s = Session::new();
        s.new_game("human_ai", true, "easy");
        let moved = parse_ok(&s.apply_move("g3", "g4"));
        assert_eq!(moved["state"]["ply"], 1);
        assert_eq!(moved["state"]["turn"], "blue");
        let ai = parse_ok(&s.ai_move(Some(40)));
        assert_eq!(ai["state"]["ply"], 2);
        let undone = parse_ok(&s.undo());
        assert_eq!(undone["state"]["ply"], 0);
        assert_eq!(undone["state"]["turn"], "red");
    }

    #[test]
    fn resign_human() {
        let mut s = Session::new();
        s.new_game("human_ai", true, "easy");
        let data = parse_ok(&s.resign());
        assert_eq!(data["state"]["result"], "blue_wins");
        assert_eq!(data["state"]["outcome"], "resign");
    }

    #[test]
    fn replay_opening() {
        let mut s = Session::new();
        s.new_game("human_ai", true, "easy");
        let data = parse_ok(&s.replay_moves(r#"[["g3","g4"],["a7","a6"]]"#));
        assert_eq!(data["applied"], 2);
        assert_eq!(data["state"]["ply"], 2);
    }

    #[test]
    fn undo_after_ai_opening() {
        let mut s = Session::new();
        s.new_game("human_ai", false, "easy");
        let ai = parse_ok(&s.ai_move(Some(40)));
        assert_eq!(ai["state"]["ply"], 1);
        let undone = parse_ok(&s.undo());
        assert_eq!(undone["state"]["ply"], 0);
        assert_eq!(undone["state"]["turn"], "blue");
    }
}
