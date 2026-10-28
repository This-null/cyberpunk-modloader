import { create } from "zustand";
import { persist } from "zustand/middleware";
import { ipc, toAppError } from "../lib/ipc";
import { useErrors } from "./errors";
import { useMods } from "./mods";

export type Theme = "system" | "dark" | "light" | "midnight" | "ocean" | "mocha" | "aurora" | "sunset" | "neon" | "emerald";
export type Motion = "system" | "full" | "reduced";
export type AutoTranslate = "ask" | "on" | "off";

interface SettingsState {
  theme: Theme;
  setTheme: (theme: Theme) => void;
  motion: Motion;
  setMotion: (motion: Motion) => void;
  translate: AutoTranslate;
  setTranslate: (translate: AutoTranslate) => void;
  updates: "on" | "off";
  setUpdates: (updates: "on" | "off") => void;
  translateLang: string;
  setTranslateLang: (lang: string) => void;
  modUpdates: "manual" | "auto";
  setModUpdates: (v: "manual" | "auto") => void;
  gameDir: string | null;
  detected: string[];
  invalid: boolean;
  init: () => Promise<void>;
  chooseGame: (path: string) => Promise<boolean>;
}

export function applyTheme(theme: Theme) {
  const root = document.documentElement;
  if (theme === "system") {
    root.removeAttribute("data-theme");
  } else {
    root.setAttribute("data-theme", theme);
  }
}

export function applyMotion(motion: Motion) {
  const root = document.documentElement;
  if (motion === "system") root.removeAttribute("data-motion");
  else root.setAttribute("data-motion", motion);
}

export function motionReduced(motion: Motion): boolean {
  if (motion !== "system") return motion === "reduced";
  return window.matchMedia?.("(prefers-reduced-motion: reduce)").matches ?? false;
}

export const useSettings = create<SettingsState>()(
  persist(
    (set, get) => ({
      theme: "system",
      setTheme: (theme) => set({ theme }),
      motion: "system",
      setMotion: (motion) => set({ motion }),
      translate: "ask",
      setTranslate: (translate) => set({ translate }),
      updates: "on",
      setUpdates: (updates) => set({ updates }),
      translateLang: "app",
      setTranslateLang: (translateLang) => set({ translateLang }),
      modUpdates: "manual",
      setModUpdates: (modUpdates) => set({ modUpdates }),
      gameDir: null,
      detected: [],
      invalid: false,

      init: async () => {
        try {
          const [gameDir, detected] = await Promise.all([ipc.getGameDir(), ipc.detectGames()]);
          set({ gameDir, detected });
          if (!gameDir && detected.length > 0) {
            await get().chooseGame(detected[0]);
          }
        } catch (e) {
          useErrors.getState().report(e);
        }
      },

      chooseGame: async (path) => {
        try {
          await ipc.setGameDir(path);
          set({ gameDir: path, invalid: false });
          await useMods.getState().refresh();
          return true;
        } catch (e) {
          if (toAppError(e).code === "game_not_found") {
            set({ invalid: true });
          } else {
            useErrors.getState().report(e);
          }
          return false;
        }
      },
    }),
    { name: "cp2077-theme", partialize: (s) => ({ theme: s.theme, motion: s.motion, translate: s.translate, updates: s.updates, translateLang: s.translateLang, modUpdates: s.modUpdates }) },
  ),
);
