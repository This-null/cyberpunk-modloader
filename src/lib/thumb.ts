export async function makeThumb(dataUrl: string, maxWidth = 420): Promise<string | null> {
  const img = new Image();
  img.decoding = "async";
  img.src = dataUrl;
  try {
    await img.decode();
  } catch {
    return null;
  }
  if (!img.naturalWidth || !img.naturalHeight) return null;
  const scale = Math.min(1, maxWidth / img.naturalWidth);
  const canvas = document.createElement("canvas");
  canvas.width = Math.max(1, Math.round(img.naturalWidth * scale));
  canvas.height = Math.max(1, Math.round(img.naturalHeight * scale));
  const ctx = canvas.getContext("2d");
  if (!ctx) return null;
  ctx.drawImage(img, 0, 0, canvas.width, canvas.height);
  const out = canvas.toDataURL("image/jpeg", 0.82);
  img.src = "";
  return out.startsWith("data:image/jpeg") ? out : null;
}
