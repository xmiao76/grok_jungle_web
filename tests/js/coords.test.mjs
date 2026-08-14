import { describe, it } from 'node:test';
import assert from 'node:assert/strict';
import {
  FILES, RANKS, parseSquare, squareName, displayCell,
  engineFromDisplay, boardToPixel, pixelToBoard,
} from '../../public/js/coords.js';

describe('coords', () => {
  it('parses and names algebraic squares', () => {
    assert.deepEqual(parseSquare('a1'), { file: 0, rank: 0 });
    assert.deepEqual(parseSquare('g9'), { file: 6, rank: 8 });
    assert.deepEqual(parseSquare('d5'), { file: 3, rank: 4 });
    assert.equal(squareName(6, 2), 'g3');
    assert.equal(parseSquare('z9'), null);
  });

  it('default view puts rank 1 at the bottom', () => {
    const a1 = displayCell(0, 0, false);
    assert.deepEqual(a1, { col: 0, row: RANKS - 1 });
    const g3 = displayCell(6, 2, false);
    assert.deepEqual(g3, { col: 6, row: RANKS - 1 - 2 });
    assert.ok(g3.row > RANKS / 2);
  });

  it('flipped view is a 180° mapping', () => {
    assert.deepEqual(displayCell(0, 0, true), { col: FILES - 1, row: 0 });
    assert.deepEqual(displayCell(6, 8, true), { col: 0, row: 8 });
  });

  it('pixel mapping inverts through flip', () => {
    const cs = 40;
    for (const flipped of [false, true]) {
      for (let file = 0; file < FILES; file++) {
        for (let rank = 0; rank < RANKS; rank++) {
          const [px, py] = boardToPixel(file, rank, cs, flipped);
          const back = pixelToBoard(px + 1, py + 1, cs, flipped);
          assert.deepEqual(back, { file, rank }, `${file},${rank} flip=${flipped}`);
        }
      }
    }
  });

  it('engineFromDisplay rejects off-board cells', () => {
    assert.equal(engineFromDisplay(-1, 0, false), null);
    assert.equal(engineFromDisplay(0, 9, false), null);
    assert.equal(engineFromDisplay(7, 0, true), null);
  });
});
