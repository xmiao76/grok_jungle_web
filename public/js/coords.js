// Pure board/pixel mapping. Engine file 0–6 (a–g), rank 0–8 (1–9),
// rank 0 = south (Gold). Flip is a 180° view transform only.

export const FILES = 7;
export const RANKS = 9;

export function parseSquare(name) {
  const n = String(name).trim().toLowerCase();
  if (n.length < 2) return null;
  const file = n.charCodeAt(0) - 97;
  const rank = parseInt(n.slice(1), 10) - 1;
  if (file < 0 || file >= FILES || rank < 0 || rank >= RANKS || Number.isNaN(rank)) {
    return null;
  }
  return { file, rank };
}

export function squareName(file, rank) {
  return String.fromCharCode(97 + file) + String(rank + 1);
}

// Display cell: col 0 left, row 0 top of the board grid.
export function displayCell(file, rank, flipped) {
  if (flipped) return { col: FILES - 1 - file, row: rank };
  return { col: file, row: RANKS - 1 - rank };
}

export function engineFromDisplay(col, row, flipped) {
  if (col < 0 || row < 0 || col >= FILES || row >= RANKS) return null;
  if (flipped) return { file: FILES - 1 - col, rank: row };
  return { file: col, rank: RANKS - 1 - row };
}

export function boardToPixel(file, rank, cellSize, flipped) {
  const { col, row } = displayCell(file, rank, flipped);
  return [col * cellSize, row * cellSize];
}

export function pixelToBoard(px, py, cellSize, flipped) {
  const col = Math.floor(px / cellSize);
  const row = Math.floor(py / cellSize);
  return engineFromDisplay(col, row, flipped);
}
