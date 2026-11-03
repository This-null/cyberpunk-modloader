import { useEffect, useState } from "react";
import { Check, Settings2, X } from "lucide-react";
import { Dropdown } from "../../components/Dropdown";
import { useT } from "../../i18n";
import { useMods } from "../../stores/mods";
import { useModProfiles } from "../../stores/modProfiles";
import { useMyPresets } from "../../stores/presets";

const NEW = "\u0000new";

export function ProfileSwitcher({ disabled }: { disabled: boolean }) {
  const t = useT();
  const list = useMyPresets((s) => s.list);
  const active = useModProfiles((s) => s.active);
  const busy = useMods((s) => s.busy);
  const [creating, setCreating] = useState(false);
  const [name, setName] = useState("");

  useEffect(() => {
    void useMyPresets.getState().load();
  }, []);

  const current = active && list.some((p) => p.name === active) ? active : "";
  const taken = list.some((p) => p.name.toLowerCase() === name.trim().toLowerCase());
  const options = [
    ...(current === "" ? [{ value: "", label: t("profiles.none") }] : []),
    ...list.map((p) => ({ value: p.name, label: p.name })),
    { value: NEW, label: `+ ${t("profiles.new")}` },
  ];

  async function create() {
    const trimmed = name.trim();
    if (trimmed === "" || taken) return;
    setCreating(false);
    setName("");
    await useModProfiles.getState().create(trimmed);
  }

  if (creating) {
    return (
      <div className="flex items-center gap-2">
        <input
          autoFocus
          value={name}
          maxLength={60}
          onChange={(e) => setName(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") void create();
            if (e.key === "Escape") setCreating(false);
          }}
          placeholder={t("profiles.name")}
          title={t("profiles.newHint")}
          className="field w-44"
        />
        <button disabled={name.trim() === "" || taken} onClick={() => void create()} className="btn btn-primary disabled:opacity-60">
          <Check size={15} />
          {t("profiles.create")}
        </button>
        <button aria-label={t("action.cancel")} onClick={() => setCreating(false)} className="btn">
          <X size={15} />
        </button>
        <span className="max-w-56 text-xs text-muted">{t("profiles.newHint")}</span>
      </div>
    );
  }

  return (
    <div className="flex items-center gap-2">
      <Dropdown
        value={current}
        options={options}
        ariaLabel={t("profiles.label")}
        className={disabled || busy ? "pointer-events-none opacity-60" : ""}
        onChange={(v) => {
          if (v === NEW) setCreating(true);
          else if (v !== "") void useModProfiles.getState().switchTo(v);
        }}
      />
      <button
        disabled={disabled || busy}
        aria-label={t("profiles.manage")}
        title={t("profiles.manage")}
        onClick={() => void useMyPresets.getState().show()}
        className="btn"
      >
        <Settings2 size={16} />
      </button>
    </div>
  );
}
