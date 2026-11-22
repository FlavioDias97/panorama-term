import { expect, test } from 'bun:test';

import { TILE_SIDE_GAP, tileSideOrigin } from '~/usecase/util/tileSide';

test('panel sits one gap past the right edge of the tile', () => {
  expect(tileSideOrigin(100, 50, 400, 1)).toEqual({ left: 100 + 400 + TILE_SIDE_GAP, top: 50 });
});

test('gap and width scale with the canvas zoom', () => {
  expect(tileSideOrigin(10, 20, 400, 0.5)).toEqual({ left: 10 + (400 + TILE_SIDE_GAP) / 2, top: 20 });
});
