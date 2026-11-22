// Gap, in canvas units, between a tile and the panel docked beside it.
export const TILE_SIDE_GAP = 24;

// Screen position of the panel docked to the right of a tile drawn at (sx, sy) with width bodyW at zoom k.
export const tileSideOrigin = (sx: number, sy: number, bodyW: number, k: number) => ({
  left: sx + (bodyW + TILE_SIDE_GAP) * k,
  top: sy
});
