import { listen } from "@tauri-apps/api/event";
import { create } from "zustand";
import { ipc, toAppError, type NexusCategory, type NexusMod } from "../lib/ipc";
import { categoryNames, SEARCH_PAGE, type Sort } from "../lib/browse";
import { useErrors } from "./errors";
import { useMods } from "./mods";

interface BrowseState {
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
