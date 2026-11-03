import { useEffect, useState, type ReactNode } from "react";
import { Check, FileArchive, MousePointer2, Power, PowerOff, Sparkles } from "lucide-react";
import clsx from "clsx";
import { useT } from "../i18n";

function usePhase(count: number, ms: number) {
  const [phase, setPhase] = useState(0);
  useEffect(() => {
    const id = setInterval(() => setPhase((p) => (p + 1) % count), ms);
    return () => clearInterval(id);
  }, [count, ms]);
  return [phase, setPhase] as const;
}

function Cursor({ x, y, press }: { x: number; y: number; press?: boolean }) {
  return (
    <MousePointer2
      size={22}
      fill="white"
      className={clsx(
        "pointer-events-none absolute z-10 text-black drop-shadow transition-all duration-700 ease-out",
        press && "scale-75",
      )}
      style={{ left: `${x}%`, top: `${y}%` }}
    />
  );
}

function Stage({ children }: { children: ReactNode }) {
  return <div className="relative h-48 overflow-hidden border border-line bg-bg p-4">{children}</div>;
}

function DemoCard({ on, name, children }: { on: boolean; name: string; children?: ReactNode }) {
  return (
    <div className={clsx("flex flex-col gap-2 border border-l-[3px] bg-surface p-3 transition-colors duration-500", on ? "border-line border-l-ok" : "border-line")}>
      <div className="flex items-center gap-2">
        <span className="font-semibold">{name}</span>
        <span className={clsx("size-2 rounded-full transition-colors duration-500", on ? "bg-ok" : "bg-muted")} />
      </div>
      {children}
    </div>
  );
}

function Welcome() {
  return (
    <Stage>
      <div className="grid h-full place-items-center">
        <span className="relative grid size-20 place-items-center border border-accent/60 bg-accent/15 text-accent">
          <span className="absolute inset-0 animate-ping border border-accent/50" />
          <Sparkles size={34} />
        </span>
      </div>
    </Stage>
  );
}

function AddScene() {
  const [phase] = usePhase(5, 900);
  const dropped = phase >= 2;
  return (
    <Stage>
      <div className={clsx("absolute inset-x-6 bottom-4 top-20 grid place-items-center border-2 border-dashed transition-colors duration-500", phase === 1 ? "border-accent bg-accent/10" : "border-line")}>
        {!dropped && <span className="text-xs text-muted">.zip  .rar  .7z</span>}
        {dropped && (
          <div className="anim-rise w-3/4">
            <DemoCard on={phase >= 3} name="Cool Mod" />
          </div>
        )}
      </div>
      <FileArchive
        size={30}
        className={clsx("absolute left-1/2 z-10 -translate-x-1/2 text-accent transition-all duration-700 ease-in", phase === 0 && "top-2", phase === 1 && "top-16", phase >= 2 && "top-20 scale-0 opacity-0")}
      />
    </Stage>
  );
}

function ToggleScene() {
  const t = useT();
  const [phase, setPhase] = usePhase(6, 1000);
  const on = phase >= 2 && phase <= 4;
  const press = phase === 2 || phase === 5;
  const files = ["cool.archive", "cool.xl", "cool.reds"];
  const pos = phase === 0 ? [75, 85] : phase === 1 || phase === 2 ? [22, 62] : phase === 3 ? [60, 40] : [22, 62];
  return (
    <Stage>
      <div className="mx-auto w-4/5">
        <DemoCard on={on} name="Cool Mod">
          <div className="flex h-5 gap-1.5 overflow-hidden text-[11px] text-muted">
            {files.map((f, i) => (
              <span
                key={f}
                className={clsx("border border-line px-1.5 transition-all duration-500", on ? "translate-y-0 opacity-100" : "-translate-y-3 opacity-0")}
                style={{ transitionDelay: `${on ? i * 120 : 0}ms` }}
              >
                {f}
              </span>
            ))}
          </div>
          <button
            onClick={() => setPhase(on ? 5 : 2)}
            className={clsx("btn self-start transition-transform", on ? "" : "btn-ok", press && "scale-95")}
          >
            {on ? <PowerOff size={15} /> : <Power size={15} />}
            {on ? t("action.disable") : t("action.enable")}
          </button>
        </DemoCard>
      </div>
      <Cursor x={pos[0]} y={pos[1]} press={press} />
    </Stage>
  );
}

function ProfilesScene() {
  const t = useT();
  const [phase] = usePhase(6, 1100);
  const second = phase >= 3;
  const open = phase === 1 || phase === 4;
  const names = [t("profiles.first"), t("profiles.first").replace("1", "2")];
  const rows = [
    { name: "Mod A", p: [true, false] },
    { name: "Mod B", p: [true, false] },
    { name: "Mod C", p: [false, true] },
  ];
  return (
    <Stage>
      <div className="relative mx-auto w-3/5">
        <div className="field flex items-center justify-between">
          <span className="transition-all">{names[second ? 1 : 0]}</span>
          <span className="text-muted">▾</span>
        </div>
        <div className={clsx("absolute inset-x-0 top-full z-10 border border-line bg-surface transition-all duration-300", open ? "opacity-100" : "pointer-events-none -translate-y-1 opacity-0")}>
          {names.map((n) => (
            <div key={n} className="px-3 py-1.5">
              {n}
            </div>
          ))}
        </div>
        <div className="mt-3 flex flex-col gap-1.5">
          {rows.map((r) => (
            <div key={r.name} className="flex items-center justify-between border border-line bg-surface px-3 py-1.5">
              <span>{r.name}</span>
              <span className={clsx("size-2 rounded-full transition-colors duration-500", r.p[second ? 1 : 0] ? "bg-ok" : "bg-muted")} />
            </div>
          ))}
        </div>
      </div>
      <Cursor x={phase === 0 ? 80 : open ? 60 : 78} y={phase === 0 ? 85 : open ? (phase === 1 ? 32 : 52) : 80} press={phase === 2 || phase === 5} />
    </Stage>
  );
}

function NexusScene() {
  const t = useT();
  const [phase] = usePhase(6, 1000);
  const pct = phase < 2 ? 0 : phase === 2 ? 35 : phase === 3 ? 80 : 100;
  const done = phase >= 4;
  return (
    <Stage>
      <div className="mx-auto flex w-3/5 flex-col gap-2 border border-line bg-surface p-3">
        <span className="font-semibold">Cool Mod</span>
        <div className="h-1.5 bg-surface-2">
          <div className="h-full bg-accent transition-all duration-700" style={{ width: `${pct}%` }} />
        </div>
        <span className={clsx("btn btn-sm self-start transition-transform", done ? "" : "btn-ok btn-play", phase === 1 && "scale-95")}>
          {done ? <Check size={14} /> : null}
          {done ? t("nexusPage.installed") : t("nexusPage.getFile")}
        </span>
      </div>
      <Cursor x={phase === 0 ? 80 : 38} y={phase === 0 ? 85 : 70} press={phase === 1} />
    </Stage>
  );
}

export const SCENES = [Welcome, AddScene, ToggleScene, ProfilesScene, NexusScene];
