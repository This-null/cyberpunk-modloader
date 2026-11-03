import { create } from "zustand";
import { ipc, toAppError, type ProfileBuilt, type ProfilePlanItem } from "../lib/ipc";
import { useErrors } from "./errors";
import { useIncoming } from "./incoming";
import { useNexus } from "./nexus";
import { useNexusBrowse } from "./nexusBrowse";
import { useNightcoop } from "./nightcoop";

export type ProfileMode = "share" | "import" | null;

interface ProfileState {
  mode: ProfileMode;
  title: string;
  built: ProfileBuilt | null;
  shared: { code: string; expires: number } | null;
  text: string;
  name: string;
  items: ProfilePlanItem[];
  encoded: string;
  working: boolean;
  invalid: boolean;
  open: (mode: Exclude<ProfileMode, null>, names: string[]) => Promise<void>;
  close: () => void;
  setTitle: (title: string) => void;
  setText: (text: string) => void;
  rebuild: (names: string[]) => Promise<void>;
  share: () => Promise<void>;
  check: () => Promise<void>;
  installMissing: () => Promise<void>;
}

const empty = { built: null, shared: null, text: "", name: "", items: [], encoded: "", invalid: false, working: false };

export const useProfile = create<ProfileState>((set, get) => ({
  mode: null,
  title: "",
  ...empty,

  open: async (mode, names) => {
    set({ mode, ...empty, title: get().title });
    if (mode === "share") await get().rebuild(names);
  },

  close: () => set({ mode: null }),
  setTitle: (title) => set({ title }),
  setText: (text) => set({ text, invalid: false }),

  rebuild: async (names) => {
    try {
      const built = await ipc.profileBuild(names, get().title);
      set({ built, shared: null });
    } catch (e) {
      set({ built: null });
      if (toAppError(e).code !== "unmappable") useErrors.getState().report(e);
    }
  },

  share: async () => {
    const built = get().built;
    if (!built || get().working) return;
    set({ working: true });
    try {
      set({ shared: await ipc.profileShare(built.code) });
    } catch (e) {
      useErrors.getState().report(e);
    } finally {
      set({ working: false });
    }
  },

  check: async () => {
    if (get().working || get().text.trim() === "") return;
    set({ working: true, invalid: false });
    try {
      const view = await ipc.profileOpen(get().text);
      set({ name: view.name, items: view.items, encoded: view.encoded });
    } catch (e) {
      const code = toAppError(e).code;
      set({ items: [], name: "", invalid: true });
      if (code !== "unmappable" && code !== "bad_link" && code !== "not_found") useErrors.getState().report(e);
    } finally {
      set({ working: false });
    }
  },

  installMissing: async () => {
    if (get().working) return;
    set({ working: true });
    try {
      const premium = useNexus.getState().status?.user?.is_premium ?? false;
      for (const item of get().items.filter((x) => !x.installed)) {
        if (item.source === "nightcoop") {
          await useNightcoop.getState().install(item.version);
        } else if (item.source === "nexus" && premium) {
          await useNexusBrowse.getState().install(item.mod_id);
          if (useNexusBrowse.getState().premiumNeeded) break;
        }
      }
    } finally {
      set({ working: false });
    }
    await get().check();
  },
}));

export function openNexusPage(modId: number) {
  useIncoming.getState().startWatch();
  void ipc.openUrl(`https://www.nexusmods.com/cyberpunk2077/mods/${modId}`);
}
