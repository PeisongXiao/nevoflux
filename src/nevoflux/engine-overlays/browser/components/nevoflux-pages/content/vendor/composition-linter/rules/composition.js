// composition-linter/rules/composition.js — ported from the upstream
// composition rule set. Rule IDs use the `comp/*` prefix.
// See ../LICENSE-NOTICE.md for attribution.

import { push } from '../utils.js';

// ── helpers ────────────────────────────────────────────────────────────────

/** Read an attribute value from a DOM element; returns null if absent. */
function readAttr(el, attr) {
  return el.hasAttribute(attr) ? (el.getAttribute(attr) ?? '') : null;
}

/** Truncate a string for use as a snippet in issue messages. */
function truncateSnippet(str, max = 120) {
  if (!str) return '';
  const s = str.replace(/\s+/g, ' ').trim();
  return s.length > max ? s.slice(0, max) + '…' : s;
}

/** Return all body-level element nodes in `doc` (skips text/comment nodes). */
function allElements(doc) {
  return Array.from(doc.querySelectorAll('*'));
}

// ── composition rule: timed_element_missing_visibility_hidden ──────────────
//
// Elements with data-start but no visibility-hidden initial state may flash
// before their scheduled start time.

function ruleTimedElementMissingVisibilityHidden(ctx, report) {
  const skipNames = new Set(['audio', 'script', 'style']);
  for (const el of allElements(ctx.doc)) {
    if (skipNames.has(el.tagName.toLowerCase())) continue;
    if (readAttr(el, 'data-start') === null) continue;
    if (readAttr(el, 'data-composition-id') !== null) continue;
    if (readAttr(el, 'data-composition-src') !== null) continue;

    const classAttr = el.getAttribute('class') || '';
    const styleAttr = el.getAttribute('style') || '';
    const hasClip = classAttr.split(/\s+/).includes('clip');
    const hasHiddenStyle =
      /visibility\s*:\s*hidden/i.test(styleAttr) || /opacity\s*:\s*0/i.test(styleAttr);

    if (!hasClip && !hasHiddenStyle) {
      const elementId = el.getAttribute('id') || undefined;
      push(report, {
        severity: 'info',
        rule_id: 'comp/timed-element-missing-visibility-hidden',
        message: `<${el.tagName.toLowerCase()}${elementId ? ` id="${elementId}"` : ''}> has data-start but no class="clip", visibility:hidden, or opacity:0. Consider adding initial hidden state if the element should not be visible before its start time.`,
        fix_hint: 'Add class="clip" (with CSS: .clip { visibility: hidden; }) or style="opacity:0" if the element should start hidden.',
        snippet: truncateSnippet(el.outerHTML),
      });
    }
  }
}

// ── composition rule: deprecated_data_layer ────────────────────────────────
//
// data-layer is replaced by data-track-index.

// ── composition rule: deprecated_data_end ─────────────────────────────────
//
// data-end is replaced by data-duration.

function ruleDeprecatedAttributes(ctx, report) {
  for (const el of allElements(ctx.doc)) {
    if (readAttr(el, 'data-layer') !== null && readAttr(el, 'data-track-index') === null) {
      const elementId = el.getAttribute('id') || undefined;
      push(report, {
        severity: 'warning',
        rule_id: 'comp/deprecated-data-layer',
        message: `<${el.tagName.toLowerCase()}${elementId ? ` id="${elementId}"` : ''}> uses data-layer instead of data-track-index.`,
        fix_hint: 'Replace data-layer with data-track-index. The runtime reads data-track-index.',
        snippet: truncateSnippet(el.outerHTML),
      });
    }
    if (readAttr(el, 'data-end') !== null && readAttr(el, 'data-duration') === null) {
      const elementId = el.getAttribute('id') || undefined;
      push(report, {
        severity: 'warning',
        rule_id: 'comp/deprecated-data-end',
        message: `<${el.tagName.toLowerCase()}${elementId ? ` id="${elementId}"` : ''}> uses data-end without data-duration. Use data-duration in source HTML.`,
        fix_hint: 'Replace data-end with data-duration. The compiler generates data-end from data-duration automatically.',
        snippet: truncateSnippet(el.outerHTML),
      });
    }
  }
}

// ── composition rule: template_literal_selector ────────────────────────────
//
// querySelector with a template literal variable crashes the CSS parser in
// the composition bundler.

function ruleTemplateLiteralSelector(ctx, report) {
  for (const script of ctx.scripts) {
    const content = script.textContent || '';
    const templateLiteralSelectorPattern =
      /(?:querySelector|querySelectorAll)\s*\(\s*`[^`]*\$\{[^}]+\}[^`]*`\s*\)/g;
    let match;
    while ((match = templateLiteralSelectorPattern.exec(content)) !== null) {
      push(report, {
        severity: 'error',
        rule_id: 'comp/template-literal-selector',
        message:
          'querySelector uses a template literal variable (e.g. `${compId}`). ' +
          'The HTML bundler\'s CSS parser crashes on these. Use a hardcoded string instead.',
        fix_hint:
          'Replace the template literal variable with a hardcoded string. The bundler\'s CSS parser cannot handle interpolated variables in script content.',
        snippet: truncateSnippet(match[0]),
      });
    }
  }
}

// ── composition rule: external_script_dependency ───────────────────────────
//
// External script URLs are informational — the composition bundler hoists
// them automatically in most pipelines.

function ruleExternalScriptDependency(ctx, report) {
  const externalScriptRe = /<script\b[^>]*\bsrc=["'](https?:\/\/[^"']+)["'][^>]*>/gi;
  let match;
  const seen = new Set();
  while ((match = externalScriptRe.exec(ctx.raw)) !== null) {
    const src = match[1] ?? '';
    if (seen.has(src)) continue;
    seen.add(src);
    push(report, {
      severity: 'info',
      rule_id: 'comp/external-script-dependency',
      message: `This composition loads an external script from \`${src}\`. The composition bundler automatically hoists CDN scripts from sub-compositions into the parent document. In unbundled runtime mode, \`loadExternalCompositions\` re-injects them. If you're using a custom pipeline that bypasses both, you'll need to include this script manually.`,
      fix_hint:
        'No action needed when using the standard composition preview or render pipeline. If using a custom pipeline, add this script tag to your root composition or HTML page.',
      snippet: truncateSnippet(match[0] ?? ''),
    });
  }
}

// ── composition rule: timed_element_missing_clip_class ────────────────────
//
// Elements that are scheduled clips (data-start present) should have
// class="clip" so the runtime controls their visibility. Elements that only
// carry data-duration (e.g. the stage root) define total duration and are
// not subject to this rule.

function ruleTimedElementMissingClipClass(ctx, report) {
  const skipNames = new Set(['audio', 'video', 'script', 'style', 'template']);
  for (const el of allElements(ctx.doc)) {
    if (skipNames.has(el.tagName.toLowerCase())) continue;
    if (readAttr(el, 'data-composition-id') !== null) continue;
    if (readAttr(el, 'data-composition-src') !== null) continue;

    // Require data-start: elements without data-start are container/root
    // elements (e.g. the stage), not scheduled clips.
    const hasStart = readAttr(el, 'data-start') !== null;
    if (!hasStart) continue;

    const classAttr = el.getAttribute('class') || '';
    const hasClip = classAttr.split(/\s+/).includes('clip');
    if (hasClip) continue;

    const elementId = el.getAttribute('id') || undefined;
    push(report, {
      severity: 'warning',
      rule_id: 'comp/timed-element-missing-clip-class',
      message: `<${el.tagName.toLowerCase()}${elementId ? ` id="${elementId}"` : ''}> has timing attributes but no class="clip". The element will be visible for the entire composition instead of only during its scheduled time range.`,
      fix_hint:
        'Add class="clip" to the element. The composition runtime uses .clip to control visibility based on data-start/data-duration.',
      snippet: truncateSnippet(el.outerHTML),
    });
  }
}

// ── composition rule: overlapping_clips_same_track ────────────────────────
//
// Clips on the same track that overlap in time cause rendering conflicts.

function ruleOverlappingClipsSameTrack(ctx, report) {
  const trackMap = new Map();

  for (const el of allElements(ctx.doc)) {
    const startStr = readAttr(el, 'data-start');
    const durationStr = readAttr(el, 'data-duration');
    const trackStr = readAttr(el, 'data-track-index');
    if (startStr === null || durationStr === null || trackStr === null) continue;

    const start = Number(startStr);
    const duration = Number(durationStr);
    const track = trackStr;

    // Skip non-numeric values (relative timing references)
    if (Number.isNaN(start) || Number.isNaN(duration)) continue;

    const clips = trackMap.get(track) || [];
    clips.push({
      start,
      end: start + duration,
      elementId: el.getAttribute('id') || undefined,
      snippet: truncateSnippet(el.outerHTML) || '',
    });
    trackMap.set(track, clips);
  }

  for (const [track, clips] of trackMap) {
    clips.sort((a, b) => a.start - b.start);
    for (let i = 0; i < clips.length - 1; i++) {
      const current = clips[i];
      const next = clips[i + 1];
      if (!current || !next) continue;
      if (current.end > next.start) {
        push(report, {
          severity: 'error',
          rule_id: 'comp/overlapping-clips-same-track',
          message: `Track ${track}: clip ending at ${current.end}s overlaps with clip starting at ${next.start}s. Overlapping clips on the same track cause rendering conflicts.`,
          fix_hint:
            'Adjust data-start or data-duration so clips on the same track do not overlap, or move one clip to a different data-track-index.',
        });
      }
    }
  }
}

// ── composition rule: root_composition_missing_data_start ─────────────────
//
// The root composition element needs data-start="0" for the runtime to begin
// playback.

function ruleRootCompositionMissingDataStart(ctx, report) {
  const rootEl = ctx.doc.querySelector('[data-composition-id]');
  if (!rootEl) return;
  const compId = rootEl.getAttribute('data-composition-id');
  if (!compId) return;
  const hasStart = readAttr(rootEl, 'data-start') !== null;
  if (!hasStart) {
    push(report, {
      severity: 'warning',
      rule_id: 'comp/root-composition-missing-data-start',
      message: `Root composition "${compId}" is missing data-start. The runtime needs data-start="0" on the root element to begin playback.`,
      fix_hint: 'Add data-start="0" to the root composition element.',
      snippet: truncateSnippet(rootEl.outerHTML),
    });
  }
}

// ── composition rule: root_composition_missing_data_duration ──────────────
//
// Without an explicit duration the runtime may infer Infinity.

function ruleRootCompositionMissingDataDuration(ctx, report) {
  const rootEl = ctx.doc.querySelector('[data-composition-id]');
  if (!rootEl) return;
  const compId = rootEl.getAttribute('data-composition-id');
  if (!compId) return;
  const hasDuration = readAttr(rootEl, 'data-duration') !== null;
  if (!hasDuration) {
    push(report, {
      severity: 'warning',
      rule_id: 'comp/root-composition-missing-data-duration',
      message: `Root composition "${compId}" is missing data-duration. Without an explicit duration, the runtime may infer Infinity for compositions with repeating animations, causing playback issues.`,
      fix_hint:
        'Add data-duration="X" to the root composition element, where X is the total duration in seconds.',
      snippet: truncateSnippet(rootEl.outerHTML),
    });
  }
}

// ── composition rule: standalone_composition_wrapped_in_template ──────────
//
// Only sub-compositions should be wrapped in <template>; standalone root
// index.html must not be.

function ruleStandaloneCompositionWrappedInTemplate(ctx, report) {
  // isSubComposition is not part of our LintContext; infer from raw HTML.
  // A sub-composition's raw source starts with <template. A standalone
  // composition that also starts with <template is flagged.
  const trimmed = ctx.raw.trimStart().toLowerCase();
  // If it IS a sub-composition (starts with <template), do not flag it.
  // We infer standalone vs sub-composition: if the HTML has a <!doctype or
  // <html wrapper, it is standalone. If it starts with <template, it could
  // be either; we flag it only when it also contains data-composition-id
  // (indicating it was intended as a standalone but wrapped).
  if (!trimmed.startsWith('<template')) return;
  // Only flag if this looks like it was meant to be standalone
  // (has doctype-like content or data-composition-id outside template context).
  // Per upstream: any root index.html starting with <template is flagged.
  // We treat all inputs as potentially standalone (caller sets composition_id).
  push(report, {
    severity: 'warning',
    rule_id: 'comp/standalone-composition-wrapped-in-template',
    message:
      'Root composition HTML is wrapped in a <template> tag. ' +
      'Only sub-compositions loaded via data-composition-src should use <template> wrappers. ' +
      'The runtime cannot play a standalone composition inside a template.',
    fix_hint:
      'Remove the <template> wrapper. Use <!DOCTYPE html><html>...<div data-composition-id>...</div>...</html> instead.',
  });
}

// ── composition rule: root_composition_missing_html_wrapper ───────────────
//
// A standalone composition that contains data-composition-id but no proper
// HTML document structure will fail in browsers and the bundler.

function ruleRootCompositionMissingHtmlWrapper(ctx, report) {
  const trimmed = ctx.raw.trimStart().toLowerCase();
  // Sub-compositions (starting with <template) are caught by the previous rule.
  if (trimmed.startsWith('<template')) return;
  const hasDoctype = trimmed.startsWith('<!doctype') || trimmed.startsWith('<html');
  const hasComposition = ctx.raw.includes('data-composition-id');
  if (hasComposition && !hasDoctype) {
    const rootEl = ctx.doc.querySelector('[data-composition-id]');
    push(report, {
      severity: 'error',
      rule_id: 'comp/root-composition-missing-html-wrapper',
      message:
        'Composition starts with a bare element instead of a proper HTML document. ' +
        'An index.html that contains data-composition-id but no <!DOCTYPE html>, <html>, or <body> ' +
        'is a fragment — browsers quirks-mode it, the preview server cannot load it, and ' +
        'the bundler will fail to inject runtime scripts.',
      fix_hint:
        'Wrap the composition in <!DOCTYPE html><html><head><meta charset="UTF-8"></head><body>...</body></html>.',
      snippet: rootEl ? truncateSnippet(rootEl.outerHTML) : undefined,
    });
  }
}

// ── composition rule: requestanimationframe_in_composition ─────────────────
//
// requestAnimationFrame runs on wall-clock time, not the composition timeline,
// causing desync during frame capture.

function ruleRequestAnimationFrameInComposition(ctx, report) {
  for (const script of ctx.scripts) {
    const content = script.textContent || '';
    // Strip comments to avoid false positives
    const stripped = content.replace(/\/\/.*$/gm, '').replace(/\/\*[\s\S]*?\*\//g, '');
    if (/requestAnimationFrame\s*\(/.test(stripped)) {
      push(report, {
        severity: 'warning',
        rule_id: 'comp/requestanimationframe-in-composition',
        message:
          '`requestAnimationFrame` runs on wall-clock time, not the GSAP timeline. It will not sync with frame capture and may cause flickering or missed frames during rendering.',
        fix_hint:
          'Use GSAP tweens or onUpdate callbacks instead of requestAnimationFrame for animation logic.',
        snippet: truncateSnippet(content),
      });
    }
  }
}

// ── export ─────────────────────────────────────────────────────────────────

export default [
  ruleTimedElementMissingVisibilityHidden,
  ruleDeprecatedAttributes,
  ruleTemplateLiteralSelector,
  ruleExternalScriptDependency,
  ruleTimedElementMissingClipClass,
  ruleOverlappingClipsSameTrack,
  ruleRootCompositionMissingDataStart,
  ruleRootCompositionMissingDataDuration,
  ruleStandaloneCompositionWrappedInTemplate,
  ruleRootCompositionMissingHtmlWrapper,
  ruleRequestAnimationFrameInComposition,
];
