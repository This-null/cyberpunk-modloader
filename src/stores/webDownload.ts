import { create } from "zustand";
import { toAppError } from "../lib/ipc";
import { useErrors } from "./errors";
import { useMods } from "./mods";
import { useNexus } from "./nexus";

export interface WebDownloadEvent {
  id: number;
  phase: "downloading" | "installing" | "done" | "error" | "cancelled";
  name: string;
  done: number;
  total: number;
  error: { code: string; message: string; details?: unknown } | null;
}

export interface DownloadItem {
  id: number;
  name: string;
  phase: WebDownloadEvent["phase"];
  done: number;
  total: number;
  message: string | null;
  added: number;
  at: number;
  moved: number;
}

interface WebDownloadState {
  items: DownloadItem[];
  handle: (e: WebDownloadEvent) => void;
  dismiss: (id: number) => void;
  clearFinished: () => void;
}

export const isActive = (i: DownloadItem) => i.phase === "downloading" || i.phase === "installing";

let busy = false;

const KEY = "cp2077-download-history";
const KEEP = 30;

function history(): DownloadItem[] {
  try {
    const raw = JSON.parse(localStorage.getItem(KEY) ?? "[]") as DownloadItem[];
    return raw
      .filter((i) => typeof i.name === "string" && typeof i.at === "number" && ["done", "error", "cancelled"].includes(i.phase))
      .slice(0, KEEP)
      .map((i, n) => ({ ...i, id: -(n + 1), moved: i.at }));
  } catch {
    return [];
  }
}

function remember(items: DownloadItem[]) {
  try {
    localStorage.setItem(KEY, JSON.stringify(items.filter((i) => !isActive(i)).slice(0, KEEP)));
  } catch {
    // the list then lasts for this session only
  }
}

export const useWebDownload = create<WebDownloadState>((set, get) => ({
  items: history(),

  handle: (e) => {
    const prev = get().items.find((i) => i.id === e.id);
    const message = e.phase === "error" ? (e.error ? toAppError(e.error).message : null) : null;
    const next: DownloadItem = {
      id: e.id,
      name: e.name || prev?.name || "",
      phase: e.phase,
      done: e.phase === "done" ? (prev?.done ?? 0) : e.done,
      total: e.phase === "installing" || e.phase === "done" ? (prev?.total ?? e.total) : e.total,
      message,
      added: e.phase === "done" ? e.done : 0,
      moved: prev && prev.done === e.done && prev.phase === e.phase ? prev.moved : Date.now(),
      at: e.phase === "done" || e.phase === "error" || e.phase === "cancelled" ? Date.now() : (prev?.at ?? Date.now()),
    };
    set({ items: prev ? get().items.map((i) => (i.id === e.id ? next : i)) : [next, ...get().items] });
    remember(get().items);

    const active = get().items.some(isActive);
    if (active && !busy) {
      busy = true;
      useMods.getState().begin();
    } else if (!active && busy) {
      busy = false;
      useMods.getState().end();
    }
    if (e.phase === "error" && e.error) useErrors.getState().report(toAppError(e.error));
    if (e.phase === "done" || e.phase === "error") void useMods.getState().refresh();
    if (e.phase === "done") void useNexus.getState().check();
  },

  dismiss: (id) => {
    set({ items: get().items.filter((i) => i.id !== id || isActive(i)) });
    remember(get().items);
  },

  clearFinished: () => {
    set({ items: get().items.filter(isActive) });
    remember(get().items);
  },
}));
