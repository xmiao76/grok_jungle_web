//! Legal-index-0 opponent with a flat board, so a reviewer can tell the seats apart.

wit_bindgen::generate!({
    world: "competitor",
    path: "../wit",
});

use exports::jungle::arena::game::{Guest, Level, Metadata, Piece, PlyMove, Settings, Side, Snapshot};

struct Fixture {
    seat: Side,
}

fn seat() -> &'static std::sync::Mutex<Side> {
    static S: std::sync::OnceLock<std::sync::Mutex<Side>> = std::sync::OnceLock::new();
    S.get_or_init(|| std::sync::Mutex::new(Side::Red))
}

impl Guest for Fixture {
    fn info() -> Metadata {
        Metadata {
            title: "Fixture".into(),
            game_version: "0.0.0".into(),
            model: "Fixture".into(),
            code_agent: "Fixture".into(),
            model_platform: "official".into(),
            github_url: "https://github.com/xmiao76/grok_jungle_web".into(),
            engine_version: "0".into(),
            runtime: "Rust".into(),
            rules_id: "desktop-jungle-v2".into(),
            rules_digest: "a457bbc009e975a9eafe5ff1f8fa40ca2548f2a689b121c8c4ed99dd0f3fdc57".into(),
        }
    }

    fn init(config: Settings) {
        *seat().lock().unwrap_or_else(|p| p.into_inner()) = config.seat;
        let _ = config.difficulty;
        let _ = Level::Easy;
    }

    fn choose_move(_state: Snapshot, legal: Vec<PlyMove>, _node_budget: u32) -> u32 {
        if legal.is_empty() { 0 } else { 0 }
    }

    fn render(state: Snapshot, flipped: bool, _progress: u16) {
        let mut px = vec![0u8; 448 * 576 * 4];
        let seat_is_red = matches!(*seat().lock().unwrap_or_else(|p| p.into_inner()), Side::Red);
        let (br, bg, bb) = if seat_is_red { (90, 28, 28) } else { (24, 48, 90) };
        for i in (0..px.len()).step_by(4) {
            px[i] = br;
            px[i + 1] = bg;
            px[i + 2] = bb;
            px[i + 3] = 255;
        }
        let label = if flipped { "FIXTURE FLIP" } else { "FIXTURE" };
        let _ = (state, label);
        jungle::arena::display::present(448, 576, &px);
    }
}

export!(Fixture);
