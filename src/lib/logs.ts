export type Level = "error" | "warn" | "info";
export type Filter = "all" | "error" | "warn";

export interface Line {
  n: number;
  text: string;
  level: Level;
}

const ERROR = /\[(error|fatal|critical)\s*\]|\b(fatal|panic|exception|segfault|access violation|unhandled|stack trace)\b|\berror\b/i;
const WARN = /\[warn(ing)?\s*\]|\bwarn(ing)?\b/i;

export const MAX_RENDERED = 4000;

export function classify(text: string): Level {
  if (ERROR.test(text)) return "error";
  if (WARN.test(text)) return "warn";
  return "info";
}

export function parseLines(text: string, firstNumber = 1): Line[] {
  if (!text) return [];
  const raw = text.split("\n");
  if (raw[raw.length - 1] === "") raw.pop();
  return raw.map((l, i) => {
    const clean = l.endsWith("\r") ? l.slice(0, -1) : l;
    return { n: firstNumber + i, text: clean, level: classify(clean) };
  });
}

export function countLevels(lines: readonly Line[]): { error: number; warn: number } {
  let error = 0;
  let warn = 0;
  for (const l of lines) {
    if (l.level === "error") error++;
    else if (l.level === "warn") warn++;
  }
  return { error, warn };
}

export function filterLines(lines: readonly Line[], filter: Filter, query: string): Line[] {
  const q = query.trim().toLowerCase();
  return lines.filter((l) => {
    if (filter !== "all" && l.level !== filter) return false;
    return q === "" || l.text.toLowerCase().includes(q);
  });
}

export function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1048576) return `${(bytes / 1024).toFixed(0)} KB`;
  if (bytes < 1073741824) return `${(bytes / 1048576).toFixed(1)} MB`;
  return `${(bytes / 1073741824).toFixed(2)} GB`;
}

export function nextIndex(levels: readonly Level[], after: number): number {
  for (let i = after + 1; i < levels.length; i++) {
    if (levels[i] === "error") return i;
  }
  for (let i = 0; i <= Math.min(after, levels.length - 1); i++) {
    if (levels[i] === "error") return i;
  }
  return -1;
}
