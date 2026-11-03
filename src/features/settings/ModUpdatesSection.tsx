import { useT } from "../../i18n";
import { useNexus } from "../../stores/nexus";
import { useSettings } from "../../stores/settings";
import { Segmented } from "./Segmented";

export function ModUpdatesSection() {
  const t = useT();
  const modUpdates = useSettings((s) => s.modUpdates);
  const setModUpdates = useSettings((s) => s.setModUpdates);
  const premium = useNexus((s) => s.status?.user?.is_premium ?? false);

  return (
    <section className="flex flex-col gap-3 rounded-xl border border-line bg-surface p-5">
      <h2 className="text-base font-semibold">{t("modupd.title")}</h2>
      <Segmented value={modUpdates} options={[{ value: "manual", label: t("modupd.manual") }, { value: "auto", label: t("modupd.auto"), disabled: !premium }]} onChange={setModUpdates} />
      <p className="text-xs text-muted">{premium ? t("modupd.hint") : t("modupd.needPremium")}</p>
    </section>
  );
}
