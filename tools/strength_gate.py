"""Play the native engine against the frozen Python searcher.

Pass if native scores >= 9/12 (win=1, draw=0.5).
Usage: python tools/strength_gate.py
Requires: cargo build --release -p jungle-engine  (engine/target/release/jungle-engine)
"""

from __future__ import annotations

import json
import os
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "public" / "py"))
sys.path.insert(0, str(ROOT))

from jungle.ai.search import Limits, Searcher
from jungle.engine.game import Game, GameConfig
from jungle.engine.position import dump_fen
from jungle.engine.types import Move, Result

BIN = ROOT / "engine" / "target" / "release" / (
    "jungle-engine.exe" if os.name == "nt" else "jungle-engine"
)


class Native:
    def __init__(self) -> None:
        if not BIN.exists():
            raise SystemExit(f"native engine not built: {BIN}")
        self.proc = subprocess.Popen(
            [str(BIN), "stdio"],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            text=True,
        )

    def search(self, game: Game, ms: int, depth: int) -> Move:
        fens = [dump_fen(p) for p in game.history]
        # current fen first, then prior positions for repetition hashes
        current = fens[-1]
        hist = " | ".join(fens[:-1])
        tail = current if not hist else f"{current} | {hist}"
        line = f"SEARCH {ms} {depth} 1 1 {tail}\n"
        assert self.proc.stdin and self.proc.stdout
        self.proc.stdin.write(line)
        self.proc.stdin.flush()
        rec = json.loads(self.proc.stdout.readline())
        if not rec.get("ok"):
            raise RuntimeError(rec.get("error") or "native search failed")
        return Move.parse(rec["data"]["uci"])

    def close(self) -> None:
        if self.proc.stdin:
            try:
                self.proc.stdin.write("quit\n")
                self.proc.stdin.flush()
            except OSError:
                pass
        self.proc.kill()
        self.proc.wait(timeout=5)


def _play(native: Native, new_side_gold: bool, py_limits: Limits) -> float:
    game = Game(GameConfig(mode="ai_ai", human_first=True))
    old = Searcher(use_see=False, use_book=False)
    while game.result is Result.ONGOING and len(game.moves) < 160:
        gold_is_new = new_side_gold
        use_new = (game.position.to_move.value == 0) == gold_is_new
        if use_new:
            move = native.search(game, 90, 12)
        else:
            move = old.search(game.position, py_limits, game.hashes)
        game.play(move)
    if game.result is Result.DRAW or game.result is Result.ONGOING:
        return 0.5
    new_won = (game.result is Result.GOLD_WINS) == new_side_gold
    return 1.0 if new_won else 0.0


def main() -> int:
    native = Native()
    try:
        py_limits = Limits(90, max_depth=5, hard_cap_ms=220)
        scores = []
        pairs = 6
        target = 9.0
        out_of = pairs * 2
        for i in range(pairs):
            scores.append(_play(native, True, py_limits))
            scores.append(_play(native, False, py_limits))
            print(f"  pair {i + 1}/{pairs}  running {sum(scores):.1f}/{len(scores)}", flush=True)
        total = sum(scores)
        print(f"native engine {total:.1f}/{out_of}", flush=True)
        if total < target:
            print(f"FAIL: need >= {target}/{out_of}")
            return 1
        print("PASS")
        return 0
    finally:
        native.close()


if __name__ == "__main__":
    raise SystemExit(main())
