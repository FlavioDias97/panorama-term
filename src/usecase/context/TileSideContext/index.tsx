import React from 'react';

// Element beside a canvas tile where its content can dock panels (null in fullscreen).
const TileSideContext = React.createContext<HTMLElement | null>(null);

export const TileSideProvider = TileSideContext.Provider;

export const useTileSide = (): HTMLElement | null => React.useContext(TileSideContext);
