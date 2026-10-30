import { create } from "zustand";
import { ipc } from "../lib/ipc";

interface GameInfoState {
  patchTime: number | null;
  load: () => Promise<void>;
}

export const useGameInfo = create<GameInfoState>((set) => ({
  patchTime: null,
  load: async () => {
    set({ patchTime: await ipc.gamePatchTime().catch(() => null) });
  },
}));
