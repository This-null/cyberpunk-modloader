import type { ModRecord } from "./ipc";

export interface Overwrite {
  by: string;
  owner: string | null;
  files: string[];
}

export function overwrites(mods: readonly ModRecord[]): Overwrite[] {
  const groups = new Map<string, Overwrite>();
  for (const mod of mods) {
    if (!mod.enabled) continue;
    for (const b of mod.backups ?? []) {
      const key = `${mod.name}\u0000${b.owner ?? ""}`;
      const found = groups.get(key) ?? { by: mod.name, owner: b.owner, files: [] };
      found.files.push(b.file);
      groups.set(key, found);
    }
  }
  return [...groups.values()].sort((a, b) => b.files.length - a.files.length || a.by.localeCompare(b.by));
}
