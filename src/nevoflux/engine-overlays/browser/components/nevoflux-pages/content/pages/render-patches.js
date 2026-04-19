// Determinism patches injected into the composition iframe before any
// user HTML runs. Spec: docs/superpowers/specs/2026-04-19-video-skill-design.md §4.2

/**
 * Install all patches into the provided iframe's contentWindow.
 * MUST run after iframe is created but BEFORE the composition HTML
 * begins executing scripts.
 */
export function installPatches(iframeWindow) {
  const w = iframeWindow;

  // --- Patch 1: Math.random (Mulberry32 seeded) ---
  let prngSeed = 42;
  let s = prngSeed;
  w.Math.random = function () {
    s |= 0;
    s = (s + 0x6D2B79F5) | 0;
    let t = Math.imul(s ^ (s >>> 15), 1 | s);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };

  // --- Patch 2: Date.now / performance.now (timeline-driven) ---
  const BASE_WALL_CLOCK = 1700000000000;
  w.__hfRenderTime = 0; // seconds; updated by seek()
  const origDate = w.Date;
  const PatchedDate = function (...args) {
    if (args.length === 0) {
      return new origDate(BASE_WALL_CLOCK + w.__hfRenderTime * 1000);
    }
    return new origDate(...args);
  };
  PatchedDate.now = () => BASE_WALL_CLOCK + w.__hfRenderTime * 1000;
  PatchedDate.parse = origDate.parse;
  PatchedDate.UTC = origDate.UTC;
  PatchedDate.prototype = origDate.prototype;
  w.Date = PatchedDate;
  w.performance.now = () => w.__hfRenderTime * 1000;

  // --- Patch 3: fetch whitelist ---
  const origFetch = w.fetch.bind(w);
  w.fetch = function (url, opts) {
    const u = typeof url === 'string' ? url : (url && url.url) || '';
    if (u.startsWith('assets/') || u.startsWith('./assets/')) {
      return origFetch(url, opts);
    }
    if (u.startsWith('https://esm.sh/')) {
      return origFetch(url, opts);
    }
    return Promise.reject(new Error(`fetch blocked in render: ${u}`));
  };

  // --- Patch 4: GSAP ticker freeze (applied after composition loads GSAP) ---
  w.addEventListener('DOMContentLoaded', () => {
    if (w.gsap && w.gsap.ticker) {
      w.gsap.ticker.sleep();
      w.gsap.ticker.lagSmoothing(0);
    }
  });

  // --- Patch 5: crypto.getRandomValues ---
  w.crypto.getRandomValues = function (array) {
    for (let i = 0; i < array.length; i++) {
      array[i] = Math.floor(w.Math.random() * 256);
    }
    return array;
  };

  // --- Patch 6: crypto.randomUUID ---
  w.crypto.randomUUID = function () {
    const hex = () => Math.floor(w.Math.random() * 16).toString(16);
    const s = (n) => [...Array(n)].map(hex).join('');
    const variantDigit = (8 + Math.floor(w.Math.random() * 4)).toString(16);
    return `${s(8)}-${s(4)}-4${s(3)}-${variantDigit}${s(3)}-${s(12)}`;
  };
}

/**
 * Advance the render-time clock (patched Date.now / performance.now
 * will return this value).
 */
export function setRenderTime(iframeWindow, seconds) {
  iframeWindow.__hfRenderTime = seconds;
}
