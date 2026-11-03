import { create } from "zustand";

const KEY = "cp2077-mod-order";

function load(): string[] {
  try {
    const raw = JSON.parse(localStorage.getItem(KEY) ?? "null") as unknown;
    return Array.isArray(raw) ? raw.filter((k): k is string => typeof k === "string") : [];
  } catch {
    return [];
  }
}

function save(order: string[]) {
  try {
    localStorage.setItem(KEY, JSON.stringify(order));
  } catch {
    // storage can be unavailable; the order then lasts for this session only
  }
}

export function applyOrder<T>(items: T[], keyOf: (item: T) => string, order: string[]): T[] {
  if (order.length === 0) return items;
  const rank = new Map(order.map((k, i) => [k, i]));
  return items
    .map((item, i) => ({ item, i, r: rank.get(keyOf(item)) ?? Number.MAX_SAFE_INTEGER }))
    .sort((a, b) => a.r - b.r || a.i - b.i)
    .map((x) => x.item);
}

export function moved(order: string[], all: string[], key: string, over: string, after: boolean): string[] {
  const base = applyOrder(all, (k) => k, order).filter((k) => k !== key);
  const at = base.indexOf(over);
  if (at < 0) return order;
  base.splice(after ? at + 1 : at, 0, key);
  return base;
}

interface ModOrderState {
  order: string[];
  reorder: (all: string[], key: string, over: string, after: boolean) => void;
  reset: () => void;
}

export const useModOrder = create<ModOrderState>((set, get) => ({
  order: load(),

  reorder: (all, key, over, after) => {
    const order = moved(get().order, all, key, over, after);
    set({ order });
    save(order);
  },

  reset: () => {
    set({ order: [] });
    save([]);
  },
}));
