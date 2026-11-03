import { create } from "zustand";
import { ipc, type ApplyReport, type Preset } from "../lib/ipc";
import { useErrors } from "./errors";
import { useMods } from "./mods";

interface PresetsState {
  open: boolean;
  list: Preset[];
  report: ApplyReport | null;
  applied: string | null;
  show: () => Promise<void>;
  close: () => void;
  load: () => Promise<void>;
  saveCurrent: (name: string) => Promise<boolean>;
  rename: (old: string, name: string) => Promise<boolean>;
  remove: (name: string) => Promise<void>;
  apply: (name: string) => Promise<void>;
}

export function sameSet(a: readonly string[], b: readonly string[]): boolean {
  return a.length === b.length && [...a].sort().join("\n") === [...b].sort().join("\n");
}

export const useMyPresets = create<PresetsState>((set, get) => ({
  open: false,
  list: [],
  report: null,
  applied: null,
  show: async () => {
    set({ open: true, report: null, applied: null });
    await get().load();
  },
  close: () => set({ open: false }),
  load: async () => {
    try {
      set({ list: await ipc.presetsList() });
    } catch (e) {
      useErrors.getState().report(e);
    }
  },
  saveCurrent: async (name) => {
    const enabled = useMods
      .getState()
      .mods.filter((m) => m.enabled)
      .map((m) => m.name);
    try {
      set({ list: await ipc.presetSave(name, enabled), report: null });
      return true;
    } catch (e) {
      useErrors.getState().report(e);
      return false;
    }
  },
  rename: async (old, name) => {
    try {
      set({ list: await ipc.presetRename(old, name) });
      return true;
    } catch (e) {
      useErrors.getState().report(e);
      return false;
    }
  },
  remove: async (name) => {
    try {
      set({ list: await ipc.presetDelete(name) });
    } catch (e) {
      useErrors.getState().report(e);
    }
  },
  apply: async (name) => {
    useMods.getState().begin();
    try {
      const report = await ipc.presetApply(name);
      set({ report, applied: name });
      await useMods.getState().refresh();
    } catch (e) {
      useErrors.getState().report(e);
    } finally {
      useMods.getState().end();
    }
  },
}));
