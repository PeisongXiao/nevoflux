---
name: scene-transition-ironrules
purpose: Iron rules for scene containers and scene-to-scene transitions in compositions
tags: [scene, transition, layout, discipline]
---

# 场景转场铁律 / Scene Transition Iron Rules

## 场景容器规则 / Scene Container Rules

- **DO use `display: flex` + `padding`** for scene containers. The `.scene-content` wrapper must fill its clip bounds with flexbox layout.
- **DON'T use `position: absolute; top: Npx`** on `.scene-*` — this causes content overflow without warning. Content may exceed the scene clip duration or render bounds. Use padding-based layout instead.
- **Component-level elements** (`.feature-list-check`, `.screenshot-reveal`, watermarks, annotations) **MAY use `position: absolute; transform: translate(-50%, -50%)`** — they are decorative layers within a scene and intentionally positioned outside the flex flow.

## 转场 track-index 约定 / Transition Track-Index Convention

Maintain layer separation by track-index:

- **Scenes themselves**: `data-track-index="1"` (main content layers)
- **Scene-to-scene transitions** (wipe, flash, crossfade): `data-track-index="10"` (above scenes, below overlays)
- **Captions / lower-thirds**: `data-track-index="15-20"` (interactive text)
- **Watermarks**: `data-track-index="50"` (topmost, subtle visual)

Keep a 5-slot buffer between categories for future overlays and effects.

## 转场时长 / Transition Duration

Target **0.3–0.8 second** range for all scene-to-scene transitions:

- **< 0.3s** feels like a glitch; the viewer's eye cannot resolve the transition visually.
- **0.3–0.8s** is perceptually smooth and maintains pacing (TikTok / shorts expectation).
- **> 0.8s** feels draggy and violates social-media-native rhythm.

## Overlap 约束 / Overlap Constraints

- **DON'T stack multiple clips on the same `data-track-index` with overlapping time ranges** — this triggers `comp/overlapping-clips-same-track` error and causes rendering conflicts.
- **Solution**: Use different `data-track-index` values OR add sequential `data-start` offsets so clips are strictly time-disjoint on the same track.
- The linter auto-detects overlaps by comparing `data-start` and `data-start + data-duration` across all clips on each track.

## 一致性 / Consistency

- **ONE transition style per composition**. Mixing flash + crossfade + wipe looks amateur and breaks rhythm.
- **For 16:9 (landscape)**: Use wipe-diagonal + crossfade combo for smooth, cinematic pacing.
- **For 9:16 (TikTok / vertical)**: Use flash-through-white for beat drops and content surprises.
- **For 1:1 (square)**: Use crossfade for universal compatibility.

Consistency in transition style reinforces brand identity and keeps pacing predictable to viewers.

## Enforced By

The composition linter flags violations:

- **`comp/overlapping-clips-same-track`** — Flags overlapping `data-start`/`data-duration` on the same `data-track-index`. Fix by offsetting clips in time or assigning different track indices.
- **`comp/timed-element-missing-clip-class`** — Flags elements with `data-start` that don't have `class="clip"` for visibility control.
- **`comp/timed-element-missing-visibility-hidden`** — Warns if a timed element lacks initial hidden state (class or style), risking flash-before-start artifacts.

## Related References

- Scene-to-scene transitions must have **entrance animations** on all elements (§ 转场铁律 Rule 2)
- Transitions are **clip adapter controlled** — do not manually fade out scenes before transitions (§ 转场铁律 Rule 3)
- Each scene must obey **Layout Before Animation** (静态布局优先) — define hero frame CSS first, then add GSAP entrance animations
