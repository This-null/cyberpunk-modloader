import { create } from "zustand";
import { translate } from "../i18n";
import { ipc } from "../lib/ipc";
import { downloadPageUrl } from "../lib/nexusPage";

const KEY = "cp2077-nexus-inside";

function load(): boolean {
  try {
    return localStorage.getItem(KEY) !== "0";
  } catch {
    return true;
  }
}

interface NexusWindowState {
  inside: boolean;
  set: (inside: boolean) => void;
}

export const useNexusWindow = create<NexusWindowState>((set) => ({
  inside: load(),
  set: (inside) => {
    try {
      localStorage.setItem(KEY, inside ? "1" : "0");
    } catch {
      // the choice then lasts for this session only
    }
    set({ inside });
  },
}));

export function openNexus(url: string, queue: string[] = []) {
  if (useNexusWindow.getState().inside) {
    const label = translate("nexusWin.next", { count: "{n}" });
    void ipc.openNexusWindow(url, queue, label).catch(() => void ipc.openUrl(url));
  } else {
    void ipc.openUrl(url);
  }
}

export async function missingPages(modId: number, installed: number[]): Promise<string[]> {
  const reqs = await ipc.nexusRequirements(modId).catch(() => []);
  const ids = reqs.filter((r) => !r.external && r.mod_id > 0 && !installed.includes(r.mod_id)).map((r) => r.mod_id);
  return [...new Set(ids)].map((id) => downloadPageUrl(id, null));
}
