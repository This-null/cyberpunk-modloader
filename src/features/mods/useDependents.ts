import { useEffect, useState } from "react";
import { dependentsOf } from "../../lib/dependents";
import type { ModRecord } from "../../lib/ipc";
import { useMods } from "../../stores/mods";

export function useDependents(mod: ModRecord, active: boolean): string[] {
  const [names, setNames] = useState<string[]>([]);
  useEffect(() => {
    if (!active) {
      setNames([]);
      return;
    }
    let live = true;
    void dependentsOf(mod, useMods.getState().mods).then((d) => live && setNames(d));
    return () => {
      live = false;
    };
  }, [active, mod]);
  return names;
}
