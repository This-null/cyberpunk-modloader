const STAMPED = /^(.+?)\s(\d{2,9})\s(\d[\w.\-]*)\s(?:20\d{2}(?:-\d{2}(?:-\d{2}(?:T[\d-]*Z?)?)?)?|20)(?:\s\S+)?$/;

export function stampedId(name: string): number | null {
  const m = STAMPED.exec(name.trim());
  if (!m) return null;
  const id = Number(m[2]);
  return Number.isSafeInteger(id) && id > 0 ? id : null;
}
