import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { create } from "zustand";
import { useErrors } from "./errors";
import { useMods } from "./mods";

export type UpdateStatus = "idle" | "checking" | "none" | "available" | "downloading" | "failed";

interface UpdateState {
  status: UpdateStatus;
  version: string;
  notes: string;
  percent: number;
  hidden: boolean;
  check: (manual?: boolean) => Promise<void>;
  install: () => Promise<void>;
  hide: () => void;
}

let found: Update | null = null;

export const useUpdate = create<UpdateState>((set, get) => ({
  status: "idle",
  version: "",
  notes: "",
  percent: 0,
  hidden: false,

  check: async (manual = false) => {
    if (get().status === "checking" || get().status === "downloading") return;
    set({ status: "checking" });
    try {
      const update = await check();
      found = update;
      if (update) {
        set({ status: "available", version: update.version, notes: update.body ?? "", hidden: false });
      } else {
        set({ status: "none" });
      }
    } catch {
      set({ status: manual ? "failed" : "idle" });
    }
  },

  install: async () => {
    const update = found;
    if (!update || get().status === "downloading" || useMods.getState().busy) return;
    set({ status: "downloading", percent: 0 });
    useMods.getState().begin();
    let total = 0;
    let done = 0;
    try {
      await update.downloadAndInstall((event) => {
        if (event.event === "Started") total = event.data.contentLength ?? 0;
        if (event.event === "Progress") {
          done += event.data.chunkLength;
          if (total > 0) set({ percent: Math.min(100, Math.round((done / total) * 100)) });
        }
      });
      await relaunch();
    } catch (e) {
      set({ status: "failed" });
      useErrors.getState().report(e);
    } finally {
      useMods.getState().end();
    }
  },

  hide: () => set({ hidden: true }),
}));
