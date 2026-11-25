//! Arena guest. One fresh set of tables per `choose-move`. Rendering does not
//! touch the searcher.

use crate::identity;
use crate::paint::{self, Frame, SpritePiece};
use crate::rules;
use crate::search::{Limits, Searcher};
use crate::wire::{self, WirePiece, BLUE, RED};
use std::sync::Mutex;

wit_bindgen::generate!({
    world: "competitor",
    path: "../wit",
});

use exports::jungle::arena::game::{
    Animal, Guest, Level, Metadata, Phase, Piece, PlyMove, Settings, Side, Snapshot,
};

struct Instance {
    level: Level,
    seed: u32,
    searcher: Searcher,
}

fn instance() -> &'static Mutex<Instance> {
    static S: std::sync::OnceLock<Mutex<Instance>> = std::sync::OnceLock::new();
    S.get_or_init(|| {
        Mutex::new(Instance {
            level: Level::Medium,
            seed: 1,
            searcher: Searcher::new(1),
        })
    })
}

struct Competitor;

impl Guest for Competitor {
    fn info() -> Metadata {
        Metadata {
            title: identity::title(),
            game_version: identity::GAME_VERSION.into(),
            model: identity::MODEL.into(),
            code_agent: identity::CODE_AGENT.into(),
            model_platform: identity::MODEL_PLATFORM.into(),
            github_url: identity::GITHUB_URL.into(),
            engine_version: identity::ENGINE_VERSION.into(),
            runtime: identity::RUNTIME.into(),
            rules_id: identity::RULES_ID.into(),
            rules_digest: identity::RULES_DIGEST.into(),
        }
    }

    fn init(config: Settings) {
        let mut inst = instance().lock().unwrap_or_else(|p| p.into_inner());
        inst.level = config.difficulty;
        inst.seed = config.seed;
        inst.searcher.clear_tables();
    }

    fn choose_move(state: Snapshot, legal: Vec<PlyMove>, node_budget: u32) -> u32 {
        if legal.is_empty() || node_budget == 0 {
            return 0;
        }
        let mut inst = instance().lock().unwrap_or_else(|p| p.into_inner());
        let level_name = match inst.level {
            Level::Easy => "easy",
            Level::Medium => "medium",
            Level::Hard => "hard",
        };
        let seed = inst.seed;
        inst.searcher.clear_tables();
        let (mut pos, hashes) = position_from(&state);
        let limits = Limits {
            move_time_ms: 0,
            max_depth: if level_name == "easy" { 2 } else { 48 },
            hard_cap_ms: 0,
            noise: if level_name == "easy" { 0.35 } else { 0.0 },
            node_limit: crate::search::arena_node_limit(level_name, node_budget),
            use_book: level_name != "easy",
            use_tb: level_name != "easy",
            seed,
        };
        let mv = inst.searcher.search(&mut pos, limits, &hashes);
        if mv.is_none() {
            return 0;
        }
        let src = wire::engine_to_wire(mv.from);
        let dst = wire::engine_to_wire(mv.to);
        legal
            .iter()
            .position(|m| m.src == src && m.dst == dst)
            .unwrap_or(0) as u32
    }

    fn render(state: Snapshot, flipped: bool, progress: u16) {
        let frame = frame_from(&state, progress);
        let rgba = paint::render(&frame, flipped, progress);
        jungle::arena::display::present(paint::FRAME_W as u16, paint::FRAME_H as u16, &rgba);
    }
}

export!(Competitor);

fn side_to_color(side: Side) -> u8 {
    match side {
        Side::Red => RED,
        Side::Blue => BLUE,
    }
}

fn animal_kind(animal: Animal) -> u8 {
    match animal {
        Animal::Rat => 1,
        Animal::Cat => 2,
        Animal::Dog => 3,
        Animal::Wolf => 4,
        Animal::Leopard => 5,
        Animal::Tiger => 6,
        Animal::Lion => 7,
        Animal::Elephant => 8,
    }
}

fn position_from(state: &Snapshot) -> (crate::pos::Pos, Vec<u64>) {
    let cells = wire_cells(&state.board);
    let first_red = matches!(state.first_side, Side::Red);
    let to_move = side_to_color(state.to_play);
    let half = state.no_capture_plies.min(255) as u8;
    let mut replay = wire::opening(first_red);
    let mut hashes = vec![replay.hash];
    let mut ok = state.board.len() == 63;
    for ply in &state.history {
        if !rules::commit_wire(&mut replay, &mut hashes, ply.src, ply.dst) {
            ok = false;
            break;
        }
    }
    let direct = wire::pos_from_wire(&cells, to_move, half);
    if ok && replay.fen() == direct.fen() {
        replay.halfmove = half;
        (replay, hashes)
    } else {
        let hash = direct.hash;
        (direct, vec![hash])
    }
}

fn wire_cells(board: &[Option<Piece>]) -> Vec<Option<WirePiece>> {
    let mut cells = vec![None; 63];
    for (i, cell) in board.iter().take(63).enumerate() {
        if let Some(piece) = cell {
            cells[i] = Some(WirePiece {
                color: side_to_color(piece.owner),
                kind: animal_kind(piece.kind),
            });
        }
    }
    cells
}

fn frame_from(state: &Snapshot, progress: u16) -> Frame {
    let cells = wire_cells(&state.board);
    let mut frame = Frame::empty();
    for (i, cell) in cells.iter().enumerate() {
        if let Some(p) = cell {
            frame.cells[i] = Some(SpritePiece {
                color: p.color,
                kind: p.kind,
            });
        }
    }
    if let Some(last) = state.history.last() {
        frame.last_src = Some(last.src);
        frame.last_dst = Some(last.dst);
        if progress < 1000 {
            frame.captured = captured_before(state, last.src, last.dst);
        }
    }
    frame.phase = match state.status {
        Phase::Playing => "playing",
        Phase::Won => "won",
        Phase::Drawn => "drawn",
        Phase::Forfeited => "forfeited",
    };
    frame.winner = match state.winner {
        Some(Side::Red) => "red",
        Some(Side::Blue) => "blue",
        None => "",
    };
    frame.reason = match state.reason.as_str() {
        "den" => "den",
        "elimination" => "elimination",
        "immobilization" => "immobilization",
        "repetition" => "repetition",
        "no-capture" => "no-capture",
        "timeout" => "timeout",
        "trap" => "trap",
        "invalid-move" => "invalid-move",
        _ => "",
    };
    frame
}

fn captured_before(state: &Snapshot, src: u8, dst: u8) -> Option<SpritePiece> {
    if state.history.len() < 1 {
        return None;
    }
    let mut replay = wire::opening(matches!(state.first_side, Side::Red));
    let mut hashes = vec![replay.hash];
    for ply in &state.history[..state.history.len() - 1] {
        if !rules::commit_wire(&mut replay, &mut hashes, ply.src, ply.dst) {
            return None;
        }
    }
    let from = wire::wire_to_engine(src);
    let to = wire::wire_to_engine(dst);
    let victim = replay.piece_at(to)?;
    let mover = replay.piece_at(from)?;
    if mover.color == victim.color {
        return None;
    }
    Some(SpritePiece {
        color: victim.color,
        kind: victim.ptype,
    })
}
