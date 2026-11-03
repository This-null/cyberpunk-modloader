import { create } from "zustand";

const KEY = "cp2077-mod-groups";

interface Saved {
  assign: Record<string, string>;
  closed: string[];
}

function load(): Saved {
  try {
    const raw = JSON.parse(localStorage.getItem(KEY) ?? "null") as Partial<Saved> | null;
    const assign: Record<string, string> = {};
    for (const [mod, group] of Object.entries(raw?.assign ?? {})) {
      if (typeof group === "string" && group.trim() !== "") assign[mod] = group;
    }
    return { assign, closed: Array.isArray(raw?.closed) ? raw.closed.filter((g) => typeof g === "string") : [] };
  } catch {
    return { assign: {}, closed: [] };
  }
}

function save(state: Saved) {
  try {
    localStorage.setItem(KEY, JSON.stringify(state));
  } catch {
    // storage can be unavailable; groups then last for this session only
  }
}

interface ModGroupsState extends Saved {
  move: (mods: string[], group: string | null) => void;
  toggle: (group: string) => void;
}

export const useModGroups = create<ModGroupsState>((set, get) => ({
  ...load(),

  move: (mods, group) => {
    const name = group?.trim().slice(0, 40) ?? "";
    const assign = Object.fromEntries(Object.entries(get().assign).filter(([m]) => !mods.includes(m)));
    if (name !== "") for (const m of mods) assign[m] = name;
    set({ assign });
    save({ assign, closed: get().closed });
  },

  toggle: (group) => {
    const closed = get().closed.includes(group) ? get().closed.filter((g) => g !== group) : [...get().closed, group];
    set({ closed });
    save({ assign: get().assign, closed });
  },
}));

export interface Section<T> {
  id: string;
  kind: "updates" | "group" | "rest";
  title: string;
  items: T[];
}

export function sectionsOf<T>(
  items: T[],
  nameOf: (item: T) => string,
  hasUpdate: (item: T) => boolean,
  assign: Record<string, string>,
): Section<T>[] {
  const updates: T[] = [];
  const byGroup = new Map<string, T[]>();
  const rest: T[] = [];
  for (const item of items) {
    const group = assign[nameOf(item)];
    if (hasUpdate(item)) updates.push(item);
    else if (group) byGroup.set(group, [...(byGroup.get(group) ?? []), item]);
    else rest.push(item);
  }
  const out: Section<T>[] = [];
  if (updates.length > 0) out.push({ id: "updates", kind: "updates", title: "", items: updates });
  for (const [title, list] of [...byGroup.entries()].sort((a, b) => a[0].localeCompare(b[0]))) {
    out.push({ id: `g:${title}`, kind: "group", title, items: list });
  }
  if (rest.length > 0) out.push({ id: "rest", kind: "rest", title: "", items: rest });
  return out;
}
