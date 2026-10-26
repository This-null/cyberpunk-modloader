const TOP = "M104,16L174,86L34,86Z";
const MID = "M38,106L218,106L240,128L218,150L38,150L16,128Z";
const BOTTOM = "M82,170L222,170L152,240Z";

export function Logo({ size = 20, className, animated = false }: { size?: number; className?: string; animated?: boolean }) {
  return (
    <svg width={size} height={size} viewBox="0 0 256 256" fill="currentColor" className={className} aria-hidden>
      <path d={TOP} className={animated ? "logo-s1" : undefined} />
      <path d={MID} className={animated ? "logo-s2" : undefined} />
      <path d={BOTTOM} className={animated ? "logo-s3" : undefined} />
    </svg>
  );
}
