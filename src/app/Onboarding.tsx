import { useEffect } from "react";
import { ArrowLeft, ArrowRight } from "lucide-react";
import clsx from "clsx";
import { useT, type MessageKey } from "../i18n";
import { TOUR_STEPS, useOnboarding } from "../stores/onboarding";
import { SCENES } from "./TourScenes";

export function Onboarding() {
  const t = useT();
  const open = useOnboarding((s) => s.open);
  const step = useOnboarding((s) => s.step);
  const { go, finish } = useOnboarding.getState();

  useEffect(() => {
    if (!open) return;
    function onKey(e: KeyboardEvent) {
      const s = useOnboarding.getState();
      if (e.key === "ArrowRight") {
        if (s.step < TOUR_STEPS.length - 1) s.go(s.step + 1);
        else s.finish();
      } else if (e.key === "ArrowLeft" && s.step > 0) s.go(s.step - 1);
      else if (e.key === "Escape") s.finish();
    }
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [open]);

  if (!open) return null;
  const last = step === TOUR_STEPS.length - 1;
  const current = TOUR_STEPS[step];
  const Scene = SCENES[step];

  return (
    <div className="fixed inset-0 z-50 grid place-items-center bg-black/65 p-6 anim-fade" role="dialog" aria-modal="true">
      <div className="anim-pop flex w-full max-w-lg flex-col gap-5 border border-line bg-surface p-7">
        <div key={step} className="anim-rise flex flex-col gap-4">
          <Scene />
          <h2 className="text-xl font-semibold">{t(current.title as MessageKey)}</h2>
          <p className="text-muted">{t(current.body as MessageKey)}</p>
        </div>
        <div className="flex items-center justify-center gap-2">
          {TOUR_STEPS.map((_, i) => (
            <button key={i} tabIndex={-1} onClick={() => go(i)} className={clsx("h-1.5 transition-all", i === step ? "w-6 bg-accent" : "w-1.5 bg-line hover:bg-muted")} />
          ))}
        </div>
        <div className="flex items-center justify-between gap-2">
          <button onClick={finish} className="text-muted hover:text-ink">
            {t("onboard.skip")}
          </button>
          <div className="flex items-center gap-2">
            {step > 0 && (
              <button onClick={() => go(step - 1)} className="btn">
                <ArrowLeft size={15} />
                {t("onboard.back")}
              </button>
            )}
            <button onClick={() => (last ? finish() : go(step + 1))} className="btn btn-primary">
              {last ? t("onboard.done") : t("onboard.next")}
              {!last && <ArrowRight size={15} />}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
