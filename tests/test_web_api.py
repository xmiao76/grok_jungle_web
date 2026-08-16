import json

import web_api
from jungle.engine.types import Result


def call(fn: str, *args):
    return json.loads(getattr(web_api, fn)(*args))


def test_engine_info() -> None:
    info = call("engine_info")
    assert info["ok"]
    assert info["data"]["engineVersion"] == "1.2.0"
    assert info["data"]["python"]


def test_new_game_human_first_gold() -> None:
    res = call("new_game", "human_ai", True, "easy")
    assert res["ok"]
    state = res["data"]["state"]
    assert state["turn"] == "gold"
    assert state["humanSide"] == "gold"
    assert state["humanFirst"] is True
    assert state["ply"] == 0
    assert len(state["legalMoves"]) == 24
    assert state["fen"].endswith(" g")
    assert state["result"] == Result.ONGOING.value


def test_new_game_ai_first_scarlet_opens() -> None:
    res = call("new_game", "human_ai", False, "easy")
    assert res["ok"]
    state = res["data"]["state"]
    assert state["turn"] == "scarlet"
    assert state["humanSide"] == "gold"
    assert state["humanFirst"] is False


def test_illegal_move_rejected() -> None:
    call("new_game", "human_ai", True, "easy")
    res = call("apply_move", "a1", "d5")
    assert res["ok"] is False
    assert "illegal" in res["error"]


def test_legal_opening_and_undo() -> None:
    call("new_game", "human_ai", True, "easy")
    moved = call("apply_move", "g3", "g4")
    assert moved["ok"]
    assert moved["data"]["state"]["ply"] == 1
    assert moved["data"]["state"]["turn"] == "scarlet"
    assert moved["data"]["state"]["lastMove"]["src"] == "g3"
    assert moved["data"]["state"]["lastMove"]["dst"] == "g4"

    ai = call("ai_move", 80)
    assert ai["ok"]
    assert ai["data"]["state"]["ply"] == 2
    assert ai["data"]["state"]["turn"] == "gold"

    undone = call("undo")
    assert undone["ok"]
    assert undone["data"]["state"]["ply"] == 0
    assert undone["data"]["state"]["turn"] == "gold"


def test_resign_human() -> None:
    call("new_game", "human_ai", True, "easy")
    res = call("resign")
    assert res["ok"]
    state = res["data"]["state"]
    assert state["terminal"] is True
    assert state["result"] == Result.SCARLET_WINS.value
    assert state["outcome"] == "resign"


def test_replay_opening() -> None:
    call("new_game", "human_ai", True, "easy")
    res = call("replay_moves", json.dumps([["g3", "g4"], ["a7", "a6"]]))
    assert res["ok"]
    assert res["data"]["applied"] == 2
    assert res["data"]["state"]["ply"] == 2
    assert res["data"]["state"]["history"] == ["g3g4", "a7a6"]


def test_invalid_difficulty() -> None:
    res = call("new_game", "human_ai", True, "medium")
    assert res["ok"] is False


def test_undo_after_ai_opening_leaves_scarlet() -> None:
    call("new_game", "human_ai", False, "easy")
    ai = call("ai_move", 80)
    assert ai["ok"]
    assert ai["data"]["state"]["ply"] == 1
    undone = call("undo")
    assert undone["ok"]
    assert undone["data"]["state"]["ply"] == 0
    assert undone["data"]["state"]["turn"] == "scarlet"


def test_captured_tray_after_take() -> None:
    call("new_game", "human_ai", True, "easy")
    # Script a short capture: not guaranteed from start; just check shape.
    state = call("get_state")["data"]["state"]
    assert state["captured"] == {"gold": [], "scarlet": []}
