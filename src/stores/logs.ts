import { create } from "zustand";
import { ipc, type CrashSummary, type LogFile } from "../lib/ipc";
import { useErrors } from "./errors";

interface LogsState {
  files: LogFile[];
  summary: CrashSummary | null;
  selected: string | null;
  text: string;
  start: number;
  end: number;
  size: number;
  loading: boolean;
  follow: boolean;
  loadList: () => Promise<void>;
  select: (id: string) => Promise<void>;
  earlier: () => Promise<void>;
  refresh: () => Promise<void>;
  poll: () => Promise<void>;
  setFollow: (on: boolean) => void;
  clear: () => Promise<{ removed: number; freed: number } | null>;
}

const MAX_TEXT = 4_000_000;
let polling = false;

function capped(text: string, start: number) {
  if (text.length <= MAX_TEXT) return { text, start };
  const cut = text.length - MAX_TEXT;
  return { text: text.slice(cut), start: start + new TextEncoder().encode(text.slice(0, cut)).length };
}

export const useLogs = create<LogsState>((set, get) => ({
  files: [],
  summary: null,
  selected: null,
  text: "",
  start: 0,
  end: 0,
  size: 0,
  loading: false,
  follow: false,

  loadList: async () => {
    try {
      const [files, summary] = await Promise.all([ipc.logsScan(), ipc.logsSummary()]);
      set({ files, summary });
    } catch (e) {
      useErrors.getState().report(e);
    }
  },

  select: async (id) => {
    set({ selected: id, loading: true, text: "", start: 0, end: 0, size: 0 });
    try {
      const crash = get().files.find((f) => f.id === id)?.crash;
      const chunk = crash ? await ipc.logsHead(id) : await ipc.logsRead(id);
      if (get().selected === id) {
        set({ text: chunk.text, start: chunk.start, end: chunk.end, size: chunk.size });
      }
    } catch (e) {
      useErrors.getState().report(e);
    } finally {
      set({ loading: false });
    }
  },

  earlier: async () => {
    const { selected, start, text } = get();
    if (!selected || start <= 0) return;
    set({ loading: true });
    try {
      const chunk = await ipc.logsRead(selected, start);
      if (get().selected === selected && get().start === start && get().text === text) {
        set({ text: chunk.text + text, start: chunk.start });
      }
    } catch (e) {
      useErrors.getState().report(e);
    } finally {
      set({ loading: false });
    }
  },

  refresh: async () => {
    const id = get().selected;
    await get().loadList();
    if (id) await get().select(id);
  },

  poll: async () => {
    const { selected, end } = get();
    if (!selected || polling) return;
    polling = true;
    try {
      const chunk = await ipc.logsPoll(selected, end);
      const now = get();
      if (now.selected !== selected || now.end !== end) return;
      if (chunk.start < end) {
        set({ ...capped(chunk.text, chunk.start), end: chunk.end, size: chunk.size });
      } else if (chunk.end > end) {
        set({ ...capped(now.text + chunk.text, now.start), end: chunk.end, size: chunk.size });
      }
    } catch {
      set({ follow: false });
    } finally {
      polling = false;
    }
  },

  setFollow: (on) => set({ follow: on }),

  clear: async () => {
    try {
      const done = await ipc.logsClear();
      set({ selected: null, text: "", start: 0, end: 0, size: 0, follow: false });
      await get().loadList();
      return done;
    } catch (e) {
      useErrors.getState().report(e);
      return null;
    }
  },
}));
