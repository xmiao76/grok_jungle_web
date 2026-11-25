"""Session API tests against the native jungle-engine CLI."""

from __future__ import annotations

import json
import os
import subprocess
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[1]
BIN = ROOT / "engine" / "target" / "release" / (
    "jungle-engine.exe" if os.name == "nt" else "jungle-engine"
)


def _engine() -> subprocess.Popen:
    if not BIN.exists():
        pytest.skip(f"native engine not built: {BIN}")
    return subprocess.Popen(
        [str(BIN), "stdio"],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        text=True,
    )


def call(proc: subprocess.Popen, line: str) -> dict:
    assert proc.stdin and proc.stdout
    proc.stdin.write(line + "\n")
    proc.stdin.flush()
    raw = proc.stdout.readline()
    return json.loads(raw)


@pytest.fixture
def eng():
    proc = _engine()
    try:
        yield proc
    finally:
        if proc.stdin:
            try:
                proc.stdin.write("quit\n")
                proc.stdin.flush()
            except OSError:
                pass
        proc.kill()
        proc.wait(timeout=5)


def test_engine_info(eng) -> None:
    info = call(eng, "INFO")
    assert info["ok"]
    assert info["data"]["engineVersion"]
    assert "Rust" in info["data"]["runtime"] or "wasm" in info["data"]["runtime"]


def test_new_game_human_first_gold(eng) -> None:
    res = call(eng, "NEW human_ai 1 easy")
    assert res["ok"]
    state = res["data"]["state"]
    assert state["turn"] == "red"
    assert state["humanSide"] == "red"
    assert state["humanFirst"] is True
    assert state["ply"] == 0
    assert len(state["legalMoves"]) == 24
    assert state["fen"].endswith(" g")
    assert state["result"] == "ongoing"


def test_new_game_ai_first_scarlet_opens(eng) -> None:
    res = call(eng, "NEW human_ai 0 easy")
    state = res["data"]["state"]
    assert state["turn"] == "blue"
    assert state["humanSide"] == "red"
    assert state["humanFirst"] is False


def test_illegal_move_rejected(eng) -> None:
    call(eng, "NEW human_ai 1 easy")
    res = call(eng, "APPLY a1 d5")
    assert res["ok"] is False
    assert "illegal" in res["error"]


def test_legal_opening_and_undo(eng) -> None:
    call(eng, "NEW human_ai 1 easy")
    moved = call(eng, "APPLY g3 g4")
    assert moved["ok"]
    assert moved["data"]["state"]["ply"] == 1
    assert moved["data"]["state"]["turn"] == "blue"

    ai = call(eng, "AI 80")
    assert ai["ok"]
    assert ai["data"]["state"]["ply"] == 2
    assert ai["data"]["state"]["turn"] == "red"

    undone = call(eng, "UNDO")
    assert undone["ok"]
    assert undone["data"]["state"]["ply"] == 0
    assert undone["data"]["state"]["turn"] == "red"


def test_resign_human(eng) -> None:
    call(eng, "NEW human_ai 1 easy")
    res = call(eng, "RESIGN")
    state = res["data"]["state"]
    assert state["terminal"] is True
    assert state["result"] == "blue_wins"
    assert state["outcome"] == "resign"


def test_replay_opening(eng) -> None:
    call(eng, "NEW human_ai 1 easy")
    res = call(eng, "REPLAY [[\"g3\",\"g4\"],[\"a7\",\"a6\"]]")
    assert res["ok"]
    assert res["data"]["applied"] == 2
    assert res["data"]["state"]["ply"] == 2
    assert res["data"]["state"]["history"] == ["g3g4", "a7a6"]


def test_invalid_difficulty(eng) -> None:
    res = call(eng, "NEW human_ai 1 nope")
    assert res["ok"] is False


def test_undo_after_ai_opening_leaves_scarlet(eng) -> None:
    call(eng, "NEW human_ai 0 easy")
    ai = call(eng, "AI 80")
    assert ai["ok"]
    assert ai["data"]["state"]["ply"] == 1
    undone = call(eng, "UNDO")
    assert undone["ok"]
    assert undone["data"]["state"]["ply"] == 0
    assert undone["data"]["state"]["turn"] == "blue"


def test_captured_tray_after_take(eng) -> None:
    call(eng, "NEW human_ai 1 easy")
    state = call(eng, "STATE")["data"]["state"]
    assert state["captured"] == {"red": [], "blue": []}
