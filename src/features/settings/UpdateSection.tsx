import { useT } from "../../i18n";
import { useSettings } from "../../stores/settings";
import { useUpdate } from "../../stores/update";
import { Segmented } from "./Segmented";

export function UpdateSection() {
  const t = useT();
  const updates = useSettings((s) => s.updates);
  const setUpdates = useSettings((s) => s.setUpdates);
  const status = useUpdate((s) => s.status);
  const version = useUpdate((s) => s.version);
  const check = useUpdate((s) => s.check);
  const install = useUpdate((s) => s.install);

  return (
    <section className="flex flex-col gap-3 rounded-xl border border-line bg-surface p-5">
      <h2 className="text-base font-semibold">{t("update.title")}</h2>
      <Segmented value={updates} options={[{ value: "on", label: t("update.optOn") }, { value: "off", label: t("update.optOff") }]} onChange={setUpdates} />
      <div className="flex flex-wrap items-center gap-3">
        <button
          disabled={status === "checking" || status === "downloading"}
          onClick={() => void check(true)}
          className="btn disabled:opacity-60"
        >
          {status === "checking" ? t("update.checking") : t("update.check")}
        </button>
        {status === "none" && <span className="text-ok">{t("update.upToDate")}</span>}
        {status === "failed" && <span className="text-danger">{t("update.failed")}</span>}
        {status === "available" && (
          <>
            <span>{t("update.available", { version })}</span>
            <button onClick={() => void install()} className="btn btn-primary">
              {t("update.now")}
            </button>
          </>
        )}
      </div>
      <p className="text-xs text-muted">{t("update.hint")}</p>
    </section>
  );
}
