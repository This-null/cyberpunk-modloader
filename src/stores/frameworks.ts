import { create } from "zustand";
import { FRAMEWORKS, isNewer } from "../lib/frameworks";
import { ipc, toAppError } from "../lib/ipc";
import { useErrors } from "./errors";
import { useNexusBrowse } from "./nexusBrowse";

export interface LatestInfo {
  mod_id: number;
  version: string;
}

interface FrameworkStore {
  present: Record<string, boolean>;
  versions: Record<string, string | null>;
  oldLoader: boolean;
  latest: Record<string, LatestInfo | null>;
  looking: boolean;
  busy: string | null;
  load: () => Promise<void>;
  lookup: () => Promise<void>;
  install: (id: string) => Promise<void>;
  remove: (id: string) => Promise<void>;
  owned: { id: string; mod: string } | null;
  plan: string[];
  arm: (id: string) => Promise<void>;
}

export const useFrameworks = create<FrameworkStore>((set, get) => ({
  present: {},
  versions: {},
  oldLoader: false,
  latest: {},
  looking: false,
  busy: null,
  owned: null,
  plan: [],
  load: async () => {
    try {
      const report = await ipc.frameworkStatus();
      set({
        present: Object.fromEntries(report.frameworks.map((f) => [f.id, f.present])),
        versions: Object.fromEntries(report.frameworks.map((f) => [f.id, f.version])),
        oldLoader: report.old_loader,
      });
    } catch {
      set({ present: {}, versions: {}, oldLoader: false });
    }
  },
  lookup: async () => {
    if (get().looking) return;
    set({ looking: true });
    const latest: Record<string, LatestInfo | null> = { ...get().latest };
    for (const fw of FRAMEWORKS) {
      if (latest[fw.id] !== undefined) continue;
      try {
        const page = await ipc.nexusSearch({ query: fw.name, categories: [], hide_adult: true, sort: "downloads", tag: null, offset: 0, count: 10 });
        const hit = page.items.find((m) => (m.name ?? "").trim().toLowerCase() === fw.name.toLowerCase());
        latest[fw.id] = hit ? { mod_id: hit.mod_id, version: hit.version } : null;
      } catch {
        latest[fw.id] = null;
      }
    }
    set({ latest, looking: false });
  },
  arm: async (id) => {
    set({ plan: await ipc.frameworkPlan(id).catch(() => []) });
  },
  remove: async (id) => {
    set({ owned: null });
    try {
      await ipc.frameworkRemove(id);
    } catch (e) {
      const a = toAppError(e);
      if (a.code === "owned") set({ owned: { id, mod: a.message.split(": ").slice(1).join(": ") } });
      else useErrors.getState().report(e);
    }
    await get().load();
  },
  install: async (id) => {
    const info = get().latest[id];
    if (!info) return;
    set({ busy: id });
    try {
      await useNexusBrowse.getState().installWithRequirements(info.mod_id);
    } finally {
      set({ busy: null });
    }
    await get().load();
  },
}));

export function frameworkOutdated(id: string, versions: Record<string, string | null>, latest: Record<string, LatestInfo | null>): boolean {
  const have = versions[id];
  const want = latest[id]?.version;
  return Boolean(have && want && isNewer(want, have));
}
