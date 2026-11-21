import type { ImageInfo } from "../bindings";

// Decision (product): what the Images screens offer comes from this data, not from markup. A new
// shape or idea is a line here, and the gallery's filters are built from the pictures themselves.
export type ShapeId = "square" | "portrait" | "landscape";

export const SHAPES: { value: ShapeId; label: string; width: number; height: number; box: string }[] = [
  { value: "square", label: "Square", width: 512, height: 512, box: "size-3.5" },
  { value: "portrait", label: "Portrait", width: 512, height: 768, box: "h-4 w-2.5" },
  { value: "landscape", label: "Landscape", width: 768, height: 512, box: "h-2.5 w-4" },
];

export const IDEAS = [
  "A lighthouse on a cliff at dusk, oil painting",
  "A cozy reading nook with rain on the window",
  "A paper-craft fox in a forest, soft light",
];

export type GallerySort = "newest" | "oldest";
export const SORTS: { value: GallerySort; label: string }[] = [
  { value: "newest", label: "Newest first" },
  { value: "oldest", label: "Oldest first" },
];

/** The shape a picture counts as, from its own size. */
export function shapeOf(image: { width: number; height: number }): ShapeId {
  if (image.width === image.height) return "square";
  return image.width < image.height ? "portrait" : "landscape";
}

export type GalleryFilter = { query: string; shape: ShapeId | "all"; sort: GallerySort };

export function filterImages(images: readonly ImageInfo[], f: GalleryFilter): ImageInfo[] {
  const q = f.query.trim().toLowerCase();
  const out = images.filter(
    (i) => (f.shape === "all" || shapeOf(i) === f.shape) && (!q || i.prompt.toLowerCase().includes(q)),
  );
  return out.sort((a, b) => (f.sort === "newest" ? b.created_ms - a.created_ms : a.created_ms - b.created_ms));
}

/** Only the shapes that exist in the gallery, with how many of each, so a filter never shows zero. */
export function shapeCounts(images: readonly ImageInfo[]): { value: ShapeId; label: string; count: number }[] {
  return SHAPES.map((s) => ({ value: s.value, label: s.label, count: images.filter((i) => shapeOf(i) === s.value).length })).filter(
    (s) => s.count > 0,
  );
}
