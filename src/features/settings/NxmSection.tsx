import { useEffect, useState } from "react";
import { Link2, Unlink } from "lucide-react";
import { useT } from "../../i18n";
import { ipc, type NxmHandler } from "../../lib/ipc";
import { useErrors } from "../../stores/errors";

export function NxmSection() {
  const t = useT();
  const [state, setState] = useState<NxmHandler | null>(null);

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
    <section className="flex flex-col gap-3 rounded-xl border border-line bg-surface p-5">
      <h2 className="text-base font-semibold">{t("nxm.title")}</h2>
      <p className="text-muted">{t("nxm.intro")}</p>
      <div className="flex flex-wrap items-center gap-3">
        {state === "ours" ? (
          <>
            <span className="text-ok">{t("nxm.on")}</span>
            <button onClick={() => void toggle(false)} className="btn">
              <Unlink size={15} />
              {t("nxm.disable")}
            </button>
          </>
        ) : (
          <>
            {state === "other" && <span className="text-muted">{t("nxm.other")}</span>}
            <button onClick={() => void toggle(true)} className="btn btn-primary">
              <Link2 size={15} />
              {state === "other" ? t("nxm.replace") : t("nxm.enable")}
            </button>
          </>
        )}
      </div>
    </section>
  );
}
