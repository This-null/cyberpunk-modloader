import { ipc, type ModRecord, type NexusRequirement } from "./ipc";

const cache = new Map<number, Promise<NexusRequirement[]>>();

function requirementsOf(modId: number): Promise<NexusRequirement[]> {
  let hit = cache.get(modId);
  if (!hit) {
    hit = ipc.nexusRequirements(modId).catch(() => {
      cache.delete(modId);
      return [] as NexusRequirement[];
    });
    cache.set(modId, hit);
  }
  return hit;
}

export function forgetRequirements() {
  cache.clear();
}

export async function dependentsOf(mod: ModRecord, mods: ModRecord[]): Promise<string[]> {
  const id = mod.nexus_id;
  if (!id) return [];
  const others = mods.filter((m) => m.name !== mod.name && m.nexus_id && m.nexus_id !== id).slice(0, 40);
  const lists = await Promise.all(others.map(async (m) => ({ m, reqs: await requirementsOf(m.nexus_id!) })));
  return lists
    .filter(({ reqs }) => reqs.some((r) => r.mod_id === id && !r.external && !/optional/i.test(r.notes)))
    .map(({ m }) => m.name);
}
