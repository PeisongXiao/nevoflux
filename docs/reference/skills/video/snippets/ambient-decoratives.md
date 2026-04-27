---
name: ambient-decoratives
purpose: Background decorative layer rules — every scene needs 2-5 ambient decoratives so it doesn't feel empty during entrance staggers
tags: [layout, motion, visual-quality, decoratives, background]
---

# Ambient decoratives

Every scene needs visual depth — persistent decorative elements that stay visible while content animates in. Without these, scenes feel **empty** during entrance staggering: hero text fades in over 0.6s on a flat background, the eye has nothing else to look at, the pacing feels broken.

This is the single biggest delta between "AI-generic" output and intentional video design.

## When to apply

Always, on every scene that has more than ~2 seconds of hero animation. Skip only on:
- Hard cuts where the next frame replaces everything (transitions handle the visual continuity).
- Compositions where the source video itself IS the visual depth (Mode 2 overlays).

## How many

**2-5 per scene.** Fewer than 2 = empty. More than 5 = noise.

## What counts as an ambient decorative

| Type | Example CSS / GSAP |
|---|---|
| Radial glows | accent-tinted, low opacity (4-8%), breathing scale 0.95→1.05 over 4s |
| Ghost text | theme word at 3-8% opacity, very large (200-400px), slow drift y: ±20px over 6s |
| Accent lines | 1px hairline rules, slow opacity pulse 0.2→0.4 over 3s |
| Grain / noise overlay | 2-4% opacity SVG noise pattern, no animation |
| Geometric shapes | offscreen-anchored circles / triangles slowly drifting in |
| Grid patterns | 1px lines at 4-8% opacity, slowly fading sections |
| Thematic decoratives | orbit rings for space, vinyl grooves for music, equalizer bars for audio |

## Hard rules

1. **All decoratives have slow ambient GSAP motion** — breathing scale, slow drift, opacity pulse. Static decoratives feel dead. Cycle ≥ 3s.
2. **Decoratives use `data-track-index` lower than content** so they paint behind. Convention: decoratives 1-4, content 5-10.
3. **Decoratives obey the same `repeat: -1` ban as everything else.** Calculate finite repeats from scene duration: `repeat: Math.ceil(duration / cycleDuration) - 1`.
4. **Decoratives derive their colour from `var(--color-primary)` / `var(--color-accent)`** — never hardcode. They follow the brand layer.
5. **Decoratives don't carry meaning.** If a user reads the screen and the decorative pulls attention away from the hero, it's wrong. The hero animation always wins focus.
6. **Decorative entrance is faster than content entrance** (0.2-0.4s vs 0.6-1.0s) so the scene "settles" before the hero arrives. Or pre-render at start (`gsap.set` baseline state) and only animate the ambient cycle.

## Anti-patterns

- ❌ Hero text fading in on a flat solid colour — even `#0a0a0f` flat = AI-generic.
- ❌ A single static gradient with no motion — feels like a poster, not video.
- ❌ Decoratives at the same z-index as content, fighting for attention.
- ❌ Decoratives using a colour outside the DESIGN.md palette.
- ❌ More than 5 decoratives — competing for the eye, kills the pace.

## Minimum viable scene

```html
<!-- decoratives layer (track 1-4) -->
<div class="ambient-glow" data-track-index="1"></div>
<div class="ambient-ghost-text" data-track-index="2">RUN</div>
<div class="ambient-grain" data-track-index="3"></div>

<!-- content layer (track 5-10) -->
<div class="hero" data-track-index="5">
  <h1 class="title"><<HEADLINE>></h1>
</div>
```

```css
.ambient-glow {
  position: absolute;
  inset: 0;
  background: radial-gradient(circle at 30% 30%,
              color-mix(in oklab, var(--color-primary, #1a6b6b) 15%, transparent),
              transparent 60%);
  pointer-events: none;
}
.ambient-ghost-text {
  position: absolute;
  bottom: 5%;
  right: 5%;
  font-size: 280px;
  font-weight: 900;
  color: var(--color-primary, #1a6b6b);
  opacity: 0.05;
  pointer-events: none;
}
.ambient-grain {
  position: absolute;
  inset: 0;
  background-image: url("data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' width='100' height='100'><filter id='n'><feTurbulence baseFrequency='0.9'/></filter><rect width='100' height='100' filter='url(%23n)' opacity='0.08'/></svg>");
  pointer-events: none;
  mix-blend-mode: overlay;
}
```

```js
// Slow ambient breathing on the radial glow
tl.fromTo('.ambient-glow',
  { scale: 0.95, opacity: 0.6 },
  { scale: 1.05, opacity: 1, duration: 4, ease: 'sine.inOut',
    repeat: Math.ceil(SCENE_DURATION / 4) - 1, yoyo: true },
  0
);

// Slow drift on the ghost text
tl.to('.ambient-ghost-text',
  { y: '+=20', duration: 6, ease: 'sine.inOut',
    repeat: Math.ceil(SCENE_DURATION / 6) - 1, yoyo: true },
  0
);
```

## Lint coverage

Currently the linter does NOT enforce ambient decorative count — adding one would risk false positives on Mode 2 overlays where the source video provides depth. The agent is expected to apply this snippet's rules by reading it via `skill_load("video")` and following the pattern.

When in doubt: open the rendered MP4, pause on a frame between hero entrance and exit, ask "does this frame look intentional, or like a static slide?" If static, add decoratives.
