/** Points the element's eyes at the pointer: writes `--ex` and `--ey` (in px, at most `max`). */
export function lookAt(node: Element, max = 4) {
  if (matchMedia("(prefers-reduced-motion: reduce)").matches) return;
  const el = node as HTMLElement | SVGElement;
  let frame = 0;
  let px = 0;
  let py = 0;
  const apply = () => {
    frame = 0;
    const r = el.getBoundingClientRect();
    const dx = px - (r.left + r.width / 2);
    const dy = py - (r.top + r.height / 2);
    const d = Math.hypot(dx, dy) || 1;
    // Far away the eyes lean fully, close up they settle, so they never twitch at the centre.
    const k = Math.min(1, d / 160);
    el.style.setProperty("--ex", ((dx / d) * max * k).toFixed(2));
    el.style.setProperty("--ey", ((dy / d) * max * k).toFixed(2));
  };
  const onMove = (e: PointerEvent) => {
    px = e.clientX;
    py = e.clientY;
    if (!frame) frame = requestAnimationFrame(apply);
  };
  addEventListener("pointermove", onMove, { passive: true });
  return {
    destroy() {
      removeEventListener("pointermove", onMove);
      if (frame) cancelAnimationFrame(frame);
    },
  };
}

/** Adds `data-in` once the element has scrolled into view, so a CSS transition can play once. */
export function inView(node: Element) {
  const io = new IntersectionObserver(
    ([entry]) => {
      if (entry.isIntersecting) {
        node.setAttribute("data-in", "");
        io.disconnect();
      }
    },
    { threshold: 0.3 },
  );
  io.observe(node);
  return { destroy: () => io.disconnect() };
}
