import { create } from "zustand";
import type { Page } from "../app/Sidebar";

interface NavState {
  page: Page;
  setPage: (page: Page) => void;
}

export const useNav = create<NavState>((set) => ({
  page: "mods",
  setPage: (page) => set({ page }),
}));
