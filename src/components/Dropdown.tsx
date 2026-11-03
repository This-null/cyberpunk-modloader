import { useEffect, useMemo, useRef, useState } from "react";
import clsx from "clsx";
import { Check, ChevronDown, CornerDownRight, Search } from "lucide-react";

export interface DropdownOption<V extends string | number> {
  value: V;
  label: string;
  depth?: number;
}

interface Props<V extends string | number> {
  value: V;
  options: DropdownOption<V>[];
  onChange: (value: V) => void;
  ariaLabel: string;
  searchPlaceholder?: string;
  className?: string;
}

export function Dropdown<V extends string | number>({ value, options, onChange, ariaLabel, searchPlaceholder, className }: Props<V>) {
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState("");
  const [active, setActive] = useState(0);
  const root = useRef<HTMLDivElement>(null);
  const list = useRef<HTMLUListElement>(null);
  const searchable = options.length > 12;
  const shown = useMemo(() => {
    const q = query.trim().toLowerCase();
    return q === "" ? options : options.filter((o) => o.label.toLowerCase().includes(q));
  }, [options, query]);
  const current = options.find((o) => o.value === value);

  useEffect(() => {
    if (!open) return;
    function onDown(e: MouseEvent) {
      if (!root.current?.contains(e.target as Node)) setOpen(false);
    }
    document.addEventListener("mousedown", onDown);
    return () => document.removeEventListener("mousedown", onDown);
  }, [open]);

  useEffect(() => {
    if (!open) return;
    setQuery("");
    setActive(Math.max(0, options.findIndex((o) => o.value === value)));
  }, [open, options, value]);

  useEffect(() => {
    list.current?.children[active]?.scrollIntoView({ block: "nearest" });
  }, [active, open]);

  function pick(o: DropdownOption<V>) {
    onChange(o.value);
    setOpen(false);
  }

  function onKey(e: React.KeyboardEvent) {
    if (e.key === "Escape") {
      setOpen(false);
    } else if (e.key === "ArrowDown") {
      e.preventDefault();
      setActive((a) => Math.min(shown.length - 1, a + 1));
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setActive((a) => Math.max(0, a - 1));
    } else if (e.key === "Enter" && open && shown[active]) {
      e.preventDefault();
      pick(shown[active]);
    }
  }

  return (
    <div ref={root} className={clsx("relative", className)} onKeyDown={onKey}>
      <button
        type="button"
        aria-label={ariaLabel}
        aria-haspopup="listbox"
        aria-expanded={open}
        onClick={() => setOpen(!open)}
        className={clsx("field flex w-full items-center justify-between gap-3 text-left", open && "field-open")}
      >
        <span className="min-w-0 truncate">{current?.label ?? ""}</span>
        <ChevronDown size={16} className={clsx("shrink-0 text-muted transition-transform duration-200", open && "rotate-180 text-accent")} />
      </button>
      {open && (
        <div className="anim-pop absolute left-0 top-full z-50 mt-2 flex min-w-full max-w-[22rem] flex-col overflow-hidden rounded-xl border border-line bg-surface-2 shadow-2xl">
          {searchable && (
            <label className="flex items-center gap-2 border-b border-line px-3 py-2">
              <Search size={15} className="shrink-0 text-muted" />
              <input
                autoFocus
                value={query}
                onChange={(e) => {
                  setQuery(e.target.value);
                  setActive(0);
                }}
                placeholder={searchPlaceholder}
                aria-label={searchPlaceholder ?? ariaLabel}
                className="min-w-0 flex-1 bg-transparent outline-none placeholder:text-muted"
              />
            </label>
          )}
          <ul ref={list} role="listbox" aria-label={ariaLabel} className="max-h-72 overflow-auto p-1">
            {shown.map((o, i) => (
              <li key={String(o.value)} role="option" aria-selected={o.value === value}>
                <button
                  type="button"
                  onClick={() => pick(o)}
                  onMouseEnter={() => setActive(i)}
                  style={{ paddingLeft: `${0.75 + (o.depth ?? 0) * 0.9}rem` }}
                  className={clsx(
                    "flex w-full items-center justify-between gap-3 rounded-lg py-2 pr-3 text-left transition-colors",
                    i === active ? "bg-accent/15 text-ink" : "text-muted",
                    o.value === value && "font-medium text-ink",
                  )}
                >
                  <span className="flex min-w-0 items-center gap-2">
                    {(o.depth ?? 0) > 0 && <CornerDownRight size={12} className="shrink-0 text-muted" aria-hidden />}
                    <span className="min-w-0 truncate">{o.label}</span>
                  </span>
                  {o.value === value && <Check size={15} className="shrink-0 text-accent" />}
                </button>
              </li>
            ))}
            {shown.length === 0 && <li className="px-3 py-2 text-muted">—</li>}
          </ul>
        </div>
      )}
    </div>
  );
}
