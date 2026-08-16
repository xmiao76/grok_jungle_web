"""Python oracle vs native legal-move compare."""

from __future__ import annotations

import json
import os
import random
import subprocess
from pathlib import Path

import pytest

from jungle.engine.apply import apply_move
from jungle.engine.movegen import legal_moves
from jungle.engine.position import dump_fen, start_position


ROOT = Path(__file__).resolve().parents[1]
BIN = ROOT / "engine" / "target" / "release" / (
    "jungle-engine.exe" if os.name == "nt" else "jungle-engine"
)


def _legal_native(fen: str) -> set[str]:
    if not BIN.exists():
        pytest.skip(f"native engine not built: {BIN}")
    out = subprocess.check_output([str(BIN), "legal", fen], text=True)
    return set(out.split())


def test_native_legal_matches_python_start() -> None:
    fen = dump_fen(start_position())
    py = {m.uci() for m in legal_moves(start_position())}
    assert py == _legal_native(fen)


def test_native_legal_matches_python_random() -> None:
    rng = random.Random(2026)
    mismatches = 0
    checked = 0
    for _ in range(200):
        pos = start_position()
        for _ply in range(rng.randint(0, 40)):
            ms = legal_moves(pos)
            if not ms:
                break
            pos = apply_move(pos, rng.choice(ms))
        fen = dump_fen(pos)
        py = {m.uci() for m in legal_moves(pos)}
        nat = _legal_native(fen)
        checked += 1
        if py != nat:
            mismatches += 1
            pytest.fail(f"legal mismatch at {fen}: py-nat={py - nat} nat-py={nat - py}")
    assert checked == 200
    assert mismatches == 0


def test_native_takes_open_den() -> None:
    if not BIN.exists():
        pytest.skip(f"native engine not built: {BIN}")
    proc = subprocess.Popen(
        [str(BIN), "stdio"],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        text=True,
    )
    try:
        assert proc.stdin and proc.stdout
        proc.stdin.write("SEARCH 80 6 0 1 c6/3D3/7/7/7/7/7/7/7 g\n")
        proc.stdin.flush()
        rec = json.loads(proc.stdout.readline())
        assert rec["ok"]
        assert rec["data"]["uci"].startswith("d8d9")
    finally:
        proc.kill()
        proc.wait(timeout=5)
