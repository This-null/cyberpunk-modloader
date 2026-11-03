import { useEffect, useState } from "react";
import { useT } from "../../i18n";
import { ipc, type NxmHandler } from "../../lib/ipc";
import { Switch } from "../../components/Switch";
import { useErrors } from "../../stores/errors";
import { useAutoInstall } from "../../stores/autoInstall";
import { useNexusWindow } from "../../stores/nexusWindow";

export function NxmSection() {
  const t = useT();
  const [state, setState] = useState<NxmHandler | null>(null);
  const auto = useAutoInstall((s) => s.on);
  const setAuto = useAutoInstall((s) => s.set);
  const inside = useNexusWindow((s) => s.inside);
  const setInside = useNexusWindow((s) => s.set);

  useEffect(() => {
    void ipc.nxmStatus().then(setState).catch(() => setState(null));
  }, []);

  async function toggle(enable: boolean) {
    try {
      setState(await ipc.nxmRegister(enable));
    } catch (e) {
      useErrors.getState().report(e);
    }
  }

  if (state === null) return null;
  return (
    <section className="flex flex-col gap-2 rounded-xl border border-line bg-surface p-5">
      <h2 className="mb-1 text-base font-semibold">{t("nxm.title")}</h2>
      <Switch
        on={state === "ours"}
        onChange={(on) => void toggle(on)}
        label={t("nxm.handle")}
        hint={state === "other" ? t("nxm.other") : t("nxm.handleHint")}
      />
      <Switch on={inside} onChange={setInside} label={t("nxm.inside")} hint={t("nxm.insideHint")} />
      <Switch on={auto} onChange={setAuto} label={t("nxm.auto")} hint={t("nxm.autoHint")} />
      <p className="text-xs text-muted">{t("nxm.browserHint")}</p>
    </section>
  );
}
