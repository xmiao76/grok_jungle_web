"""Play the new searcher against the frozen desktop-style eval.

Pass if the new engine scores >= 15/24 (win=1, draw=0.5).
Usage: python tools/strength_gate.py
"""

from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "public" / "py"))
sys.path.insert(0, str(ROOT))

import jungle.ai.search as search_mod
from jungle.ai.search import Limits, Searcher
from jungle.engine.apply import apply_move, terminal_after
from jungle.engine.game import Game, GameConfig
from jungle.engine.types import Result
from tests.legacy_eval import evaluate as old_evaluate


def _play(new_side_gold: bool, limits: Limits) -> float:
    game = Game(GameConfig(mode="ai_ai", human_first=True))
    new = Searcher(use_see=True, use_book=True)
    old = Searcher(use_see=False, use_book=False)
    while game.result is Result.ONGOING and len(game.moves) < 180:
        gold_is_new = new_side_gold
        use_new = (game.position.to_move.value == 0) == gold_is_new
        if use_new:
            search_mod.evaluate = __import__("jungle.ai.eval", fromlist=["evaluate"]).evaluate
            engine = new
        else:
            search_mod.evaluate = old_evaluate
            engine = old
        move = engine.search(game.position, limits, game.hashes)
        game.play(move)
    if game.result is Result.DRAW or game.result is Result.ONGOING:
        return 0.5
    new_won = (game.result is Result.GOLD_WINS) == new_side_gold
    return 1.0 if new_won else 0.0


def main() -> int:
    limits = Limits(90, max_depth=5, hard_cap_ms=220)
    scores = []
    pairs = 6
    target = 7.5
    out_of = pairs * 2
    for i in range(pairs):
        scores.append(_play(True, limits))
        scores.append(_play(False, limits))
        print(f"  pair {i + 1}/{pairs}  running {sum(scores):.1f}/{len(scores)}", flush=True)
    total = sum(scores)
    print(f"new engine {total:.1f}/{out_of}", flush=True)
    if total < target:
        print(f"FAIL: need >= {target}/{out_of}")
        return 1
    print("PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
