import { create } from "zustand";
import { translate } from "../i18n";
import { ipc, toAppError, type Conflict, type ExternalMod, type ModRecord } from "../lib/ipc";
import { modNameFromPath } from "../lib/names";
import { useErrors } from "./errors";

export interface PendingConflict {
  name: string;
  conflicts: Conflict[];
  retry?: () => Promise<void>;
}

interface ModsState {
  mods: ModRecord[];
  external: ExternalMod[];
  busy: boolean;
  begin: () => void;
  end: () => void;
  pending: PendingConflict | null;
  images: Record<string, string | null>;
  detail: string | null;
  notice: string | null;
  skipped: string[];
  clearSkipped: () => void;
  selected: string[];
  setSelected: (keys: string[]) => void;
  toggleSelected: (key: string) => void;
  bulkEnable: (names: string[]) => Promise<void>;
  bulkDisable: (names: string[]) => Promise<void>;
  bulkUninstall: (names: string[]) => Promise<void>;
  bulkImport: (names: string[]) => Promise<void>;
  exportSome: (names: string[], dest: string) => Promise<void>;
  loadImage: (name: string) => Promise<void>;
  openDetail: (name: string | null) => void;
  saveInfo: (name: string, description: string, version: string, imagePath: string | null) => Promise<void>;
  exportOne: (name: string, dest: string) => Promise<void>;
  exportAll: (dest: string) => Promise<void>;
  clearNotice: () => void;
  refresh: () => Promise<void>;
  install: (paths: string[]) => Promise<void>;
  importExternal: (name: string) => Promise<void>;
  enable: (name: string, force?: boolean) => Promise<void>;
  disable: (name: string) => Promise<void>;
  uninstall: (name: string) => Promise<void>;
  cancelPending: () => void;
}

export const useMods = create<ModsState>((set, get) => {
  let depth = 0;
  const begin = () => {
    depth += 1;
    set({ busy: true });
  };
  const end = () => {
    depth = Math.max(0, depth - 1);
    set({ busy: depth > 0 });
  };

  async function run(action: () => Promise<void>) {
    begin();
    try {
      await action();
    } catch (e) {
      useErrors.getState().report(e);
    } finally {
      end();
    }
  }

  return {
    mods: [],
    external: [],
    busy: false,
    begin,
    end,
    pending: null,
    images: {},
    detail: null,
    notice: null,
    skipped: [],
    clearSkipped: () => set({ skipped: [] }),
    selected: [],

    setSelected: (keys) => set({ selected: keys }),

    toggleSelected: (key) =>
      set((s) => ({ selected: s.selected.includes(key) ? s.selected.filter((k) => k !== key) : [...s.selected, key] })),

    loadImage: async (name) => {
      if (name in get().images) return;
      const url = await ipc.modImage(name).catch(() => null);
      set((s) => ({ images: { ...s.images, [name]: url } }));
    },

    openDetail: (name) => set({ detail: name }),

    saveInfo: (name, description, version, imagePath) =>
      run(async () => {
        await ipc.setModInfo(name, description, version, imagePath);
        set((s) => {
          const { [name]: _gone, ...rest } = s.images;
          return { images: rest };
        });
        await get().refresh();
        await get().loadImage(name);
      }),

    exportOne: (name, dest) =>
      run(async () => {
        await ipc.exportMod(name, dest);
        set({ notice: dest });
      }),

    exportAll: (dest) =>
      run(async () => {
        await ipc.exportPack(
          get().mods.map((m) => m.name),
          dest,
        );
        set({ notice: dest });
      }),

    exportSome: (names, dest) =>
      run(async () => {
        await ipc.exportPack(names, dest);
        set({ notice: dest });
      }),

    bulkDisable: (names) =>
      run(async () => {
        try {
          for (const name of names) await ipc.disableMod(name);
        } finally {
          await get().refresh();
        }
      }),

    bulkUninstall: (names) =>
      run(async () => {
        try {
          for (const name of names) {
            await ipc.uninstallMod(name);
            set((s) => {
              const { [name]: _gone, ...rest } = s.images;
              return { images: rest, detail: s.detail === name ? null : s.detail };
            });
          }
        } finally {
          await get().refresh();
        }
      }),

    bulkImport: (names) =>
      run(async () => {
        try {
          for (const name of names) await ipc.importMod(name);
        } finally {
          await get().refresh();
        }
      }),

    bulkEnable: async (names) => {
      begin();
      const conflicted: { name: string; conflicts: Conflict[] }[] = [];
      try {
        for (const name of names) {
          try {
            await ipc.enableMod(name, false);
          } catch (e) {
            const a = toAppError(e);
            if (a.code === "conflict" && a.details) conflicted.push({ name, conflicts: a.details });
            else throw e;
          }
        }
        await get().refresh();
        if (conflicted.length > 0) {
          const seen = new Set<string>();
          const merged = conflicted
            .flatMap((c) => c.conflicts)
            .filter((c) => {
              const key = `${c.file}|${c.owner ?? ""}`;
              if (seen.has(key)) return false;
              seen.add(key);
              return true;
            });
          set({
            pending: {
              name: translate("mods.countMods", { count: conflicted.length }),
              conflicts: merged,
              retry: async () => {
                begin();
                try {
                  for (const c of conflicted) await ipc.enableMod(c.name, true);
                  set({ pending: null });
                } catch (e) {
                  useErrors.getState().report(e);
                } finally {
                  await get().refresh();
                  end();
                }
              },
            },
          });
        }
      } catch (e) {
        useErrors.getState().report(e);
        await get().refresh();
      } finally {
        end();
      }
    },

    clearNotice: () => set({ notice: null }),

    refresh: async () => {
      try {
        const [mods, external] = await Promise.all([ipc.listMods(), ipc.scanExternal()]);
        set({ mods, external });
      } catch (e) {
        if (toAppError(e).code === "game_not_found") {
          set({ mods: [], external: [] });
        } else {
          useErrors.getState().report(e);
        }
      }
    },

    install: (paths) =>
      run(async () => {
        const skipped: string[] = [];
        try {
          for (const path of paths) {
            const name = modNameFromPath(
              path,
              get().mods.map((m) => m.name),
            );
            const done = await ipc.installMod(path, name);
            skipped.push(...done.skipped);
            await get().refresh();
          }
        } finally {
          set({ skipped });
        }
      }),

    importExternal: (name) =>
      run(async () => {
        await ipc.importMod(name);
        await get().refresh();
      }),

    enable: async (name, force = false) => {
      begin();
      try {
        await ipc.enableMod(name, force);
        set({ pending: null });
        await get().refresh();
      } catch (e) {
        const a = toAppError(e);
        if (a.code === "conflict" && a.details) {
          set({ pending: { name, conflicts: a.details } });
        } else {
          useErrors.getState().report(e);
        }
      } finally {
        end();
      }
    },

    disable: (name) =>
      run(async () => {
        await ipc.disableMod(name);
        await get().refresh();
      }),

    uninstall: (name) =>
      run(async () => {
        await ipc.uninstallMod(name);
        set((s) => {
          const { [name]: _gone, ...rest } = s.images;
          return { images: rest, detail: s.detail === name ? null : s.detail };
        });
        await get().refresh();
      }),

    cancelPending: () => set({ pending: null }),
  };
});
