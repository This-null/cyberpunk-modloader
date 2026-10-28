import { AlertTriangle } from "lucide-react";
import { useT } from "../../i18n";
import { missingFrameworks } from "../../lib/frameworks";
import { useFrameworks } from "../../stores/frameworks";
import { useMods } from "../../stores/mods";
import { useNav } from "../../stores/nav";
import { useNexusBrowse } from "../../stores/nexusBrowse";

export function FrameworkBanner() {
  const t = useT();
  const present = useFrameworks((s) => s.present);
  const mods = useMods((s) => s.mods);
  const missing = missingFrameworks(present, mods);
  if (missing.length === 0) return null;

  return (
    <div className="anim-rise flex flex-col gap-2 rounded-lg border border-danger/50 bg-danger/10 p-3">
      {missing.map(({ framework, mods: users }) => (
        <div key={framework.id} className="flex flex-wrap items-center gap-3">
          <AlertTriangle size={16} className="shrink-0 text-danger" />
          <span className="min-w-0 flex-1 text-ink">{t("fw.missing", { name: framework.name, mods: users.join(", ") })}</span>
          <button
            className="btn btn-sm"
            onClick={() => {
              useNexusBrowse.getState().setQuery(framework.name);
              useNav.getState().setPage("nexus");
            }}
          >
            {t("fw.find")}
          </button>
        </div>
      ))}
    </div>
  );
}
