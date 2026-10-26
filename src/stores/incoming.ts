import { listen } from "@tauri-apps/api/event";
import { create } from "zustand";
import { ipc, toAppError, type FoundDownload, type NxmLink } from "../lib/ipc";
import { useErrors } from "./errors";
import { useMods } from "./mods";

const now = () => Math.floor(Date.now() / 1000);

interface IncomingState {
  link: NxmLink | null;
  name: string;
  installing: boolean;
  progress: { done: number; total: number } | null;
  expired: boolean;
  watchSince: number | null;
  found: FoundDownload[];
  dismissed: string[];
  receive: (link: NxmLink) => Promise<void>;
  cancelLink: () => void;
  confirmLink: () => Promise<void>;
  startWatch: () => void;
  check: () => Promise<void>;
  dismiss: (path: string) => void;
  installFound: (path: string) => Promise<void>;
}

export const useIncoming = create<IncomingState>((set, get) => ({
  link: null,
  name: "",
  installing: false,
  progress: null,
  expired: false,
  watchSince: null,
  found: [],
  dismissed: [],

  receive: async (link) => {
    set({ link, name: "", expired: link.expires * 1000 < Date.now() });
    try {
      const name = await ipc.nexusModName(link.mod_id);
      if (get().link === link) set({ name });
    } catch {
      if (get().link === link) set({ name: "" });
    }
  },

  cancelLink: () => {
    if (!get().installing) set({ link: null, name: "", expired: false });
  },

  confirmLink: async () => {
    const link = get().link;
    if (!link || get().installing) return;
    set({ installing: true, progress: null });
    useMods.getState().begin();
    let stop = () => {};
    try {
      stop = await listen<{ done: number; total: number }>("nexus-progress", (e) => set({ progress: e.payload }));
      await ipc.nexusInstallNxm(link);
      await useMods.getState().refresh();
      set({ link: null, name: "" });
    } catch (e) {
      set({ link: null, name: "" });
      useErrors.getState().report(e);
    } finally {
      stop();
      useMods.getState().end();
      set({ installing: false, progress: null });
    }
  },

  startWatch: () => set({ watchSince: now() - 5 }),

  check: async () => {
    const since = get().watchSince;
    if (since === null) return;
    try {
      const list = await ipc.downloadsRecent(since);
      set({ found: list.filter((f) => !get().dismissed.includes(f.path)) });
    } catch (e) {
      if (toAppError(e).code !== "game_not_found") set({ found: [] });
    }
  },

  dismiss: (path) =>
    set((s) => ({ dismissed: [...s.dismissed, path], found: s.found.filter((f) => f.path !== path) })),

  installFound: async (path) => {
    useMods.getState().begin();
    try {
      await ipc.installDownload(path);
      await useMods.getState().refresh();
      get().dismiss(path);
    } catch (e) {
      useErrors.getState().report(e);
    } finally {
      useMods.getState().end();
    }
  },
}));
