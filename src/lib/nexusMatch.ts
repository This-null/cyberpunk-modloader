export type Grade = "exact" | "likely";

export function cleanName(name: string): string {
  return name
    .toLowerCase()
    .replace(/\([^)]*\)/g, " ")
    .replace(/\bfor\s+\d+(\.\d+)*\b/g, " ")
    .replace(/\bv?\d+(\.\d+)+[\w-]*/g, " ")
    .replace(/\bv\d+\b/g, " ")
    .replace(/[^a-z0-9]+/g, " ")
    .trim();
}

function tokens(text: string): Set<string> {
  return new Set(text.split(" ").filter((w) => w.length > 0));
}

export function grade(local: string, title: string): Grade | null {
  const a = cleanName(local);
  const b = cleanName(title);
  if (a === "" || b === "") return null;
  if (a === b) return "exact";
  const ta = tokens(a);
  const tb = tokens(b);
  if (ta.size < 2 || tb.size < 2) return null;
  let shared = 0;
  for (const w of ta) if (tb.has(w)) shared += 1;
  const union = ta.size + tb.size - shared;
  return shared / union >= 0.75 ? "likely" : null;
}

export function bestMatch<T extends { name: string | null }>(local: string, results: T[]): { item: T; grade: Grade } | null {
  let likely: { item: T; grade: Grade } | null = null;
  for (const item of results) {
    const g = grade(local, item.name ?? "");
    if (g === "exact") return { item, grade: g };
    if (g === "likely" && likely === null) likely = { item, grade: g };
  }
  return likely;
}
