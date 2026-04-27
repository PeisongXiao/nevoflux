// canvas-inspect/inspect.mjs — runtime layout + WCAG audit for a composition.
//
// Loaded by background.js when the daemon dispatches a
// `canvas_video_inspect_request`. Renders the composition into a hidden
// iframe, seeks the timeline at N timestamps, collects bboxes, and runs
// WCAG AA contrast on every text element.
//
// Reports the same shape the daemon's `InspectReport` expects.

const SETTLE_TIMEOUT_MS = 1500;
const POLL_INTERVAL_MS = 30;

/**
 * Run an inspect pass. Returns an `InspectReport`.
 *
 * @param {{ html: string, stage_w: number, stage_h: number,
 *          frames: number, at: number[] }} req
 */
export async function inspect(req) {
  const t0 = (typeof performance !== 'undefined' && performance.now)
    ? performance.now() : Date.now();
  const issues = [];
  const stageW = req.stage_w || 1920;
  const stageH = req.stage_h || 1080;

  const iframe = document.createElement('iframe');
  iframe.style.cssText =
    `position:absolute; left:-99999px; top:0; width:${stageW}px; height:${stageH}px;` +
    ` visibility:hidden; pointer-events:none; border:0;`;
  iframe.srcdoc = req.html || '';
  document.body.appendChild(iframe);

  let framesChecked = 0;
  try {
    await waitForLoad(iframe);
    const win = iframe.contentWindow;
    const doc = iframe.contentDocument;

    // Wait for GSAP timelines to register.
    const settled = await waitForTimelines(win);
    if (!settled) {
      issues.push({
        t: 0, kind: 'internal',
        selector: ':root',
        stage_w: stageW, stage_h: stageH,
        fix_hint: 'No GSAP timelines registered after ' + SETTLE_TIMEOUT_MS + 'ms — composition may be malformed or use a non-GSAP animation library.',
      });
    }

    // Compute total duration from #stage data-duration attr.
    const stageEl = doc.getElementById('stage')
      || doc.querySelector('[data-composition-id]')
      || doc.body.firstElementChild;
    const totalDuration = parseFloat(
      stageEl ? stageEl.getAttribute('data-duration') || '0' : '0'
    ) || 10;

    // Build timestamp list: N evenly-spaced + explicit `at` entries.
    const N = Math.max(1, Math.min(30, req.frames || 8));
    const samples = [];
    for (let i = 0; i < N; i++) samples.push(((i + 0.5) / N) * totalDuration);
    if (Array.isArray(req.at)) {
      for (const t of req.at) {
        if (Number.isFinite(t) && t >= 0 && t <= totalDuration) samples.push(t);
      }
    }
    samples.sort((a, b) => a - b);

    for (const t of samples) {
      seekAll(win, t);
      forceLayoutFlush(doc);
      framesChecked++;

      // ─── Layout audit ─────────────────────────────────────────────
      const trackEls = Array.from(doc.querySelectorAll('[data-track-index], .clip'));
      for (const el of trackEls) {
        if (el.hasAttribute('data-layout-allow-overflow')) continue;
        if (el.hasAttribute('data-layout-ignore')) continue;
        const ds = parseFloat(el.getAttribute('data-start') || '0');
        const dd = parseFloat(el.getAttribute('data-duration') || '0');
        const active = dd === 0 || (t >= ds && t <= ds + dd + 0.05);
        if (!active) continue;

        const cs = win.getComputedStyle(el);
        if (cs.display === 'none' || cs.visibility === 'hidden') continue;

        const r = el.getBoundingClientRect();
        const sel = elementSelector(el);
        const bbox = { x: round1(r.left), y: round1(r.top), w: round1(r.width), h: round1(r.height) };

        // Off-stage: bbox entirely outside stage rect.
        if (r.left >= stageW || r.top >= stageH || r.right <= 0 || r.bottom <= 0) {
          issues.push({ t: round3(t), kind: 'off_stage', selector: sel,
            bbox, stage_w: stageW, stage_h: stageH,
            fix_hint: 'Element entirely off-stage during its visibility window. Check entrance / exit tween end states.',
          });
          continue;
        }

        // Overflow on horizontal/vertical axes (with 1px tolerance).
        if (r.right > stageW + 1) {
          issues.push({ t: round3(t), kind: 'overflow_x', selector: sel,
            bbox, stage_w: stageW, stage_h: stageH,
            fix_hint: `Extends ${Math.round(r.right - stageW)}px past right edge. Reduce font-size, max-width, or padding.`,
          });
        } else if (r.left < -1) {
          issues.push({ t: round3(t), kind: 'overflow_x', selector: sel,
            bbox, stage_w: stageW, stage_h: stageH,
            fix_hint: `Extends ${Math.round(-r.left)}px past left edge.`,
          });
        }
        if (r.bottom > stageH + 1) {
          issues.push({ t: round3(t), kind: 'overflow_y', selector: sel,
            bbox, stage_w: stageW, stage_h: stageH,
            fix_hint: `Extends ${Math.round(r.bottom - stageH)}px below stage bottom.`,
          });
        } else if (r.top < -1) {
          issues.push({ t: round3(t), kind: 'overflow_y', selector: sel,
            bbox, stage_w: stageW, stage_h: stageH,
            fix_hint: `Extends ${Math.round(-r.top)}px above stage top.`,
          });
        }

        // Zero-size: nominally visible (opacity > 0 / display block) but
        // 0×0. This catches missing content or broken flex/grid containers.
        if (r.width === 0 && r.height === 0 && parseFloat(cs.opacity || '1') > 0.05) {
          issues.push({ t: round3(t), kind: 'zero_size', selector: sel,
            bbox, stage_w: stageW, stage_h: stageH,
            fix_hint: 'Element has zero size during its active window. Check that content was injected and the parent layout is sized.',
          });
        }
      }

      // ─── WCAG contrast audit ──────────────────────────────────────
      // Text elements: only check leaf nodes that contain direct text
      // (not just child elements). Walk the candidates list once per
      // sample timestamp.
      const textCandidates = doc.querySelectorAll(
        'h1, h2, h3, h4, h5, h6, p, span, a, li, td, th, label, button, div'
      );
      for (const el of textCandidates) {
        if (!hasDirectText(el)) continue;
        const cs = win.getComputedStyle(el);
        if (cs.display === 'none' || cs.visibility === 'hidden') continue;
        if (parseFloat(cs.opacity || '1') < 0.5) continue;

        const r = el.getBoundingClientRect();
        if (r.width <= 0 || r.height <= 0) continue;

        const fontSize = parseFloat(cs.fontSize) || 0;
        const fontWeight = parseInt(cs.fontWeight, 10) || 400;
        const isLarge = fontSize >= 24 || (fontSize >= 19 && fontWeight >= 700);
        const required = isLarge ? 3.0 : 4.5;

        const fg = cs.color;
        const bg = effectiveBackground(el, win);
        const fgRgb = parseRgb(fg);
        const bgRgb = parseRgb(bg);
        if (!fgRgb || !bgRgb) continue;
        const ratio = contrastRatio(fgRgb, bgRgb);

        // -0.05 tolerance — avoid flapping right at the threshold.
        if (ratio < required - 0.05) {
          issues.push({ t: round3(t), kind: 'contrast',
            selector: elementSelector(el),
            stage_w: stageW, stage_h: stageH,
            fg: rgbToHex(fgRgb), bg: rgbToHex(bgRgb),
            ratio: Math.round(ratio * 100) / 100,
            required,
            fix_hint: 'Use a darker / lighter color from DESIGN.md Colors. WCAG AA requires '
              + required + ':1 (got ' + ratio.toFixed(2) + ':1).',
          });
        }
      }
    }
  } catch (err) {
    issues.push({
      t: 0, kind: 'internal',
      selector: ':root',
      stage_w: stageW, stage_h: stageH,
      fix_hint: 'inspect failed: ' + (err && err.message ? err.message : String(err)),
    });
  } finally {
    iframe.remove();
  }

  return {
    frames_checked: framesChecked,
    stage_w: stageW,
    stage_h: stageH,
    issues,
    elapsed_ms: Math.round(((typeof performance !== 'undefined' && performance.now)
      ? performance.now() : Date.now()) - t0),
  };
}

// ─── Helpers ─────────────────────────────────────────────────────────────

function waitForLoad(iframe) {
  return new Promise((resolve) => {
    if (iframe.contentDocument && iframe.contentDocument.readyState === 'complete') {
      resolve(); return;
    }
    iframe.addEventListener('load', () => resolve(), { once: true });
  });
}

async function waitForTimelines(win) {
  const start = Date.now();
  while (Date.now() - start < SETTLE_TIMEOUT_MS) {
    const tls = win.__timelines;
    if (Array.isArray(tls) && tls.length > 0) return true;
    if (tls && typeof tls === 'object' && Object.keys(tls).length > 0) return true;
    await new Promise((r) => setTimeout(r, POLL_INTERVAL_MS));
  }
  return false;
}

function seekAll(win, t) {
  win.__nfRenderTime = t;
  const tls = win.__timelines;
  const list = Array.isArray(tls) ? tls : (tls ? Object.values(tls) : []);
  for (const tl of list) {
    try { if (tl && typeof tl.seek === 'function') tl.seek(t); } catch (_) { /* ignore */ }
  }
}

function forceLayoutFlush(doc) {
  // Read offsetHeight to trigger synchronous layout.
  // eslint-disable-next-line no-unused-expressions
  void doc.body.offsetHeight;
}

function elementSelector(el) {
  if (el.id) return '#' + el.id;
  const cls = (el.getAttribute('class') || '').split(/\s+/).filter(Boolean);
  if (cls.length) return el.tagName.toLowerCase() + '.' + cls[0];
  return el.tagName.toLowerCase();
}

function hasDirectText(el) {
  for (const node of el.childNodes) {
    if (node.nodeType === 3 /* TEXT_NODE */ && node.nodeValue.trim().length > 0) {
      return true;
    }
  }
  return false;
}

function effectiveBackground(el, win) {
  // Walk up the parent chain until a non-transparent background-color is found.
  let cur = el;
  while (cur && cur !== cur.ownerDocument.documentElement) {
    const cs = win.getComputedStyle(cur);
    const bg = cs.backgroundColor;
    if (bg && bg !== 'rgba(0, 0, 0, 0)' && bg !== 'transparent') {
      return bg;
    }
    cur = cur.parentElement;
  }
  // Fallback: composition default-bg from #stage data-bg, else white.
  const stage = el.ownerDocument.getElementById('stage');
  return (stage && stage.getAttribute('data-bg')) || '#ffffff';
}

function parseRgb(s) {
  if (!s) return null;
  // 'rgb(R, G, B)' / 'rgba(R, G, B, A)' / '#hex' / 'name'
  const rgbMatch = String(s).match(/rgba?\(\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)/i);
  if (rgbMatch) {
    return [parseInt(rgbMatch[1], 10), parseInt(rgbMatch[2], 10), parseInt(rgbMatch[3], 10)];
  }
  const hex = String(s).replace(/^#/, '');
  if (/^[0-9a-f]{6}$/i.test(hex)) {
    return [parseInt(hex.slice(0, 2), 16), parseInt(hex.slice(2, 4), 16), parseInt(hex.slice(4, 6), 16)];
  }
  if (/^[0-9a-f]{3}$/i.test(hex)) {
    return [parseInt(hex[0] + hex[0], 16), parseInt(hex[1] + hex[1], 16), parseInt(hex[2] + hex[2], 16)];
  }
  return null;
}

function rgbToHex(rgb) {
  const [r, g, b] = rgb;
  return '#' + [r, g, b].map((c) => c.toString(16).padStart(2, '0')).join('');
}

function relativeLuminance(rgb) {
  const [r, g, b] = rgb.map((c) => {
    const s = c / 255;
    return s <= 0.03928 ? s / 12.92 : Math.pow((s + 0.055) / 1.055, 2.4);
  });
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}

function contrastRatio(a, b) {
  const la = relativeLuminance(a);
  const lb = relativeLuminance(b);
  const [hi, lo] = la > lb ? [la, lb] : [lb, la];
  return (hi + 0.05) / (lo + 0.05);
}

function round1(n) { return Math.round(n * 10) / 10; }
function round3(n) { return Math.round(n * 1000) / 1000; }
