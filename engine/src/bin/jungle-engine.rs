use jungle_engine::movegen::legal_uci;
use jungle_engine::pos::Pos;
use jungle_engine::session::Session;
use serde_json::json;
use std::env;
use std::io::{self, BufRead, Write};

fn main() {
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        Some("perft") => {
            let depth: u32 = args.next().and_then(|s| s.parse().ok()).unwrap_or(4);
            let fen = args.next().unwrap_or_else(|| Pos::start().fen());
            match jungle_engine::perft_fen(&fen, depth) {
                Ok(n) => println!("{n}"),
                Err(e) => {
                    eprintln!("{e}");
                    std::process::exit(1);
                }
            }
        }
        Some("search") => {
            let fen = args.next().unwrap_or_else(|| Pos::start().fen());
            let ms: u64 = args.next().and_then(|s| s.parse().ok()).unwrap_or(200);
            let depth: i32 = args.next().and_then(|s| s.parse().ok()).unwrap_or(12);
            let book = args.next().as_deref() == Some("1");
            match jungle_engine::search_fen(
                &fen,
                &[],
                ms,
                depth,
                (ms as f64 * 2.2) as u64,
                0.0,
                0,
                book,
                true,
            ) {
                Ok((uci, nodes, d)) => println!("{uci} nodes={nodes} depth={d}"),
                Err(e) => {
                    eprintln!("{e}");
                    std::process::exit(1);
                }
            }
        }
        Some("gen-tb") => {
            let path = args
                .next()
                .unwrap_or_else(|| "src/tb2.bin".to_string());
            match jungle_engine::tb::write_bin(&path) {
                Ok(()) => println!("wrote {path}"),
                Err(e) => {
                    eprintln!("{e}");
                    std::process::exit(1);
                }
            }
        }
        Some("legal") => {
            let fen = args.next().unwrap_or_else(|| Pos::start().fen());
            match Pos::parse_fen(&fen) {
                Ok(pos) => {
                    let mut ms = legal_uci(&pos);
                    ms.sort();
                    println!("{}", ms.join(" "));
                }
                Err(e) => {
                    eprintln!("{e}");
                    std::process::exit(1);
                }
            }
        }
        Some("stdio") | None => stdio(),
        Some(other) => {
            eprintln!("unknown command {other}");
            std::process::exit(2);
        }
    }
}

fn stdio() {
    let mut session = Session::new();
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if line.eq_ignore_ascii_case("quit") {
            break;
        }
        let reply = handle(&mut session, line);
        let _ = writeln!(stdout, "{reply}");
        let _ = stdout.flush();
    }
}

fn handle(session: &mut Session, line: &str) -> String {
    let mut parts = line.splitn(2, ' ');
    let cmd = parts.next().unwrap_or("");
    let rest = parts.next().unwrap_or("").trim();
    match cmd {
        "INFO" => Session::engine_info(),
        "NEW" => {
            // NEW <mode> <human_first 0|1> <difficulty>
            let mut it = rest.split_whitespace();
            let mode = it.next().unwrap_or("human_ai");
            let hf = it.next().unwrap_or("1") != "0";
            let diff = it.next().unwrap_or("normal");
            session.new_game(mode, hf, diff)
        }
        "APPLY" => {
            let mut it = rest.split_whitespace();
            let src = it.next().unwrap_or("");
            let dst = it.next().unwrap_or("");
            session.apply_move(src, dst)
        }
        "AI" => {
            let ms = rest.parse::<u64>().ok().filter(|&n| n > 0);
            session.ai_move(ms)
        }
        "UNDO" => session.undo(),
        "RESIGN" => session.resign(),
        "STATE" => session.get_state(),
        "REPLAY" => session.replay_moves(rest),
        "SEARCH" => {
            // SEARCH <ms> <depth> <book 0|1> <tb 0|1> <fen>[ | <histfen>...]
            let mut it = rest.splitn(5, ' ');
            let ms: u64 = it.next().and_then(|s| s.parse().ok()).unwrap_or(90);
            let depth: i32 = it.next().and_then(|s| s.parse().ok()).unwrap_or(12);
            let book = it.next() == Some("1");
            let tb = it.next() != Some("0");
            let tail = it.next().unwrap_or("");
            let mut fens: Vec<String> = tail
                .split(" | ")
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            let fen = if fens.is_empty() {
                String::new()
            } else {
                fens.remove(0)
            };
            match session.search_raw(&fen, &fens, ms, depth, book, tb) {
                Ok((uci, nodes, d)) => json!({
                    "ok": true,
                    "data": { "uci": uci, "nodes": nodes, "depth": d },
                    "error": null
                })
                .to_string(),
                Err(e) => json!({"ok": false, "data": null, "error": e}).to_string(),
            }
        }
        "LEGAL" => match Pos::parse_fen(rest) {
            Ok(pos) => {
                let mut ms = legal_uci(&pos);
                ms.sort();
                json!({"ok": true, "data": {"moves": ms}, "error": null}).to_string()
            }
            Err(e) => json!({"ok": false, "data": null, "error": e}).to_string(),
        },
        other => json!({
            "ok": false,
            "data": null,
            "error": format!("unknown command {other}")
        })
        .to_string(),
    }
}
