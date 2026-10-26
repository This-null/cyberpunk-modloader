import { create } from "zustand";
import { ipc, toAppError, type NexusItem, type NexusStatus } from "../lib/ipc";
import { useErrors } from "./errors";
import { useMods } from "./mods";

interface NexusState {
  status: NexusStatus | null;
  items: Record<string, NexusItem>;
  connecting: boolean;
  checking: boolean;
  connectError: string | null;
  load: () => Promise<void>;
  connect: (key: string) => Promise<boolean>;
  disconnect: () => Promise<void>;
  check: () => Promise<void>;
  link: (name: string, target: string) => Promise<string | null>;
  unlink: (name: string) => Promise<void>;
}

const INLINE = new Set(["nexus_key", "rate_limited", "network"]);

export const useNexus = create<NexusState>((set, get) => ({
  status: null,
  items: {},
  connecting: false,
  checking: false,
  connectError: null,

  load: async () => {
    try {
      const status = await ipc.nexusStatus();
      set({ status });
      if (status.connected) await get().check();
    } catch {
      set({ status: { connected: false, user: null } });
    }
  },

  connect: async (key) => {
    set({ connecting: true, connectError: null });
    try {
      const status = await ipc.nexusConnect(key);
      set({ status });
      void get().check();
      return true;
    } catch (e) {
      const a = toAppError(e);
      if (INLINE.has(a.code)) set({ connectError: a.code });
      else useErrors.getState().report(e);
      return false;
    } finally {
      set({ connecting: false });
    }
  },

  disconnect: async () => {
    try {
      await ipc.nexusDisconnect();
      set({ status: { connected: false, user: null }, items: {}, connectError: null });
    } catch (e) {
      useErrors.getState().report(e);
    }
  },

  check: async () => {
    if (get().checking) return;
    set({ checking: true });
    try {
      const list = await ipc.nexusCheck();
      set({ items: Object.fromEntries(list.map((i) => [i.name, i])) });
    } catch (e) {
      if (toAppError(e).code === "nexus_key") set({ status: { connected: false, user: null }, items: {} });
    } finally {
      set({ checking: false });
    }
  },

  link: async (name, target) => {
    try {
      await ipc.nexusLink(name, target);
      await useMods.getState().refresh();
      void get().check();
      return null;
    } catch (e) {
      const a = toAppError(e);
      if (a.code === "bad_link") return a.code;
      useErrors.getState().report(e);
      return a.code;
    }
  },

  unlink: async (name) => {
    try {
      await ipc.nexusUnlink(name);
      await useMods.getState().refresh();
      set((s) => {
        const { [name]: _gone, ...rest } = s.items;
        return { items: rest };
      });
    } catch (e) {
      useErrors.getState().report(e);
    }
  },
}));
