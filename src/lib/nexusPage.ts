import type { NexusFileInfo } from "./ipc";

const GAME = "cyberpunk2077";

export function pickFile(files: NexusFileInfo[]): NexusFileInfo | null {
  const usable = files.filter((f) => f.file_id > 0 && !["OLD_VERSION", "ARCHIVED", "DELETED"].includes(f.category_name.toUpperCase()));
  return (
    usable.find((f) => f.is_primary) ??
    usable.find((f) => f.category_name.toUpperCase() === "MAIN") ??
    [...usable].sort((a, b) => b.uploaded_timestamp - a.uploaded_timestamp)[0] ??
    null
  );
}

export function downloadPageUrl(modId: number, fileId: number | null): string {
  const page = `https://www.nexusmods.com/${GAME}/mods/${modId}`;
  return fileId ? `${page}?tab=files&file_id=${fileId}&nmm=1` : `${page}?tab=files`;
}
