from __future__ import annotations

import random
import time
from dataclasses import dataclass

from jungle.ai.eval import MATE, evaluate
from jungle.ai.opening_book import lookup as book_lookup
from jungle.ai.see import see_capture
from jungle.engine.apply import apply_move, inspect_terminal
from jungle.engine.board import enemy_den_index
from jungle.engine.movegen import legal_moves
from jungle.engine.position import Position
from jungle.engine.types import Move, PieceType, Result, Side
from jungle.engine.zobrist import side_key

EXACT, LOWER, UPPER = 0, 1, 2
MAX_PLY = 64


@dataclass(frozen=True)
class Limits:
    move_time_ms: int
    max_depth: int = 8
    noise: float = 0.0
    hard_cap_ms: int | None = None

    @property
    def hard_ms(self) -> int:
        return self.hard_cap_ms if self.hard_cap_ms is not None else int(self.move_time_ms * 2.2)


def difficulty_limits(name: str) -> Limits:
    table = {
        "easy": Limits(70, max_depth=2, noise=0.30),
        "normal": Limits(750, max_depth=12, noise=0.0),
        "strong": Limits(2400, max_depth=16, noise=0.0),
    }
    return table.get(name, table["normal"])


class _Timeout(Exception):
    pass


@dataclass
class _TTEntry:
    depth: int
    flag: int
    score: int
    move: Move | None


def _is_den_entry(pos: Position, move: Move) -> bool:
    piece = pos.board[move.src.index()]
    if piece is None:
        return False
    return move.dst.index() == enemy_den_index(piece.side)


def _den_adjacent(pos: Position, move: Move) -> bool:
    piece = pos.board[move.src.index()]
    if piece is None:
        return False
    den = enemy_den_index(piece.side)
    df = abs(move.dst.file - den % 7)
    dr = abs(move.dst.rank - den // 7)
    return df + dr == 1


def _mvv_lva(pos: Position, move: Move) -> int:
    victim = pos.board[move.dst.index()]
    attacker = pos.board[move.src.index()]
    if victim is None or attacker is None:
        return 0
    return int(victim.ptype) * 16 - int(attacker.ptype) + (50 if move.is_jump else 0)


class Searcher:
    def __init__(self, use_see: bool = True, use_book: bool = True) -> None:
        self.use_see = use_see
        self.use_book = use_book
        self.tt: dict[int, _TTEntry] = {}
        self.killers: list[list[Move | None]] = [[None, None] for _ in range(MAX_PLY)]
        self.history: list[list[list[int]]] = [
            [[0] * 63 for _ in range(63)] for _ in range(2)
        ]
        self.deadline = 0.0
        self.nodes = 0
        self.best: Move | None = None

    def clear_if_huge(self) -> None:
        if len(self.tt) > 400_000:
            self.tt.clear()

    def search(self, pos: Position, limits: Limits, history: list[int] | None = None) -> Move:
        self.clear_if_huge()
        self.deadline = time.perf_counter() + limits.hard_ms / 1000.0
        soft = time.perf_counter() + limits.move_time_ms / 1000.0
        self.nodes = 0
        hashes = list(history or [pos.hash])
        moves = legal_moves(pos)
        if not moves:
            raise ValueError("no legal moves")
        # Immediate den win — no search required.
        for move in moves:
            if _is_den_entry(pos, move):
                self.best = move
                return move
        if self.use_book and limits.noise == 0:
            booked = book_lookup(pos)
            if booked is not None and any(
                m.src == booked.src and m.dst == booked.dst for m in moves
            ):
                self.best = booked
                return booked
        self.best = moves[0]
        prev = 0
        extra = 1 if pos.counts[0] + pos.counts[1] <= 6 else 0
        try:
            for depth in range(1, limits.max_depth + 1 + extra):
                window = 40 if depth >= 3 else MATE
                alpha, beta = prev - window, prev + window
                score = self._root(pos, depth, alpha, beta, hashes, moves, limits)
                if score <= alpha or score >= beta:
                    score = self._root(pos, depth, -MATE, MATE, hashes, moves, limits)
                prev = score
                if time.perf_counter() >= soft:
                    break
                if score >= MATE - 64:
                    break
        except _Timeout:
            pass
        assert self.best is not None
        return self.best

    def _root(
        self,
        pos: Position,
        depth: int,
        alpha: int,
        beta: int,
        hashes: list[int],
        moves: list[Move],
        limits: Limits,
    ) -> int:
        ordered = self._order(pos, moves, self.best, 0)
        best_score = -MATE
        scored: list[tuple[int, Move]] = []
        for move in ordered:
            child = apply_move(pos, move)
            hashes.append(child.hash)
            score = -self._alphabeta(child, depth - 1, -beta, -alpha, hashes, 1)
            hashes.pop()
            scored.append((score, move))
            if score > best_score:
                best_score = score
                self.best = move
            if score > alpha:
                alpha = score
        if scored:
            scored.sort(key=lambda t: t[0], reverse=True)
            self.best = scored[0][1]
            if limits.noise > 0 and len(scored) > 1:
                top = [m for s, m in scored if s >= scored[0][0] - 50][:3]
                if random.random() < limits.noise:
                    self.best = random.choice(top)
        return best_score

    def _order(self, pos: Position, moves: list[Move], tt_move: Move | None, ply: int) -> list[Move]:
        killers = self.killers[ply] if ply < MAX_PLY else [None, None]

        def key(m: Move) -> tuple:
            is_hash = tt_move is not None and m.src == tt_move.src and m.dst == tt_move.dst
            den = 0 if _is_den_entry(pos, m) else 1
            cap = _mvv_lva(pos, m)
            see = 0
            if self.use_see and pos.board[m.dst.index()] is not None:
                see = see_capture(pos, m)
            is_killer = 0 if (killers[0] and m == killers[0]) or (killers[1] and m == killers[1]) else 1
            hist = -self.history[pos.to_move][m.src.index()][m.dst.index()]
            return (0 if is_hash else 1, den, -see, -cap, is_killer, hist)

        return sorted(moves, key=key)

    def _alphabeta(
        self,
        pos: Position,
        depth: int,
        alpha: int,
        beta: int,
        hashes: list[int],
        ply: int,
    ) -> int:
        self.nodes += 1
        if (self.nodes & 127) == 0 and time.perf_counter() >= self.deadline:
            raise _Timeout()

        result, _ = inspect_terminal(pos, hashes)
        if result is Result.DRAW:
            return 0
        if result is Result.GOLD_WINS:
            score = MATE - ply
            return score if pos.to_move is Side.GOLD else -score
        if result is Result.SCARLET_WINS:
            score = MATE - ply
            return score if pos.to_move is Side.SCARLET else -score

        tt_move = None
        hit = self.tt.get(pos.hash)
        if hit is not None:
            tt_move = hit.move
            if hit.depth >= depth:
                if hit.flag == EXACT:
                    return hit.score
                if hit.flag == LOWER:
                    alpha = max(alpha, hit.score)
                elif hit.flag == UPPER:
                    beta = min(beta, hit.score)
                if alpha >= beta:
                    return hit.score

        if depth <= 0:
            return self._quiesce(pos, alpha, beta, hashes, ply)

        stand = evaluate(pos)
        if depth <= 2 and stand - 150 * depth >= beta and beta < MATE - 400:
            return stand
        if (
            depth >= 3
            and ply > 0
            and beta < MATE - 400
            and pos.counts[0] >= 4
            and pos.counts[1] >= 4
            and stand >= beta
        ):
            child = pos.copy()
            child.to_move = child.to_move.other()
            child.hash ^= side_key()
            hashes.append(child.hash)
            score = -self._alphabeta(child, depth - 3, -beta, -beta + 1, hashes, ply + 1)
            hashes.pop()
            if score >= beta:
                return score

        moves = legal_moves(pos)
        if not moves:
            return -MATE + ply

        orig_alpha = alpha
        best_move: Move | None = None
        best = -MATE
        ordered = self._order(pos, moves, tt_move, ply)
        for i, move in enumerate(ordered):
            child = apply_move(pos, move)
            hashes.append(child.hash)
            ext = 1 if _den_adjacent(pos, move) and depth < 12 else 0
            reduction = 0
            is_cap = pos.board[move.dst.index()] is not None
            if depth >= 3 and i >= 3 and not is_cap and ext == 0 and not _is_den_entry(pos, move):
                reduction = 1
            new_depth = depth - 1 + ext - reduction
            if i == 0:
                score = -self._alphabeta(child, new_depth, -beta, -alpha, hashes, ply + 1)
            else:
                score = -self._alphabeta(child, new_depth, -alpha - 1, -alpha, hashes, ply + 1)
                if alpha < score < beta:
                    score = -self._alphabeta(child, depth - 1 + ext, -beta, -alpha, hashes, ply + 1)
            if reduction and score > alpha:
                score = -self._alphabeta(child, depth - 1 + ext, -beta, -alpha, hashes, ply + 1)
            hashes.pop()
            if score > best:
                best = score
                best_move = move
            if score > alpha:
                alpha = score
            if alpha >= beta:
                if not is_cap and ply < MAX_PLY:
                    if self.killers[ply][0] != move:
                        self.killers[ply][1] = self.killers[ply][0]
                        self.killers[ply][0] = move
                    self.history[pos.to_move][move.src.index()][move.dst.index()] += depth * depth
                break
        flag = EXACT
        if best <= orig_alpha:
            flag = UPPER
        elif best >= beta:
            flag = LOWER
        self.tt[pos.hash] = _TTEntry(depth, flag, best, best_move)
        return best

    def _quiesce(self, pos: Position, alpha: int, beta: int, hashes: list[int], ply: int) -> int:
        if ply > 28:
            return evaluate(pos)
        result, _ = inspect_terminal(pos, hashes)
        if result is Result.DRAW:
            return 0
        if result is Result.GOLD_WINS:
            score = MATE - ply
            return score if pos.to_move is Side.GOLD else -score
        if result is Result.SCARLET_WINS:
            score = MATE - ply
            return score if pos.to_move is Side.SCARLET else -score
        stand = evaluate(pos)
        if stand >= beta:
            return stand
        if stand > alpha:
            alpha = stand
        captures = []
        for m in legal_moves(pos):
            if _is_den_entry(pos, m):
                captures.append(m)
                continue
            if pos.board[m.dst.index()] is None:
                continue
            if self.use_see and see_capture(pos, m) < 0:
                continue
            captures.append(m)
        for move in self._order(pos, captures, None, min(ply, MAX_PLY - 1)):
            child = apply_move(pos, move)
            hashes.append(child.hash)
            score = -self._quiesce(child, -beta, -alpha, hashes, ply + 1)
            hashes.pop()
            if score >= beta:
                return score
            if score > alpha:
                alpha = score
        return alpha


_ENGINE = Searcher()


def best_move(pos: Position, limits: Limits | None = None, history: list[int] | None = None) -> Move:
    limits = limits or difficulty_limits("normal")
    return _ENGINE.search(pos, limits, history)


_ = PieceType
