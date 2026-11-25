// Algebraic squares (a1 = bottom-left, Red's side) <-> wire squares (0 = top-left).

export function algebraicToWire(name) {
  const file = name.charCodeAt(0) - 97;
  const rank = Number(name.slice(1)) - 1;
  return (8 - rank) * 7 + file;
}

export function wireToAlgebraic(wire) {
  const row = Math.floor(wire / 7);
  const file = wire % 7;
  const rank = 8 - row;
  return String.fromCharCode(97 + file) + String(rank + 1);
}

export function legalWire(state) {
  const moves = (state.legalMoves || []).map((m) => ({
    src: algebraicToWire(m.src),
    dst: algebraicToWire(m.dst),
    algSrc: m.src,
    algDst: m.dst,
  }));
  moves.sort((a, b) => a.src - b.src || a.dst - b.dst);
  return moves;
}

export function snapshotFromState(state, history, firstSide) {
  const board = Array.from({ length: 63 }, () => null);
  for (const piece of state.board || []) {
    board[algebraicToWire(piece.sq)] = { owner: piece.side, kind: piece.type };
  }
  let status = 'playing';
  let winner;
  if (state.terminal) {
    if (state.result === 'draw') status = 'drawn';
    else if (state.result === 'red_wins') {
      status = 'won';
      winner = 'red';
    } else if (state.result === 'blue_wins') {
      status = 'won';
      winner = 'blue';
    }
  }
  const reason = state.outcome || '';
  return {
    rev: state.ply,
    board,
    firstSide,
    toPlay: state.turn,
    history: history.map(([src, dst]) => ({ src, dst })),
    noCapturePlies: state.noCapturePlies || 0,
    status,
    winner,
    reason,
  };
}
