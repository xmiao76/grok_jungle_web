use jungle_engine::pos::Pos;
use jungle_engine::search::{best_move, Limits};

fn parse_epd(line: &str) -> Option<(String, Vec<String>, Vec<String>, String)> {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
        return None;
    }
    let mut fen = String::new();
    let mut bm = Vec::new();
    let mut am = Vec::new();
    let mut id = String::new();
    for part in line.split(';') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        if part.starts_with("bm ") {
            bm.extend(part[3..].split_whitespace().map(|s| s.to_string()));
        } else if part.starts_with("am ") {
            am.extend(part[3..].split_whitespace().map(|s| s.to_string()));
        } else if part.starts_with("id ") {
            id = part[3..].trim().trim_matches('"').to_string();
        } else if fen.is_empty() {
            fen = part.to_string();
        }
    }
    if fen.is_empty() || (bm.is_empty() && am.is_empty()) {
        return None;
    }
    Some((fen, bm, am, id))
}

#[test]
fn tactics_suite() {
    let epd = include_str!("tactics.epd");
    let mut failed = Vec::new();
    let mut n = 0;
    for line in epd.lines() {
        let Some((fen, bm, am, id)) = parse_epd(line) else {
            continue;
        };
        n += 1;
        let mut pos = Pos::parse_fen(&fen).unwrap_or_else(|e| panic!("{id}: {e}"));
        let h = pos.hash;
        let rec = best_move(&mut pos, Limits::timed(150, 10), &[h]);
        let uci = rec.mv.uci();
        if !bm.is_empty() && !bm.iter().any(|b| b == &uci || uci.starts_with(b)) {
            failed.push(format!("{id}: got {uci}, want {bm:?}"));
        }
        if am.iter().any(|a| a == &uci || uci.starts_with(a)) {
            failed.push(format!("{id}: played avoided {uci}"));
        }
    }
    assert!(n >= 8, "parsed {n} tactics");
    assert!(failed.is_empty(), "failed tactics:\n{}", failed.join("\n"));
}
