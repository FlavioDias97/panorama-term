export interface StoreChange {
  name: string;
  revision: number;
  origin: string | null;
}

export const shouldReload = (event: StoreChange, origin: string): boolean =>
  event.origin !== origin && (event.name === 'workspaces.json' || /^workspaces\/[^/]+\.json$/.test(event.name));

export const createDeferredReload = (reload: (names: string[]) => void) => {
  let down = false;
  const pending = new Set<string>();
  const flush = () => {
    if (down || !pending.size) return;
    const names = [...pending];
    pending.clear();
    reload(names);
  };
  return {
    start: () => {
      down = true;
    },
    end: () => {
      down = false;
      flush();
    },
    change: (name: string) => {
      pending.add(name);
      flush();
    }
  };
};

export const shouldPersist = (
  applied: { tiles: unknown; frames: unknown },
  current: { tiles: unknown; frames: unknown }
): boolean => applied.tiles !== current.tiles || applied.frames !== current.frames;
