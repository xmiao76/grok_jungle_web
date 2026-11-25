use crate::session::Session;
use std::sync::{Mutex, OnceLock};
use wasm_bindgen::prelude::*;

fn session() -> &'static Mutex<Session> {
    static S: OnceLock<Mutex<Session>> = OnceLock::new();
    S.get_or_init(|| Mutex::new(Session::new()))
}

fn lock() -> std::sync::MutexGuard<'static, Session> {
    session().lock().unwrap_or_else(|p| p.into_inner())
}

#[wasm_bindgen]
pub fn init_engine(tt_mb: u32) {
    lock().set_tt_mb(tt_mb.max(1) as usize);
}

#[wasm_bindgen]
pub fn engine_info() -> String {
    Session::engine_info()
}

#[wasm_bindgen]
pub fn new_game(mode: &str, human_first: bool, difficulty: &str) -> String {
    lock().new_game(mode, human_first, difficulty)
}

#[wasm_bindgen]
pub fn get_state() -> String {
    lock().get_state()
}

#[wasm_bindgen]
pub fn apply_move(src: &str, dst: &str) -> String {
    lock().apply_move(src, dst)
}

#[wasm_bindgen]
pub fn ai_move(time_budget_ms: i32) -> String {
    let budget = if time_budget_ms > 0 {
        Some(time_budget_ms as u64)
    } else {
        None
    };
    lock().ai_move(budget)
}

#[wasm_bindgen]
pub fn undo() -> String {
    lock().undo()
}

#[wasm_bindgen]
pub fn resign() -> String {
    lock().resign()
}

#[wasm_bindgen]
pub fn replay_moves(moves_json: &str) -> String {
    lock().replay_moves(moves_json)
}
