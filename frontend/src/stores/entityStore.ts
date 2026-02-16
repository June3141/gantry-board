import { create } from 'zustand';

interface Entity {
  id: string;
}

export interface EntityState<T extends Entity> {
  ids: string[];
  entities: Record<string, T>;
  setAll: (items: T[]) => void;
  setOne: (item: T) => void;
  removeOne: (id: string) => void;
  getById: (id: string) => T | undefined;
  getAll: () => T[];
  clear: () => void;
}

export function createEntityStore<T extends Entity>() {
  return create<EntityState<T>>((set, get) => ({
    ids: [],
    entities: {},
    setAll: (items) => {
      const entities: Record<string, T> = {};
      const ids: string[] = [];
      for (const item of items) {
        entities[item.id] = item;
        ids.push(item.id);
      }
      set({ ids, entities });
    },
    setOne: (item) => {
      set((state) => {
        const exists = item.id in state.entities;
        return {
          entities: { ...state.entities, [item.id]: item },
          ids: exists ? state.ids : [...state.ids, item.id],
        };
      });
    },
    removeOne: (id) => {
      set((state) => {
        if (!(id in state.entities)) return state;
        const { [id]: _, ...rest } = state.entities;
        return {
          entities: rest,
          ids: state.ids.filter((i) => i !== id),
        };
      });
    },
    getById: (id) => get().entities[id],
    getAll: () => {
      const { ids, entities } = get();
      return ids.map((id) => entities[id]).filter(Boolean);
    },
    clear: () => set({ ids: [], entities: {} }),
  }));
}
