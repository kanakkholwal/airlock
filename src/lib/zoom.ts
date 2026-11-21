export type Size = { w: number; h: number };
export type View = { scale: number; x: number; y: number };

export const MIN_SCALE = 1;
export const MAX_SCALE = 8;
export const FIT: View = { scale: 1, x: 0, y: 0 };

/** The size an image takes when it is fitted inside a stage without being cropped. */
export function containSize(natural: Size, stage: Size): Size {
  if (natural.w <= 0 || natural.h <= 0 || stage.w <= 0 || stage.h <= 0) return { w: 0, h: 0 };
  const k = Math.min(stage.w / natural.w, stage.h / natural.h);
  return { w: natural.w * k, h: natural.h * k };
}

/** Keeps the picture from being dragged out of view; at fit size it stays centred. */
export function clampView(v: View, base: Size, stage: Size): View {
  const scale = Math.min(MAX_SCALE, Math.max(MIN_SCALE, v.scale));
  const maxX = Math.max(0, (base.w * scale - stage.w) / 2);
  const maxY = Math.max(0, (base.h * scale - stage.h) / 2);
  return {
    scale,
    // `|| 0` turns -0 into 0, so a centred picture compares equal to the fit view.
    x: Math.min(maxX, Math.max(-maxX, v.x)) || 0,
    y: Math.min(maxY, Math.max(-maxY, v.y)) || 0,
  };
}

/** Zooms by a factor while keeping the point under the cursor still. `point` is measured from the stage centre. */
export function zoomAt(v: View, factor: number, point: { x: number; y: number }, base: Size, stage: Size): View {
  const next = Math.min(MAX_SCALE, Math.max(MIN_SCALE, v.scale * factor));
  const r = next / v.scale;
  return clampView({ scale: next, x: point.x - (point.x - v.x) * r, y: point.y - (point.y - v.y) * r }, base, stage);
}

/** The scale at which one image pixel is one screen pixel, or null when fit already shows more. */
export function actualPixelsScale(natural: Size, base: Size): number | null {
  if (base.w <= 0) return null;
  const s = natural.w / base.w;
  return s > MIN_SCALE ? Math.min(s, MAX_SCALE) : null;
}
