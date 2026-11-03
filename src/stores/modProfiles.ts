import { create } from "zustand";
import { translate } from "../i18n";
import { ipc } from "../lib/ipc";
import { useErrors } from "./errors";
import { useMods } from "./mods";
import { useMyPresets } from "./presets";

const KEY = "cp2077-active-profile";

function load(): string | null {
  try {
    return localStorage.getItem(KEY);
  } catch {
    return null;
  }
}

function remember(name: string | null) {
  try {
    if (name) localStorage.setItem(KEY, name);
    else localStorage.removeItem(KEY);
  } catch {
    // the choice then lasts for this session only
  }
}

const enabledNow = () =>
  useMods
    .getState()
    .mods.filter((m) => m.enabled)
    .map((m) => m.name);

const has = (list: { name: string }[], name: string) => list.some((p) => p.name.toLowerCase() === name.toLowerCase());

interface ProfilesState {
  active: string | null;
  switchTo: (name: string) => Promise<void>;
  create: (name: string) => Promise<void>;
}

export const useModProfiles = create<ProfilesState>((set, get) => {
  async function run(action: () => Promise<string | null>) {
    const mods = useMods.getState();
    mods.begin();
    try {
      const name = await action();
      remember(name);
      set({ active: name });
      useMyPresets.setState({ list: await ipc.presetsList() });
    } catch (e) {
      useErrors.getState().report(e);
    } finally {
      await mods.refresh();
      mods.end();
    }
  }

  useMyPresets.subscribe((s, prev) => {
    if (s.applied && s.applied !== prev.applied) {
      remember(s.applied);
      set({ active: s.applied });
    } else if (s.list !== prev.list) {
      const a = get().active;
      if (a && !has(s.list, a)) {
        remember(null);
        set({ active: null });
      }
    }
  });

  return {
    active: load(),

    switchTo: (name) =>
      run(async () => {
        const active = get().active;
        const listed = has(await ipc.presetsList(), active ?? "");
        if (active && listed && useMods.getState().mods.length > 0 && active.toLowerCase() !== name.toLowerCase()) {
          await ipc.presetSave(active, enabledNow());
        }
        const report = await ipc.presetApply(name);
        if (report.failed.length > 0 || report.missing.length > 0) {
          useMyPresets.setState({ report, applied: name, open: true });
          return null;
        }
        return name;
      }),

    create: (name) =>
      run(async () => {
        const list = await ipc.presetsList();
        if (has(list, name)) throw new Error(name);
        let from = get().active;
        if (!from || !has(list, from)) {
          let first = translate("profiles.first");
          for (let n = 2; has(list, first); n += 1) first = `${translate("profiles.first")} (${n})`;
          from = first;
        }
        await ipc.presetSave(from, enabledNow());
        await ipc.presetSave(name, []);
        await ipc.presetApply(name);
        return name;
      }),
  };
});
