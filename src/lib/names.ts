export function modNameFromPath(path: string, existing: readonly string[]): string {
  const file = path.split(/[\\/]/).pop() ?? "";
  const base = file.replace(/\.(zip|rar|7z)$/i, "");
  const clean =
    base
      .replace(/[\\/:*?"<>|]/g, "_")
      .replace(/^\.+/, "")
      .trim()
      .slice(0, 56) || "mod";
  let name = clean;
  let n = 2;
  while (existing.some((e) => e.toLowerCase() === name.toLowerCase())) {
    name = `${clean} (${n++})`;
  }
  return name;
}
