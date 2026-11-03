import { Logo } from "./Logo";

export function coverStyle(name: string): { hue: number; letters: string } {
  let h = 0;
  for (const ch of name) h = (h * 31 + ch.codePointAt(0)!) % 360;
  const letters = (name.match(/[\p{L}\p{N}]/gu) ?? []).slice(0, 2).join("").toUpperCase();
  return { hue: h, letters: letters || "?" };
}

export function CoverPlaceholder({ name, size = "md" }: { name: string; size?: "md" | "lg" }) {
  const { hue, letters } = coverStyle(name);
  return (
    <div
      className="relative grid h-full w-full place-items-center overflow-hidden"
      style={{ background: `linear-gradient(135deg, hsl(${hue} 38% 24%), hsl(${(hue + 40) % 360} 42% 14%))` }}
      aria-hidden
    >
      <Logo size={size === "lg" ? 150 : 96} className="absolute -right-3 -bottom-4 text-white/10" />
      <span className={size === "lg" ? "text-5xl font-semibold tracking-wide text-white/85" : "text-3xl font-semibold tracking-wide text-white/85"}>{letters}</span>
    </div>
  );
}
