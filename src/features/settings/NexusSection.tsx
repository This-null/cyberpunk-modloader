import { useState } from "react";
import { useT } from "../../i18n";
import { ipc } from "../../lib/ipc";
import { NEXUS_DEV, NEXUS_PASTE_KEY } from "../../lib/flags";
import { errorMessageKey, useErrors } from "../../stores/errors";
import { useNexus } from "../../stores/nexus";

export function NexusSection() {
  const t = useT();
  const status = useNexus((s) => s.status);
  const connecting = useNexus((s) => s.connecting);
  const connectError = useNexus((s) => s.connectError);
  const connect = useNexus((s) => s.connect);
  const disconnect = useNexus((s) => s.disconnect);
  const [key, setKey] = useState("");
  const [probe, setProbe] = useState("");
  const errorKey = connectError ? errorMessageKey(connectError) : null;

  async function submit() {
    if (await connect(key)) setKey("");
  }

  return (
    <section className="flex flex-col gap-3 rounded-xl border border-line bg-surface p-5">
      <h2 className="text-base font-semibold">{t("nexus.title")}</h2>
      {status?.connected ? (
        <div className="flex flex-wrap items-center gap-3">
          <span>{t("nexus.connectedAs", { name: status.user?.name ?? "?" })}</span>
          {status.user?.is_premium && (
            <span className="rounded bg-accent px-1.5 py-0.5 text-xs font-medium text-accent-ink">{t("nexus.premium")}</span>
          )}
          {status.user?.is_supporter && (
            <span className="rounded border border-line px-1.5 py-0.5 text-xs text-muted">{t("nexus.supporter")}</span>
          )}
          <button onClick={() => void disconnect()} className="rounded border border-line px-4 py-2 hover:bg-surface-2">
            {t("nexus.disconnect")}
          </button>
          {NEXUS_DEV && (
            <>
              <button
                onClick={async () => {
                  try {
                    const size = await ipc.nexusProbe();
                    setProbe(t("nexus.probeDone", { size: Math.round(size / 1024) }));
                  } catch (e) {
                    useErrors.getState().report(e);
                  }
                }}
                className="rounded border border-line px-4 py-2 hover:bg-surface-2"
              >
                {t("nexus.probe")}
              </button>
              {probe && <span className="text-ok">{probe}</span>}
            </>
          )}
        </div>
      ) : (
        <>
          <p className="text-muted">{t("nexus.intro")}</p>
          {NEXUS_PASTE_KEY && (
            <>
              <ol className="flex list-decimal flex-col gap-1 pl-5 text-muted">
                <li>{t("nexus.step1")}</li>
                <li>{t("nexus.step2")}</li>
                <li>{t("nexus.step3")}</li>
              </ol>
              <div>
                <button
                  onClick={() => void ipc.openUrl("https://www.nexusmods.com/users/myaccount?tab=api")}
                  className="rounded border border-line px-4 py-2 hover:bg-surface-2"
                >
                  {t("nexus.openPage")}
                </button>
              </div>
              <div className="flex flex-wrap items-center gap-2">
                <input
                  type="password"
                  autoComplete="off"
                  spellCheck={false}
                  value={key}
                  onChange={(e) => setKey(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter" && key.trim() && !connecting) void submit();
                  }}
                  placeholder={t("nexus.keyLabel")}
                  aria-label={t("nexus.keyLabel")}
                  className="min-w-64 flex-1 rounded border border-line bg-surface px-3 py-2"
                />
                <button
                  disabled={!key.trim() || connecting}
                  onClick={() => void submit()}
                  className="rounded bg-accent px-4 py-2 font-medium text-accent-ink"
                >
                  {connecting ? t("nexus.connecting") : t("nexus.connect")}
                </button>
              </div>
              {errorKey && <p className="text-danger">{t(errorKey)}</p>}
              <p className="max-w-xl text-xs text-muted">{t("nexus.testNote")}</p>
            </>
          )}
        </>
      )}
    </section>
  );
}
