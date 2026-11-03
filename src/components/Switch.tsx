import clsx from "clsx";
import type { ReactNode } from "react";

interface Props {
  on: boolean;
  onChange: (on: boolean) => void;
  label: ReactNode;
  hint?: ReactNode;
  disabled?: boolean;
}

export function Switch({ on, onChange, label, hint, disabled }: Props) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={on}
      disabled={disabled}
      onClick={() => onChange(!on)}
      className="group flex w-full items-center gap-4 border border-line bg-surface-2 px-4 py-3 text-left transition-colors hover:border-accent/50 disabled:opacity-60"
    >
      <span className="flex min-w-0 flex-1 flex-col gap-0.5">
        <span className="font-medium">{label}</span>
        {hint && <span className="text-xs text-muted">{hint}</span>}
      </span>
      <span
        aria-hidden
        className={clsx(
          "relative h-6 w-12 shrink-0 border transition-colors duration-200",
          on ? "border-accent bg-accent/25" : "border-line bg-bg",
        )}
      >
        <span
          className={clsx(
            "absolute top-0.5 h-4 w-4 transition-all duration-200",
            on ? "left-[calc(100%-1.125rem)] bg-accent" : "left-0.5 bg-muted",
          )}
        />
      </span>
    </button>
  );
}
