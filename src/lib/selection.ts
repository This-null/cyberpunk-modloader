export const modKey = (name: string) => `m:${name}`;
export const extKey = (name: string) => `e:${name}`;

export function toggleKey(selected: readonly string[], key: string): string[] {
  return selected.includes(key) ? selected.filter((k) => k !== key) : [...selected, key];
}

export function addKeys(selected: readonly string[], keys: readonly string[]): string[] {
  return [...new Set([...selected, ...keys])];
}

export function rangeKeys(order: readonly string[], from: string | null, to: string): string[] {
  const b = order.indexOf(to);
  if (b < 0) return [];
  const a = from === null ? -1 : order.indexOf(from);
  if (a < 0) return [to];
  return order.slice(Math.min(a, b), Math.max(a, b) + 1);
}

export function pruneSelection(selected: readonly string[], valid: readonly string[]): string[] {
  const known = new Set(valid);
  return selected.filter((k) => known.has(k));
}

export function matchesQuery(parts: readonly string[], query: string): boolean {
  const q = query.trim().toLowerCase();
  return q === "" || parts.some((p) => p.toLowerCase().includes(q));
}

export function namesWith(selected: readonly string[], prefix: "m:" | "e:"): string[] {
  return selected.filter((k) => k.startsWith(prefix)).map((k) => k.slice(prefix.length));
}
