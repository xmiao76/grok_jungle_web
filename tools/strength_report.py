"""Two-game, colors-swapped matches at fixed node budgets.

The new engine is engine/target/release/jungle-engine (stdio NODES).
The previous engine is target/baseline/jungle-engine-nodes.exe when present,
otherwise target/baseline/jungle-engine-1.2.0.exe is not used: that binary
has no node-budget command. Build the nodes binary with
tools/build-baseline-nodes.mjs.

Score is from the new engine's point of view (win 1, draw 0.5, loss 0).
A two-game match cannot support a tight confidence interval; the report
prints the Wilson interval anyway so the README can quote it.
"""

from __future__ import annotations

import json
import math
import os
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
NEW = ROOT / "engine" / "target" / "release" / (
    "jungle-engine.exe" if os.name == "nt" else "jungle-engine"
)
OLD = ROOT / "target" / "baseline" / (
    "jungle-engine-nodes.exe" if os.name == "nt" else "jungle-engine-nodes"
)


class Engine:
    def __init__(self, path: Path, kind: str) -> None:
        if not path.exists():
            raise SystemExit(f"missing {path}")
        self.kind = kind
        self.proc = subprocess.Popen(
            [str(path), "stdio"],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            text=True,
        )

    def nodes(self, fen: str, history: list[str], budget: int) -> str:
        tail = " | ".join([fen, *history])
        line = f"NODES {budget} 1 1 {tail}\n"
        assert self.proc.stdin and self.proc.stdout
        self.proc.stdin.write(line)
        self.proc.stdin.flush()
        rec = json.loads(self.proc.stdout.readline())
        if not rec.get("ok"):
            raise RuntimeError(rec.get("error") or "search failed")
        return rec["data"]["uci"]

    def close(self) -> None:
        if self.proc.stdin:
            try:
                self.proc.stdin.write("quit\n")
                self.proc.stdin.flush()
            except OSError:
                pass
        self.proc.kill()
        self.proc.wait(timeout=5)


def wilson(wins: float, n: int) -> tuple[float, float]:
    if n == 0:
        return (0.0, 1.0)
    z = 1.96
    p = wins / n
    den = 1 + z * z / n
    centre = (p + z * z / (2 * n)) / den
    margin = z * math.sqrt((p * (1 - p) + z * z / (4 * n)) / n) / den
    return (max(0.0, centre - margin), min(1.0, centre + margin))


def play(new: Engine, old: Engine, budget: int, new_is_gold: bool) -> float:
    # Drive the rules with the new engine's session. Each side only searches.
    assert new.proc.stdin and new.proc.stdout
    new.proc.stdin.write("NEW ai_ai 1 medium\n")
    new.proc.stdin.flush()
    state = json.loads(new.proc.stdout.readline())["data"]["state"]
    history: list[str] = []
    guard = 0
    while not state["terminal"] and guard < 240:
        guard += 1
        fen = state["fen"]
        red_to_move = state["turn"] == "red"
        use_new = red_to_move == new_is_gold
        engine = new if use_new else old
        uci = engine.nodes(fen, history, budget)
        src, dst = uci[:2], uci[2:4]
        new.proc.stdin.write(f"APPLY {src} {dst}\n")
        new.proc.stdin.flush()
        rec = json.loads(new.proc.stdout.readline())
        if not rec.get("ok"):
            raise RuntimeError(rec.get("error") or f"illegal {uci}")
        state = rec["data"]["state"]
        history.append(fen)
    if not state["terminal"] or state["result"] == "draw":
        return 0.5
    new_won = (state["result"] == "red_wins") == new_is_gold
    return 1.0 if new_won else 0.0


def main() -> int:
    budgets = [int(a) for a in sys.argv[1:]] or [5000, 20000]
    new = Engine(NEW, "new")
    old = Engine(OLD, "old")
    try:
        lines = []
        for budget in budgets:
            scores = [
                play(new, old, budget, True),
                play(new, old, budget, False),
            ]
            total = sum(scores)
            lo, hi = wilson(total, 2)
            line = (
                f"{budget} nodes: new {total:.1f}/2 "
                f"(games {scores[0]:.1f} as Red, {scores[1]:.1f} as Blue); "
                f"Wilson 95% CI [{lo:.2f}, {hi:.2f}]"
            )
            print(line, flush=True)
            lines.append(line)
        out = ROOT / "release" / "strength.txt"
        out.write_text("\n".join(lines) + "\n", encoding="utf-8")
        print(f"wrote {out}")
    finally:
        new.close()
        old.close()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
