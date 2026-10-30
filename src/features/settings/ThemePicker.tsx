import clsx from "clsx";
import { useT, type MessageKey } from "../../i18n";
import { useSettings, type Theme } from "../../stores/settings";

const options: { value: Theme; label: MessageKey; swatch: [string, string, string] }[] = [
  { value: "system", label: "theme.system", swatch: ["#17181b", "#f5f2ea", "#e0b84a"] },
  { value: "dark", label: "theme.dark", swatch: ["#17181b", "#27282d", "#e0b84a"] },
  { value: "light", label: "theme.light", swatch: ["#f5f2ea", "#ece8dd", "#c99a1e"] },
  { value: "midnight", label: "theme.midnight", swatch: ["#0e1626", "#1a2942", "#8ab4ff"] },
  { value: "ocean", label: "theme.ocean", swatch: ["#0f2429", "#1b3d44", "#e1b382"] },
  { value: "mocha", label: "theme.mocha", swatch: ["#1c1815", "#2e2924", "#e29b8d"] },
  { value: "aurora", label: "theme.aurora", swatch: ["#7c5cff", "#121a2c", "#2dd4bf"] },
  { value: "sunset", label: "theme.sunset", swatch: ["#2b1335", "#4c2020", "#ff9e6d"] },
  { value: "neon", label: "theme.neon", swatch: ["#ff0080", "#140c20", "#00e5ff"] },
  { value: "emerald", label: "theme.emerald", swatch: ["#0e1f18", "#34d399", "#e9c46a"] },
  { value: "nightcity", label: "theme.nightcity", swatch: ["#07070c", "#00f0ff", "#fcee0a"] },
  { value: "crimson", label: "theme.crimson", swatch: ["#120708", "#dc2626", "#ff5a5f"] },
  { value: "arctic", label: "theme.arctic", swatch: ["#eaf1f8", "#ffffff", "#2f6fdc"] },
  { value: "rose", label: "theme.rose", swatch: ["#1a1016", "#f4a6b8", "#facc15"] },
  { value: "matrix", label: "theme.matrix", swatch: ["#030906", "#0d1f18", "#22f27c"] },
];

export function ThemePicker() {
  const t = useT();
  const theme = useSettings((s) => s.theme);
  const setTheme = useSettings((s) => s.setTheme);
  return (
    <div className="grid grid-cols-[repeat(auto-fill,minmax(8.5rem,1fr))] gap-2">
      {options.map((o) => (
        <button
          key={o.value}
          onClick={() => setTheme(o.value)}
          aria-pressed={theme === o.value}
          className={clsx(
            "flex flex-col gap-2 rounded-lg border p-2 text-left transition-colors",
            theme === o.value ? "border-accent bg-accent/10" : "border-line hover:bg-surface-2",
          )}
        >
          <span className="flex h-8 overflow-hidden rounded-md border border-line">
            {o.swatch.map((c) => (
              <span key={c} className="flex-1" style={{ background: c }} />
            ))}
          </span>
          <span className={clsx("px-1", theme === o.value && "font-medium")}>{t(o.label)}</span>
        </button>
      ))}
    </div>
  );
}
