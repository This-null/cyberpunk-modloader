import { create } from "zustand";
import { initialSelection, normalize, toggled } from "../lib/fomod";
import { ipc, type FomodEvaluation, type FomodSelection, type FomodView } from "../lib/ipc";
import { setFomodHandler, useErrors } from "./errors";
import { useMods } from "./mods";

interface FomodState {
  open: boolean;
  name: string;
  view: FomodView | null;
  selection: FomodSelection;
  evaluation: FomodEvaluation | null;
  working: boolean;
  start: () => Promise<void>;
  choose: (step: number, group: number, plugin: number) => Promise<void>;
  install: () => Promise<void>;
  cancel: () => Promise<void>;
}

async function settle(view: FomodView, selection: FomodSelection): Promise<{ selection: FomodSelection; evaluation: FomodEvaluation }> {
  let current = selection;
  let evaluation = await ipc.fomodEval(current);
  for (let round = 0; round < 3; round++) {
    const next = normalize(view, evaluation, current);
    if (JSON.stringify(next) === JSON.stringify(current)) break;
    current = next;
    evaluation = await ipc.fomodEval(current);
  }
  return { selection: current, evaluation };
}

export const useFomod = create<FomodState>((set, get) => ({
  open: false,
  name: "",
  view: null,
  selection: [],
  evaluation: null,
  working: false,

  start: async () => {
    try {
      const pending = await ipc.fomodPending();
      const first = await ipc.fomodEval([]);
      const chosen = await settle(pending.view, initialSelection(pending.view, first));
      set({ open: true, name: pending.name, view: pending.view, ...chosen });
    } catch (e) {
      useErrors.getState().report(e);
    }
  },

  choose: async (step, group, plugin) => {
    const { view, selection } = get();
    if (!view) return;
    const next = selection.map((s) => s.map((g) => [...g]));
    next[step][group] = toggled(view.steps[step].groups[group].kind, next[step][group] ?? [], plugin);
    try {
      set(await settle(view, next));
    } catch (e) {
      useErrors.getState().report(e);
    }
  },

  install: async () => {
    if (get().working) return;
    set({ working: true });
    try {
      const done = await ipc.fomodInstall(get().selection);
      await useMods.getState().refresh();
      useMods.setState({ skipped: done.skipped, ignored: done.ignored ?? [] });
      set({ open: false, view: null, evaluation: null });
    } catch (e) {
      useErrors.getState().report(e);
    } finally {
      set({ working: false });
    }
  },

  cancel: async () => {
    set({ open: false, view: null, evaluation: null });
    await ipc.fomodCancel().catch(() => {});
  },
}));

setFomodHandler(() => void useFomod.getState().start());
