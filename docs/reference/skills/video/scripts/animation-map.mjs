#!/usr/bin/env node
// animation-map.mjs — static GSAP timeline analyzer for /video compositions.
//
// Reads a composition HTML file, extracts GSAP tween calls via regex (same
// approach as the composition-linter's gsap.js heuristic), and produces an
// `animation-map.json` report covering:
//
// - **Per-tween summaries**: "#hero-title animates opacity+y over 0.5s starting at 0.3s"
// - **ASCII Gantt chart**: visual timeline of all tweens
// - **Stagger detection**: sequences with consistent intervals
// - **Dead zones**: > 1s gaps with no animation
// - **Element lifecycles**: first/last animation time per selector
// - **Pacing flags**: tweens under 0.2s (paced-fast) or over 2s (paced-slow)
//
// Usage:
//   node animation-map.mjs <composition.html>
//   node animation-map.mjs <composition.html> --json   # JSON only, no Gantt
//   node animation-map.mjs <composition.html> --out report.json
//
// NOTE: This is a static analyzer — it doesn't render the composition.
// It catches choreography problems (stagger inconsistency, dead zones,
// pacing extremes) that the linter's "comp/overlapping-gsap-tweens"
// heuristic doesn't surface. For visual layout audit (bbox overflow,
// element off-frame), use `canvas_inspect_layout` (P5.5 — pending).

import { readFile, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';

const argv = process.argv.slice(2);
if (argv.length === 0 || argv[0] === '--help') {
  console.log('Usage: animation-map.mjs <composition.html> [--json] [--out <path>]');
  process.exit(argv[0] === '--help' ? 0 : 1);
}
const file = argv[0];
const jsonOnly = argv.includes('--json');
const outIdx = argv.indexOf('--out');
const outPath = outIdx > -1 ? argv[outIdx + 1] : null;

const html = await readFile(file, 'utf8');

// ─── Tween extraction (mirrors composition-linter/rules/gsap.js) ────────────

const TIMELINE_VAR_RE = /(?:const|let|var)\s+(\w+)\s*=\s*gsap\.timeline\s*\(/;

/**
 * Extract every tween call from the script. Returns an array of:
 * { method, selector, propsRaw, position, duration, repeat, ease, raw, scriptIdx }
 *
 * Position is parsed from the optional 4th argument:
 *   tl.to(".x", {...}, 0.5)         → position 0.5
 *   tl.to(".x", {...}, "+=0.3")     → relative; we record as { rel: 0.3 }
 *   tl.to(".x", {...})              → position null (chained, sequential)
 */
function extractTweens(scriptText, scriptIdx) {
  const tlMatch = scriptText.match(TIMELINE_VAR_RE);
  if (!tlMatch) return [];
  const tlVar = tlMatch[1];

  // GSAP signatures we care about:
  //   tl.set(sel, vars [, pos])
  //   tl.to(sel, vars [, pos])
  //   tl.from(sel, vars [, pos])
  //   tl.fromTo(sel, fromVars, toVars [, pos])
  //
  // Two-stage match: first locate the call's selector + method, then
  // walk the parens body to collect the literal arguments handling
  // brace depth. This is more robust than the single-shot regex
  // (which misses fromTo's two `{}` blocks).
  const startRe = new RegExp(
    `${tlVar}\\.(set|to|from|fromTo)\\s*\\(\\s*["']([^"']+)["']\\s*,`,
    'g',
  );
  const out = [];
  let m;
  while ((m = startRe.exec(scriptText)) !== null) {
    const method = m[1];
    const selector = m[2];
    const argsStart = m.index + m[0].length;

    // Walk the call's argument list, tracking paren / brace depth.
    let depth = 1;
    let i = argsStart;
    let blockStart = i;
    const args = [];
    let cur = '';
    while (i < scriptText.length && depth > 0) {
      const c = scriptText[i];
      if (c === '(' || c === '{' || c === '[') depth++;
      else if (c === ')' || c === '}' || c === ']') {
        depth--;
        if (depth === 0) break;
      } else if (c === ',' && depth === 1) {
        args.push(cur.trim());
        cur = '';
        i++;
        continue;
      }
      cur += c;
      i++;
    }
    if (cur.trim() !== '') args.push(cur.trim());
    if (depth !== 0) continue; // malformed; skip

    // Pick the "vars" object whose props matter for choreography:
    // - set/to/from: args[0]
    // - fromTo:      args[1] (the toVars carries duration / ease)
    const varsArg = method === 'fromTo' ? args[1] : args[0];
    const posArg = method === 'fromTo' ? args[2] : args[1];
    const propMap = parsePropsLiteral(varsArg || '{}');
    const duration = num(propMap.duration, 0);
    const repeat = int(propMap.repeat, 0);
    const ease = (propMap.ease || '').replace(/['"]/g, '');
    const propKeys = Object.keys(propMap).filter(
      (k) => !['duration', 'ease', 'repeat', 'yoyo', 'overwrite', 'delay', 'immediateRender', 'stagger'].includes(k),
    );

    const position = parsePosition(posArg);
    const raw = scriptText.slice(m.index, i + 1).replace(/\s+/g, ' ').trim().slice(0, 120);

    out.push({
      method,
      selector,
      properties: propKeys,
      duration,
      repeat,
      ease,
      position,
      raw,
      scriptIdx,
    });
  }
  return out;
}

function parsePropsLiteral(text) {
  const out = {};
  const propRe = /(\w+)\s*:\s*("[^"]*"|'[^']*'|-?[\d.]+|true|false|"auto"|'auto'|\w[\w.]*)/g;
  let m;
  while ((m = propRe.exec(text)) !== null) out[m[1]] = m[2];
  return out;
}
function num(v, fallback) {
  if (v == null) return fallback;
  const n = parseFloat(String(v).replace(/['"]/g, ''));
  return Number.isFinite(n) ? n : fallback;
}
function int(v, fallback) {
  if (v == null) return fallback;
  const n = parseInt(String(v).replace(/['"]/g, ''), 10);
  return Number.isFinite(n) ? n : fallback;
}
function parsePosition(raw) {
  if (raw == null) return null;
  const s = String(raw).trim();
  if (s === '') return null;
  // Numeric absolute: 0.5 / 2 / 0.3
  if (/^-?[\d.]+$/.test(s)) return { abs: parseFloat(s) };
  // Quoted relative offset: "+=0.3" / "-=0.5" / "<+0.2" / etc.
  const relMatch = s.match(/^["'](?:[<>])?([+-]=)([-\d.]+)["']$/);
  if (relMatch) {
    const sign = relMatch[1] === '+=' ? 1 : -1;
    return { rel: sign * parseFloat(relMatch[2]) };
  }
  // Quoted clip-id reference: "intro+2" / "el-1"
  return { ref: s.replace(/^["']|["']$/g, '') };
}

// ─── Resolve position chain → absolute start time ──────────────────────────

/**
 * Walk through the tween array in source order, computing each tween's
 * absolute start time. Position semantics (per GSAP):
 *   - null:           starts at the timeline's current end (sequential)
 *   - { abs: N }:     absolute time N seconds from timeline start
 *   - { rel: +N }:    N seconds after the current end
 *   - { rel: -N }:    overlap with previous by N seconds
 *   - { ref: id }:    NOT resolved here (composition-id refs need DOM lookup)
 *
 * For unresolvable positions, we record `start: null` so consumers can
 * skip them in the Gantt chart but still see them in the per-tween list.
 */
function resolveStartTimes(tweens) {
  let cursor = 0; // current end of the timeline
  const resolved = [];
  for (const tw of tweens) {
    let start = null;
    if (tw.position == null) {
      start = cursor;
    } else if (typeof tw.position.abs === 'number') {
      start = tw.position.abs;
    } else if (typeof tw.position.rel === 'number') {
      start = cursor + tw.position.rel;
    } else {
      // Unresolvable ref — skip in Gantt
    }
    const end = start != null ? start + tw.duration * (1 + tw.repeat) : null;
    resolved.push({ ...tw, start, end });
    if (end != null && end > cursor) cursor = end;
  }
  return resolved;
}

// ─── Analysis ──────────────────────────────────────────────────────────────

function findStaggers(tweens, tolerance = 0.025) {
  // Group by similar property set; within each group, sort by start, look
  // for ≥3 consecutive tweens with consistent inter-start intervals.
  const groups = new Map();
  for (const tw of tweens) {
    if (tw.start == null) continue;
    const key = tw.properties.slice().sort().join(',');
    if (!groups.has(key)) groups.set(key, []);
    groups.get(key).push(tw);
  }
  const found = [];
  for (const [key, list] of groups) {
    if (list.length < 3) continue;
    list.sort((a, b) => a.start - b.start);
    let runStart = 0;
    for (let i = 2; i < list.length; i++) {
      const d1 = list[i - 1].start - list[i - 2].start;
      const d2 = list[i].start - list[i - 1].start;
      if (Math.abs(d1 - d2) > tolerance) {
        if (i - runStart >= 3) {
          found.push({
            properties: key,
            count: i - runStart,
            interval_ms: Math.round(d1 * 1000),
            first_selector: list[runStart].selector,
            t_start: list[runStart].start,
          });
        }
        runStart = i - 1;
      } else if (i === list.length - 1 && i - runStart + 1 >= 3) {
        found.push({
          properties: key,
          count: i - runStart + 1,
          interval_ms: Math.round(d1 * 1000),
          first_selector: list[runStart].selector,
          t_start: list[runStart].start,
        });
      }
    }
  }
  return found;
}

function findDeadZones(tweens, threshold = 1.0) {
  // Build a sorted list of [start, end] intervals; gaps >= threshold are dead zones.
  const intervals = tweens
    .filter((tw) => tw.start != null && tw.end != null)
    .map((tw) => [tw.start, tw.end])
    .sort((a, b) => a[0] - b[0]);

  if (intervals.length === 0) return [];

  // Merge overlapping
  const merged = [intervals[0].slice()];
  for (let i = 1; i < intervals.length; i++) {
    const [s, e] = intervals[i];
    const last = merged[merged.length - 1];
    if (s <= last[1]) last[1] = Math.max(last[1], e);
    else merged.push([s, e]);
  }

  const zones = [];
  for (let i = 1; i < merged.length; i++) {
    const gap = merged[i][0] - merged[i - 1][1];
    if (gap >= threshold) {
      zones.push({
        start: +merged[i - 1][1].toFixed(3),
        end: +merged[i][0].toFixed(3),
        duration_sec: +gap.toFixed(3),
      });
    }
  }
  return zones;
}

function lifecycles(tweens) {
  const map = new Map();
  for (const tw of tweens) {
    if (tw.start == null || tw.end == null) continue;
    const e = map.get(tw.selector) || { selector: tw.selector, first: tw.start, last: tw.end, count: 0 };
    e.first = Math.min(e.first, tw.start);
    e.last = Math.max(e.last, tw.end);
    e.count++;
    map.set(tw.selector, e);
  }
  return [...map.values()].sort((a, b) => a.first - b.first);
}

function flagPacing(tweens) {
  const flags = [];
  for (const tw of tweens) {
    if (tw.duration > 0 && tw.duration < 0.2)
      flags.push({ kind: 'paced-fast', selector: tw.selector, duration_ms: Math.round(tw.duration * 1000) });
    if (tw.duration > 2.0)
      flags.push({ kind: 'paced-slow', selector: tw.selector, duration_ms: Math.round(tw.duration * 1000) });
  }
  return flags;
}

// ─── Render ASCII Gantt ────────────────────────────────────────────────────

function renderGantt(tweens, totalDuration, width = 60) {
  const visible = tweens.filter((tw) => tw.start != null && tw.end != null);
  if (visible.length === 0 || totalDuration <= 0) return '(no tweens with resolved positions)';

  const lines = [];
  const maxLabel = Math.max(...visible.map((tw) => `${tw.selector} ${tw.properties.join(',')}`.length));
  const labelW = Math.min(maxLabel, 28);

  for (const tw of visible) {
    const startCol = Math.floor((tw.start / totalDuration) * width);
    const endCol = Math.max(startCol + 1, Math.floor((tw.end / totalDuration) * width));
    const bar = ' '.repeat(startCol) + '█'.repeat(Math.min(endCol - startCol, width - startCol));
    const label = `${tw.selector} ${tw.properties.join(',')}`.slice(0, labelW).padEnd(labelW);
    lines.push(`${label} │${bar.padEnd(width)}│ ${tw.start.toFixed(2)}s → ${tw.end.toFixed(2)}s`);
  }
  // Time ruler
  const ruler = ' '.repeat(labelW) + ' └' + '─'.repeat(width) + '┘ 0s → ' + totalDuration.toFixed(2) + 's';
  lines.push(ruler);
  return lines.join('\n');
}

// ─── Main ──────────────────────────────────────────────────────────────────

const scriptRe = /<script[^>]*>([\s\S]*?)<\/script>/gi;
const allTweens = [];
let scriptIdx = 0;
let m;
while ((m = scriptRe.exec(html)) !== null) {
  const body = m[1];
  if (!/gsap\.(timeline|to|from|fromTo|set)/.test(body)) continue;
  allTweens.push(...extractTweens(body, scriptIdx++));
}

const resolved = resolveStartTimes(allTweens);
const totalDuration = Math.max(0, ...resolved.filter((t) => t.end != null).map((t) => t.end));

// Composition data-duration (from <stage data-duration="..."> attr)
const stageDur = (() => {
  const sm = html.match(/<[^>]*id=["']stage["'][^>]*data-duration=["']([\d.]+)["']/);
  return sm ? parseFloat(sm[1]) : null;
})();

const report = {
  file: resolve(file),
  composition_data_duration: stageDur,
  resolved_total_duration: +totalDuration.toFixed(3),
  total_tweens: resolved.length,
  resolved_tweens: resolved.filter((t) => t.start != null).length,
  tweens: resolved.map((tw) => ({
    method: tw.method,
    selector: tw.selector,
    properties: tw.properties,
    start: tw.start != null ? +tw.start.toFixed(3) : null,
    end: tw.end != null ? +tw.end.toFixed(3) : null,
    duration: +tw.duration.toFixed(3),
    repeat: tw.repeat,
    ease: tw.ease,
    summary: summarize(tw),
  })),
  staggers: findStaggers(resolved),
  dead_zones: findDeadZones(resolved),
  lifecycles: lifecycles(resolved).map((l) => ({
    ...l,
    first: +l.first.toFixed(3),
    last: +l.last.toFixed(3),
  })),
  pacing_flags: flagPacing(resolved),
};

function summarize(tw) {
  const props = tw.properties.length ? tw.properties.join('+') : '(meta only)';
  const tdur = tw.duration ? `${tw.duration.toFixed(2)}s` : 'instant';
  const tstart = tw.start != null ? `start ${tw.start.toFixed(2)}s` : 'unresolved-start';
  const ease = tw.ease ? ` ease=${tw.ease}` : '';
  return `${tw.selector} ${tw.method} ${props} (${tdur}, ${tstart}${ease})`;
}

if (jsonOnly || outPath) {
  const json = JSON.stringify(report, null, 2);
  if (outPath) {
    await writeFile(outPath, json);
    console.error(`Wrote ${outPath}`);
  } else {
    process.stdout.write(json);
  }
  process.exit(0);
}

// Human-readable output
console.log(`Animation map for: ${report.file}`);
console.log(`Composition duration: ${report.composition_data_duration ?? '(unknown)'}s | Resolved end: ${report.resolved_total_duration}s`);
console.log(`Tweens: ${report.total_tweens} (${report.resolved_tweens} with resolved positions)`);
console.log('');
console.log('Per-tween summary:');
for (const tw of report.tweens) console.log(`  • ${tw.summary}`);
console.log('');
if (report.staggers.length > 0) {
  console.log('Staggers detected:');
  for (const s of report.staggers) {
    console.log(`  ${s.count} elements stagger ${s.interval_ms}ms on [${s.properties}] starting at ${s.t_start.toFixed(2)}s`);
  }
  console.log('');
}
if (report.dead_zones.length > 0) {
  console.log('Dead zones (>1s gap with no animation):');
  for (const z of report.dead_zones) {
    console.log(`  ${z.start}s → ${z.end}s  (${z.duration_sec}s)`);
  }
  console.log('');
}
if (report.pacing_flags.length > 0) {
  console.log('Pacing flags:');
  for (const f of report.pacing_flags) {
    console.log(`  [${f.kind}] ${f.selector} = ${f.duration_ms}ms`);
  }
  console.log('');
}
console.log('Element lifecycles:');
for (const l of report.lifecycles) {
  console.log(`  ${l.selector.padEnd(28)}  ${l.first.toFixed(2)}s → ${l.last.toFixed(2)}s  (${l.count} tweens)`);
}
console.log('');
console.log('ASCII Gantt:');
console.log(renderGantt(resolved, totalDuration));
