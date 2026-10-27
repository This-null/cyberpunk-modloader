import { listen } from "@tauri-apps/api/event";
import { create } from "zustand";
import { ipc, toAppError, type NexusCategory, type NexusMod, type NexusRequirement } from "../lib/ipc";
import { categoryNames, SEARCH_PAGE, type Sort } from "../lib/browse";
import { useErrors } from "./errors";
import { useMods } from "./mods";

export interface RequirementsEntry {
  loading: boolean;
  failed: boolean;
  items: NexusRequirement[];
}

interface BrowseState {
  reqs: Record<number, RequirementsEntry>;
  loadReqs: (modId: number) => Promise<void>;
  installMissing: (modId: number) => Promise<void>;
  installWithRequirements: (modId: number) => Promise<void>;
  chain: { done: number; total: number } | null;
  items: NexusMod[];
  total: number;
  categories: NexusCategory[];
  installed: number[];
  loading: boolean;
  loadingMore: boolean;
  failed: string | null;
  installing: number | null;
  progress: { done: number; total: number } | null;
  premiumNeeded: boolean;
  sort: Sort;
  query: string;
  category: number;
  hideAdult: boolean;
  tag: string | null;
  start: () => Promise<void>;
  reload: () => Promise<void>;
  loadMore: () => Promise<void>;
  install: (modId: number) => Promise<void>;
  setSort: (sort: Sort) => void;
  setQuery: (query: string) => void;
  setCategory: (category: number) => void;
  setHideAdult: (hide: boolean) => void;
  setTag: (tag: string | null) => void;
}

let ticket = 0;

export function needed(items: NexusRequirement[], installed: number[]): NexusRequirement[] {
  return items.filter((r) => !r.external && r.mod_id > 0 && !/optional/i.test(r.notes) && !installed.includes(r.mod_id));
}

export const useNexusBrowse = create<BrowseState>((set, get) => {
  function params(offset: number) {
    const s = get();
    return {
      query: s.query,
      categories: categoryNames(s.categories, s.category),
      hide_adult: s.hideAdult,
      sort: s.sort,
      tag: s.tag,
      offset,
      count: SEARCH_PAGE,
    };
  }

  function refetch() {
    void get().reload();
  }

  return {
    reqs: {},
    chain: null,
    items: [],
    total: 0,
    categories: [],
    installed: [],
    loading: false,
    loadingMore: false,
    failed: null,
    installing: null,
    progress: null,
    premiumNeeded: false,
    sort: "downloads",
    query: "",
    category: 0,
    hideAdult: true,
    tag: null,

    start: async () => {
      try {
        const base = await ipc.nexusBrowse("trending");
        set({ categories: base.categories, installed: base.installed });
      } catch {
        set({ categories: [] });
      }
      await get().reload();
    },

    reload: async () => {
      const mine = ++ticket;
      set({ loading: true, loadingMore: false, failed: null });
      try {
        const page = await ipc.nexusSearch(params(0));
        if (mine === ticket) set({ items: page.items, total: page.total });
      } catch (e) {
        if (mine === ticket) set({ failed: toAppError(e).code, items: [], total: 0 });
      } finally {
        if (mine === ticket) set({ loading: false });
      }
    },

    loadMore: async () => {
      const s = get();
      if (s.loading || s.loadingMore || s.failed || s.items.length === 0 || s.items.length >= s.total) return;
      const mine = ticket;
      set({ loadingMore: true });
      try {
        const page = await ipc.nexusSearch(params(s.items.length));
        if (mine === ticket) {
          set((cur) => {
            const have = new Set(cur.items.map((m) => m.mod_id));
            return { items: [...cur.items, ...page.items.filter((m) => !have.has(m.mod_id))], total: page.total };
          });
        }
      } catch (e) {
        if (mine === ticket) set({ failed: toAppError(e).code });
      } finally {
        if (mine === ticket) set({ loadingMore: false });
      }
    },

    install: async (modId) => {
      set({ installing: modId, progress: null, premiumNeeded: false });
      useMods.getState().begin();
      let stop = () => {};
      try {
        stop = await listen<{ done: number; total: number }>("nexus-progress", (e) => set({ progress: e.payload }));
        await ipc.nexusInstall(modId);
        await useMods.getState().refresh();
        set((s) => ({ installed: [...new Set([...s.installed, modId])] }));
      } catch (e) {
        const a = toAppError(e);
        if (a.code === "nexus_premium") set({ premiumNeeded: true });
        else useErrors.getState().report(e);
      } finally {
        stop();
        useMods.getState().end();
        set({ installing: null, progress: null });
      }
    },

    loadReqs: async (modId) => {
      const have = get().reqs[modId];
      if (have && (have.loading || have.items.length > 0)) return;
      set((st) => ({ reqs: { ...st.reqs, [modId]: { loading: true, failed: false, items: [] } } }));
      try {
        const items = await ipc.nexusRequirements(modId);
        set((st) => ({ reqs: { ...st.reqs, [modId]: { loading: false, failed: false, items } } }));
      } catch {
        set((st) => ({ reqs: { ...st.reqs, [modId]: { loading: false, failed: true, items: [] } } }));
      }
    },

    installMissing: async (modId) => {
      const wanted = needed(get().reqs[modId]?.items ?? [], get().installed);
      for (const r of wanted) {
        await get().install(r.mod_id);
        if (get().premiumNeeded || !get().installed.includes(r.mod_id)) break;
      }
    },

    installWithRequirements: async (modId) => {
      const depth = new Map<number, number>([[modId, 0]]);
      let frontier = [modId];
      for (let level = 1; level <= 3 && frontier.length > 0 && depth.size < 16; level++) {
        const next: number[] = [];
        for (const id of frontier) {
          const items = await ipc.nexusRequirements(id).catch(() => [] as NexusRequirement[]);
          for (const r of needed(items, get().installed)) {
            if (!depth.has(r.mod_id) && depth.size < 16) {
              depth.set(r.mod_id, level);
              next.push(r.mod_id);
            }
          }
        }
        frontier = next;
      }
      const order = [...depth.entries()].sort((a, b) => b[1] - a[1]).map(([id]) => id);
      set({ chain: order.length > 1 ? { done: 0, total: order.length } : null });
      for (const [i, id] of order.entries()) {
        await get().install(id);
        if (get().premiumNeeded || !get().installed.includes(id)) break;
        set({ chain: order.length > 1 ? { done: i + 1, total: order.length } : null });
      }
      set({ chain: null });
    },

    setSort: (sort) => {
      set({ sort });
      refetch();
    },
    setQuery: (query) => {
      const sort = query.trim() === "" && get().sort === "relevance" ? "downloads" : get().sort;
      set({ query, sort });
      refetch();
    },
    setCategory: (category) => {
      set({ category });
      refetch();
    },
    setHideAdult: (hideAdult) => {
      set({ hideAdult });
      refetch();
    },
    setTag: (tag) => {
      set({ tag });
      refetch();
    },
  };
});
