import type { NexusCategory, NexusMod } from "./ipc";

export type Kind = "trending" | "latest_added" | "latest_updated";

export const KINDS: Kind[] = ["trending", "latest_added", "latest_updated"];

export type Sort = "downloads" | "endorsements" | "createdAt" | "updatedAt" | "relevance";
export const SORTS: Sort[] = ["downloads", "endorsements", "createdAt", "updatedAt"];
export const SEARCH_PAGE = 24;
export const PAGE_SIZE = 12;

export interface Filters {
  query: string;
  category: number;
  hideAdult: boolean;
}

export function categoryIds(categories: readonly NexusCategory[], id: number): number[] {
  if (id === 0) return [];
  const ids = new Set<number>([id]);
  let grew = true;
  while (grew) {
    grew = false;
    for (const c of categories) {
      if (c.parent !== null && ids.has(c.parent) && !ids.has(c.category_id)) {
        ids.add(c.category_id);
        grew = true;
      }
    }
  }
  return [...ids];
}

export interface CategoryOption {
  id: number;
  name: string;
  depth: number;
}

export function categoryOptions(categories: readonly NexusCategory[]): CategoryOption[] {
  const known = new Set(categories.map((c) => c.category_id));
  const byParent = new Map<number | null, NexusCategory[]>();
  for (const c of categories) {
    const parent = c.parent !== null && known.has(c.parent) && c.parent !== c.category_id ? c.parent : null;
    byParent.set(parent, [...(byParent.get(parent) ?? []), c]);
  }
  const out: CategoryOption[] = [];
  const seen = new Set<number>();
  const walk = (parent: number | null, depth: number) => {
    for (const c of [...(byParent.get(parent) ?? [])].sort((a, b) => a.name.localeCompare(b.name))) {
      if (seen.has(c.category_id)) continue;
      seen.add(c.category_id);
      out.push({ id: c.category_id, name: c.name, depth });
      walk(c.category_id, depth + 1);
    }
  };
  walk(null, 0);
  return out;
}

export function filterMods(
  items: readonly NexusMod[],
  categories: readonly NexusCategory[],
  filters: Filters,
): NexusMod[] {
  const q = filters.query.trim().toLowerCase();
  const wanted = categoryIds(categories, filters.category);
  return items.filter((m) => {
    if (filters.hideAdult && m.contains_adult_content) return false;
    if (wanted.length > 0 && !wanted.includes(m.category_id)) return false;
    if (q === "") return true;
    return [m.name ?? "", m.summary ?? "", m.author].some((s) => s.toLowerCase().includes(q));
  });
}

export function pageCount(total: number, size = PAGE_SIZE): number {
  return Math.max(1, Math.ceil(total / size));
}

export function paginate<T>(list: readonly T[], page: number, size = PAGE_SIZE): T[] {
  const safe = Math.min(Math.max(1, page), pageCount(list.length, size));
  return list.slice((safe - 1) * size, safe * size);
}

export function compactNumber(n: number, locale: string): string {
  try {
    return new Intl.NumberFormat(locale, { notation: "compact", maximumFractionDigits: 1 }).format(n);
  } catch {
    return String(n);
  }
}

export function categoryName(categories: readonly NexusCategory[], id: number): string {
  return categories.find((c) => c.category_id === id)?.name ?? "";
}

export function categoryNames(categories: readonly NexusCategory[], id: number): string[] {
  return categoryIds(categories, id)
    .map((i) => categoryName(categories, i))
    .filter((n, i, all) => n !== "" && all.indexOf(n) === i);
}
