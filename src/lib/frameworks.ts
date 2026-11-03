import type { ModRecord } from "./ipc";

export interface Framework {
  id: string;
  name: string;
  marker: string;
}

export const FRAMEWORKS: Framework[] = [
  { id: "cet", name: "Cyber Engine Tweaks", marker: "bin/x64/plugins/cyber_engine_tweaks.asi" },
  { id: "red4ext", name: "RED4ext", marker: "red4ext/red4ext.dll" },
  { id: "archivexl", name: "ArchiveXL", marker: "red4ext/plugins/archivexl/" },
  { id: "tweakxl", name: "TweakXL", marker: "red4ext/plugins/tweakxl/" },
  { id: "redscript", name: "redscript", marker: "engine/tools/scc.exe" },
  { id: "codeware", name: "Codeware", marker: "red4ext/plugins/codeware/" },
  { id: "audioware", name: "Audioware", marker: "red4ext/plugins/audioware/" },
  { id: "inputloader", name: "Input Loader", marker: "red4ext/plugins/input_loader/" },
];

function needs(files: string[]): Set<string> {
  const out = new Set<string>();
  for (const raw of files) {
    const f = raw.replace(/\\/g, "/").toLowerCase();
    if (f.endsWith(".xl")) out.add("archivexl");
    if (f.endsWith(".reds")) {
      out.add("redscript");
      out.add("red4ext");
    }
    if (f.startsWith("r6/tweaks/") && /\.(ya?ml|tweak)$/.test(f)) {
      out.add("tweakxl");
      out.add("red4ext");
    }
    if (f.includes("/cyber_engine_tweaks/mods/")) out.add("cet");
    if (f.startsWith("red4ext/plugins/")) out.add("red4ext");
    if (f.startsWith("r6/audioware/")) out.add("audioware");
    if (f.startsWith("r6/inputs/")) out.add("inputloader");
  }
  return out;
}

function provides(files: string[], fw: Framework): boolean {
  return files.some((raw) => {
    const f = raw.replace(/\\/g, "/").toLowerCase();
    return fw.marker.endsWith("/") ? f.startsWith(fw.marker) : f === fw.marker;
  });
}

export function neededFrameworks(mods: ModRecord[]): Record<string, string[]> {
  const out: Record<string, string[]> = {};
  for (const mod of mods) {
    if (!mod.enabled) continue;
    for (const id of needs(mod.files)) {
      const fw = FRAMEWORKS.find((x) => x.id === id);
      if (!fw || provides(mod.files, fw)) continue;
      (out[id] ??= []).push(mod.name);
    }
  }
  return out;
}

export function missingFrameworks(present: Record<string, boolean>, mods: ModRecord[]): { framework: Framework; mods: string[] }[] {
  const needed = neededFrameworks(mods);
  return FRAMEWORKS.filter((fw) => needed[fw.id] && present[fw.id] === false).map((framework) => ({ framework, mods: needed[framework.id] }));
}

export function isNewer(latest: string, installed: string): boolean {
  const parts = (v: string) => (v.match(/\d+/g) ?? []).map(Number);
  const a = parts(latest);
  const b = parts(installed);
  if (a.length === 0 || b.length === 0) return false;
  for (let i = 0; i < Math.max(a.length, b.length); i++) {
    const x = a[i] ?? 0;
    const y = b[i] ?? 0;
    if (x !== y) return x > y;
  }
  return false;
}
