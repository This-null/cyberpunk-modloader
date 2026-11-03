import { create } from "zustand";

const KEY = "cp2077-auto-install";

function load(): boolean {
  try {
    return localStorage.getItem(KEY) === "1";
  } catch {
    return false;
  }
}

interface AutoInstallState {
  on: boolean;
  set: (on: boolean) => void;
}

export const useAutoInstall = create<AutoInstallState>((set) => ({
  on: load(),
  set: (on) => {
    try {
      localStorage.setItem(KEY, on ? "1" : "0");
    } catch {
      // the choice then lasts for this session only
    }
    set({ on });
  },
}));
