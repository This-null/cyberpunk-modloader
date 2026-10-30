import { useEffect, useState } from "react";
import { open, save } from "@tauri-apps/plugin-dialog";
import { CoverPlaceholder } from "../../brand/CoverPlaceholder";
import { useT } from "../../i18n";
import { ipc } from "../../lib/ipc";
import { errorMessageKey } from "../../stores/errors";
import { useMods } from "../../stores/mods";
import { useNexus } from "../../stores/nexus";

export function ModDetails() {
  const t = useT();
  const detail = useMods((s) => s.detail);
  const mod = useMods((s) => s.mods.find((m) => m.name === s.detail));
  const image = useMods((s) => (s.detail ? s.images[s.detail] : null));
  const busy = useMods((s) => s.busy);
  const openDetail = useMods((s) => s.openDetail);
  const saveInfo = useMods((s) => s.saveInfo);
  const exportOne = useMods((s) => s.exportOne);
  const [description, setDescription] = useState("");
  const [version, setVersion] = useState("");
  const [picked, setPicked] = useState<string | null>(null);
  const [target, setTarget] = useState("");
  const [linkError, setLinkError] = useState<string | null>(null);
  const link = useNexus((s) => s.link);
  const unlink = useNexus((s) => s.unlink);

  useEffect(() => {
    setDescription(mod?.description ?? "");
    setVersion(mod?.version ?? "");
    setPicked(null);
    setTarget("");
    setLinkError(null);
  }, [detail]);

  if (!detail || !mod) return null;

  async function chooseImage() {
    const chosen = await open({ filters: [{ name: "Image", extensions: ["png", "jpg", "jpeg", "webp"] }] });
    if (typeof chosen === "string") setPicked(chosen);
  }

  async function exportMod() {
    const dest = await save({ defaultPath: `${detail}.zip`, filters: [{ name: "ZIP", extensions: ["zip"] }] });
    if (dest) await exportOne(detail!, dest);
  }

  const dirty = description !== mod.description || version !== mod.version || picked !== null;

  return (
    <div className="fixed inset-0 z-40 grid place-items-center bg-black/55 p-6 anim-fade" role="dialog" aria-modal="true">
      <div className="anim-pop flex max-h-[calc(100vh-3rem)] w-full max-w-2xl flex-col gap-4 overflow-auto rounded-lg border border-line bg-surface p-6">
        <div className="grid h-52 w-full place-items-center overflow-hidden rounded-md bg-surface-2">
          {image ? (
            <img src={image} alt="" className="h-full w-full object-cover" draggable={false} />
          ) : (
            <CoverPlaceholder name={mod.name} size="lg" />
          )}
        </div>
        <h2 className="text-lg font-semibold">{mod.name}</h2>

        <div className="flex flex-wrap items-center gap-2">
          <button onClick={chooseImage} className="btn">
            {t("action.changeImage")}
          </button>
          {picked && <span className="truncate text-xs text-muted">{picked.split(/[\\/]/).pop()}</span>}
        </div>

        <label className="flex flex-col gap-1">
          <span className="text-muted">{t("mods.version")}</span>
          <input
            value={version}
            onChange={(e) => setVersion(e.target.value)}
            maxLength={40}
            className="w-48 rounded border border-line bg-surface-2 px-3 py-2"
          />
        </label>

        <label className="flex flex-col gap-1">
          <span className="text-muted">{t("mods.description")}</span>
          <textarea
            value={description}
            onChange={(e) => setDescription(e.target.value)}
            rows={5}
            maxLength={4000}
            className="resize-y rounded border border-line bg-surface-2 p-3"
          />
        </label>

        <div className="flex flex-col gap-2">
          <span className="text-muted">{t("nexus.pageLabel")}</span>
          {mod.nexus_id ? (
            <div className="flex flex-wrap items-center gap-2">
              <span>{t("nexus.linked", { id: mod.nexus_id })}</span>
              <button
                onClick={() => void ipc.openUrl(`https://www.nexusmods.com/cyberpunk2077/mods/${mod.nexus_id}`)}
                className="btn"
              >
                {t("nexus.openOnNexus")}
              </button>
              <button onClick={() => void unlink(mod.name)} className="btn">
                {t("nexus.unlink")}
              </button>
            </div>
          ) : (
            <div className="flex gap-2">
              <input
                value={target}
                onChange={(e) => {
                  setTarget(e.target.value);
                  setLinkError(null);
                }}
                placeholder={t("nexus.pagePlaceholder")}
                aria-label={t("nexus.pageLabel")}
                className="min-w-0 flex-1 rounded border border-line bg-surface-2 px-3 py-2"
              />
              <button
                disabled={!target.trim()}
                onClick={async () => {
                  const code = await link(mod.name, target);
                  setLinkError(code);
                  if (!code) setTarget("");
                }}
                className="btn"
              >
                {t("nexus.link")}
              </button>
            </div>
          )}
          {linkError && errorMessageKey(linkError) && <p className="text-danger">{t(errorMessageKey(linkError)!)}</p>}
        </div>

        <details className="text-muted">
          <summary className="cursor-pointer">
            {t("mods.fileList")} ({mod.files.length})
          </summary>
          <ul className="mt-2 max-h-40 overflow-auto rounded bg-surface-2 p-3 text-xs">
            {mod.files.map((f) => (
              <li key={f} className="break-all py-0.5">
                {f}
              </li>
            ))}
          </ul>
        </details>

        <p className="text-xs text-muted">{t("export.notice")}</p>

        <div className="flex flex-wrap justify-end gap-2">
          <button disabled={busy} onClick={exportMod} className="btn">
            {t("action.export")}
          </button>
          <button
            disabled={busy || !dirty}
            onClick={() => saveInfo(mod.name, description, version, picked)}
            className="btn btn-primary"
          >
            {t("action.save")}
          </button>
          <button onClick={() => openDetail(null)} className="btn">
            {t("action.close")}
          </button>
        </div>
      </div>
    </div>
  );
}
