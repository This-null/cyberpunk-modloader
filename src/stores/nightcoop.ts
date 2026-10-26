import { listen } from "@tauri-apps/api/event";
import { create } from "zustand";
import { activeLocale, useLocale } from "../i18n";
import { ipc, toAppError, type Detected, type NcData } from "../lib/ipc";
import { ncLang } from "../lib/nightcoop";
import { useErrors } from "./errors";
import { useMods } from "./mods";

interface NightcoopState {
  data: NcData | null;
  detected: Detected | null;
  loading: boolean;
  failed: boolean;
  installing: string | null;
  progress: { done: number; total: number } | null;
  fetch: () => Promise<void>;
  detect: () => Promise<void>;
  install: (version: string, force?: boolean) => Promise<void>;
}

function lang() {
  return ncLang(activeLocale(useLocale.getState().choice));
}

export const useNightcoop = create<NightcoopState>((set, get) => ({
  data: null,
  detected: null,
  loading: false,
  failed: false,
  installing: null,
  progress: null,

  fetch: async () => {
    set({ loading: true });
    try {
      set({ data: await ipc.ncFetch(lang()), failed: false });
      void get().detect();
    } catch {
      set({ failed: true });
    } finally {
      set({ loading: false });
    }
  },

  detect: async () => {
    const data = get().data;
    if (!data) return;
    try {
      const builds = data.releases.filter((r) => r.build).map((r) => ({ version: r.version, build: r.build }));
      set({ detected: await ipc.ncDetect(builds) });
    } catch {
      set({ detected: null });
    }
  },

  install: async (version, force = false) => {
    set({ installing: version, progress: null });
    useMods.getState().begin();
    let stop = () => {};
    try {
      stop = await listen<{ done: number; total: number }>("nc-progress", (e) => set({ progress: e.payload }));
      await ipc.ncInstall(version, lang(), force);
      useMods.setState({ pending: null });
      await useMods.getState().refresh();
      void get().detect();
    } catch (e) {
      const a = toAppError(e);
      if (a.code === "conflict" && a.details) {
        useMods.setState({
          pending: { name: `NightCoop ${version}`, conflicts: a.details, retry: () => get().install(version, true) },
        });
      } else {
        useErrors.getState().report(e);
      }
    } finally {
      stop();
      useMods.getState().end();
      set({ installing: null, progress: null });
    }
  },
}));
