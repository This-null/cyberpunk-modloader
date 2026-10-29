import { create } from "zustand";
import type { NexusItem } from "../lib/ipc";
import { useMods } from "./mods";
import { useNexus } from "./nexus";
import { useNexusBrowse } from "./nexusBrowse";

interface ModUpdatesState {
  running: string | null;
  updated: string[];
  tried: number[];
  updateOne: (name: string) => Promise<void>;
  updateAll: () => Promise<void>;
  auto: () => Promise<void>;
  clearUpdated: () => void;
}

export function updatable(items: Record<string, NexusItem>): NexusItem[] {
  const seen = new Set<number>();
  return Object.values(items).filter((i) => {
    if (!i.update || !i.latest || i.mod_id <= 0 || seen.has(i.mod_id)) return false;
    seen.add(i.mod_id);
    return true;
  });
}

export const useModUpdates = create<ModUpdatesState>((set, get) => ({
  running: null,
  updated: [],
  tried: [],

  updateOne: async (name) => {
    const item = useNexus.getState().items[name];
    if (!item || !item.update || get().running) return;
    set({ running: item.name, tried: [...get().tried, item.mod_id] });
    try {
      await useNexusBrowse.getState().install(item.mod_id);
      await useMods.getState().refresh();
      await useNexus.getState().check();
      if (useNexus.getState().items[name]?.update !== true) set({ updated: [...get().updated, name] });
    } finally {
      set({ running: null });
    }
  },

  updateAll: async () => {
    for (const item of updatable(useNexus.getState().items)) {
      if (useNexusBrowse.getState().premiumNeeded) break;
      await get().updateOne(item.name);
    }
  },

  auto: async () => {
    if (get().running || useMods.getState().busy) return;
    const next = updatable(useNexus.getState().items).find((i) => !get().tried.includes(i.mod_id));
    if (next) await get().updateOne(next.name);
  },

  clearUpdated: () => set({ updated: [] }),
}));
