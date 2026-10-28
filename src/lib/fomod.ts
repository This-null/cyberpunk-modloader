import type { FomodEvaluation, FomodSelection, FomodView } from "./ipc";

const lower = (s: string) => s.toLowerCase();

export function normalize(view: FomodView, evaluation: FomodEvaluation, selection: FomodSelection): FomodSelection {
  return view.steps.map((step, si) =>
    step.groups.map((group, gi) => {
      const types = evaluation.types[si]?.[gi] ?? [];
      const usable = group.plugins.map((_, pi) => pi).filter((pi) => lower(types[pi] ?? "") !== "notusable");
      const forced = usable.filter((pi) => lower(types[pi] ?? "") === "required");
      const chosen = (selection[si]?.[gi] ?? []).filter((pi) => usable.includes(pi));
      const kind = lower(group.kind);
      let out = [...new Set([...chosen, ...forced])];
      if (kind === "selectall") return usable;
      if (kind === "selectexactlyone" || kind === "selectatmostone") {
        if (out.length > 1) out = [out.find((pi) => forced.includes(pi)) ?? out[out.length - 1]];
      }
      if ((kind === "selectexactlyone" || kind === "selectatleastone") && out.length === 0 && usable.length > 0) {
        out = [usable.find((pi) => lower(types[pi] ?? "") === "recommended") ?? usable[0]];
      }
      return out.sort((a, b) => a - b);
    }),
  );
}

export function initialSelection(view: FomodView, evaluation: FomodEvaluation): FomodSelection {
  const picked: FomodSelection = view.steps.map((step, si) =>
    step.groups.map((_, gi) => (evaluation.types[si]?.[gi] ?? []).map((t, pi) => (lower(t) === "recommended" || lower(t) === "required" ? pi : -1)).filter((pi) => pi >= 0)),
  );
  return normalize(view, evaluation, picked);
}

export function toggled(kind: string, current: number[], pi: number): number[] {
  const k = lower(kind);
  const has = current.includes(pi);
  if (k === "selectexactlyone") return [pi];
  if (k === "selectatmostone") return has ? [] : [pi];
  return has ? current.filter((x) => x !== pi) : [...current, pi];
}

export function displayOrder(names: string[], order: string): number[] {
  const idx = names.map((_, i) => i);
  const o = order.toLowerCase();
  if (o !== "ascending" && o !== "descending") return idx;
  const key = (i: number) => names[i].toLowerCase();
  idx.sort((a, b) => (key(a) < key(b) ? -1 : key(a) > key(b) ? 1 : a - b));
  return o === "descending" ? idx.reverse() : idx;
}
