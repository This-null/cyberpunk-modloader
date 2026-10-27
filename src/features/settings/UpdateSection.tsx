import clsx from "clsx";
import { useT } from "../../i18n";
import { useSettings } from "../../stores/settings";
import { useUpdate } from "../../stores/update";

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
      <div className="flex w-fit overflow-hidden rounded border border-line">
        {(["on", "off"] as const).map((v) => (
          <button
            key={v}
            onClick={() => setUpdates(v)}
            aria-pressed={updates === v}
            className={clsx(
              "px-4 py-2",
              updates === v ? "bg-accent font-medium text-accent-ink" : "bg-surface hover:bg-surface-2",
            )}
          >
            {v === "on" ? t("update.optOn") : t("update.optOff")}
          </button>
        ))}
      </div>
      <div className="flex flex-wrap items-center gap-3">
        <button
          disabled={status === "checking" || status === "downloading"}
          onClick={() => void check(true)}
          className="rounded border border-line px-4 py-2 hover:bg-surface-2 disabled:opacity-60"
        >
          {status === "checking" ? t("update.checking") : t("update.check")}
        </button>
        {status === "none" && <span className="text-ok">{t("update.upToDate")}</span>}
        {status === "failed" && <span className="text-danger">{t("update.failed")}</span>}
        {status === "available" && (
          <>
            <span>{t("update.available", { version })}</span>
            <button onClick={() => void install()} className="rounded bg-accent px-4 py-2 font-medium text-accent-ink">
              {t("update.now")}
            </button>
          </>
        )}
      </div>
      <p className="text-xs text-muted">{t("update.hint")}</p>
    </section>
  );
}
