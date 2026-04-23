---
name: "brand-name"
version: "1.0.0"
description: "Brand identity for a video composition. Edit per-project."

# ─── Google design.md base ─────────────────────────────────────────────
colors:
  primary: "#1a6b6b"        # main brand accent; teal/blue/violet work well
  secondary: "#4a7a7a"      # supporting accent; lighter/darker shade of primary
  accent: "#ff8c42"         # highlight / CTA; one pop color only — use sparingly
  background: "#0b1020"     # composition canvas background; dark recommended
  foreground: "#f5f5f7"     # default text; must pass WCAG AA on {colors.background}

typography:
  hero:
    family: "Inter, 'Noto Sans SC', sans-serif"  # display/headline; max 2 families total
    weight: 700             # 600–900; heavier for impact, 600 for elegant headers
  body:
    family: "Noto Sans SC, Inter, sans-serif"    # CJK-safe fallback required
    weight: 400             # 300–500; 400-500 for captions, 300 for ambient text

spacing:
  xs: "4px"                 # micro-gaps: icon padding, badge spacing
  sm: "8px"                 # intra-component: between icon and label
  md: "16px"                # default padding: cards, buttons
  lg: "24px"                # between distinct text blocks in a scene
  xl: "48px"                # major section separators: hero vertical padding

rounded:
  sm: "4px"                 # subtle: chips, caption boxes, progress bars
  md: "8px"                 # standard: buttons, lower-thirds, input fields
  lg: "16px"                # prominent: feature cards, modal panels

components:
  button:
    bg: "{colors.primary}"
    fg: "{colors.foreground}"
    radius: "{rounded.md}"  # use {rounded.sm} for sharper corporate look
  card:
    bg: "{colors.background}"
    border: "{colors.secondary}"
    radius: "{rounded.lg}"
  caption_box:
    bg: "{colors.primary}"
    fg: "{colors.foreground}"
    radius: "{rounded.sm}"
    padding: "{spacing.sm} {spacing.md}"
  lower_third:
    accent_bar: "{colors.accent}"   # 4 px left border strip
    bg: "rgba(11,16,32,0.82)"       # semi-transparent; adjust alpha 0.70–0.92
    radius: "{rounded.md}"

# ─── Video extensions (optional — defaults apply if omitted) ───────────
# Unknown top-level keys; Google design.md linter treats as warnings, not errors.

motion:
  ease_default: "power2.out"       # GSAP easing; workhorse for most tweens
  ease_entrance: "back.out(1.7)"   # scene entrances; factor 1.2–2.5 controls bounce
  ease_exit: "power2.in"           # element/scene exits; power3.in for snappier cuts
  scene_duration_default: "5s"     # default per-scene duration (2s–10s)
  stagger_default: "0.3s"          # sibling stagger; 0.1s tight / 0.5s loose
  beat_interval: "1.5s"            # visual-beat cadence (1.5s social, 3s corporate)
  layout_before_animation: true    # structural discipline flag; no runtime effect

voice:
  provider: "kokoro"               # kokoro | elevenlabs
  voice_id: "af"                   # kokoro: af|am|bf|bm|zf|zm; elevenlabs: see docs
  speed: 1.0                       # 0.5–2.0; 1.1–1.2 energetic, 0.85–0.9 deliberate
  tone: "neutral"                  # neutral | energetic | warm | clinical
  pronunciation:
    "API": "A P I"                 # per-project phonetic overrides for TTS

aspect:
  default: "16:9"                  # 16:9 | 9:16 | 1:1
  width: 1920                      # 1920×1080 (16:9) / 1080×1920 (9:16) / 1080×1080 (1:1)
  height: 1080
  safe_zones:
    top: "120px"                   # platform UI chrome (YouTube progress bar, TikTok icons)
    bottom: "160px"                # caption reserve; increase to 220 px for 9:16
    sides: "80px"                  # 5 % edge crop safety for YouTube / LinkedIn
---

## 概览 Overview

Copy to VirtualFS root as `DESIGN.md`. Replace all placeholder values; remove comments.
**Personality:** Modern, purposeful, direct. Technical precision with approachable warmth.
Confident tone — conversational for social, polished for demos. No jargon.
**Audience:** Technically literate viewers who value clarity. They skip padded content.
**Philosophy:** Information is the visual. Every element earns its place. When in doubt, cut.

## 配色 Colors

| Token | Hex | Usage |
|-------|-----|-------|
| `{colors.primary}` | `#1a6b6b` | Buttons, progress bars, active indicators |
| `{colors.secondary}` | `#4a7a7a` | Hover states, secondary labels |
| `{colors.accent}` | `#ff8c42` | One CTA highlight per scene — never two |
| `{colors.background}` | `#0b1020` | Canvas, card base, overlay fill |
| `{colors.foreground}` | `#f5f5f7` | Default text, caption boxes, icon fills |

One `{colors.accent}` focal point per scene. For cooler moods swap accent to `#e8c870`
(see *minimal* / *corporate* rows in `vocabulary.md`).

## 字体 Typography

Two families maximum; both include CJK fallbacks. Display leads at 72 px+; body handles
everything else. Size scale (1920×1080; × 1.3 for 9:16):

| Role | px range |
|------|----------|
| Hero / hook question | 140–180 px |
| Section title | 72–100 px |
| Subtitle / bullet heading | 44–56 px |
| Body text | 28–36 px |
| Caption / lower-third label | 20–28 px |

Use `font-variant-numeric: tabular-nums` on animated counters.

## 布局 Layout

8-column grid derived from `{spacing.*}`.

| Token | Value | When to use |
|-------|-------|-------------|
| `{spacing.xs}` | 4 px | Icon-to-label gap |
| `{spacing.sm}` | 8 px | Intra-component line gaps |
| `{spacing.md}` | 16 px | Default card / button padding |
| `{spacing.lg}` | 24 px | Between text blocks |
| `{spacing.xl}` | 48 px | Hero vertical padding |

Content must stay inside `{aspect.safe_zones}`; increase sides to 120 px for 9:16.

## 立体感 Elevation & Depth

Flat-first. Elevation via opacity + thin borders, not drop shadows (H.264 noise).

- **Level 0 (canvas):** `{colors.background}` solid.
- **Level 1 (cards):** + `1px solid {colors.secondary}` at 40 % opacity.
- **Level 2 (overlays):** `rgba(11,16,32,0.82)` layered over Level 0.
- **Glow accent** (once per scene): `box-shadow: 0 0 24px {colors.primary}` at 30 %.

Avoid full-screen linear gradients on dark backgrounds — H.264 banding visible >10 s.
Use radial gradients or solid fills with a localized glow.

## 形状 Shapes

Rounded rectangles only. No circles (too playful), no 0 px sharp corners (too legacy).
Larger surface area → larger radius. Full-bleed backgrounds use 0 px.

| Token | Value | Applied to |
|-------|-------|------------|
| `{rounded.sm}` | 4 px | Caption boxes, chips, progress bars |
| `{rounded.md}` | 8 px | Buttons, lower-thirds |
| `{rounded.lg}` | 16 px | Feature cards, panels |

## 组件 Components

Token-derived; change a token and every component updates.

- **Button:** bg `{components.button.bg}` · fg `{components.button.fg}` · radius `{components.button.radius}`. Focus: `2px solid {colors.accent}`.
- **Card:** bg `{components.card.bg}` · border `1px solid {components.card.border}` · radius `{components.card.radius}` · padding `{spacing.lg}`.
- **Caption Box:** bg `{components.caption_box.bg}` · padding `{components.caption_box.padding}` · radius `{components.caption_box.radius}`. Max-width 80 %; centered.
- **Lower Third:** 4 px left bar `{components.lower_third.accent_bar}` · bg `{components.lower_third.bg}` · radius `{components.lower_third.radius}`. Name w600, role w400.

## 规范 Do's and Don'ts

**DO**
- Use `{colors.accent}` for exactly one focal point per scene.
- Keep all essential content inside `{aspect.safe_zones}` on every platform.
- Set GSAP timelines `paused: true`; let the Canvas renderer drive playback.
- Scale font sizes × 1.3 when switching to 9:16.

**DON'T**
- Apply glassmorphism — H.264 compression amplifies blur noise on dark backgrounds.
- Use more than two typeface families in one composition.
- Set `repeat: -1` on any GSAP timeline — the renderer has no user, only finite time.
- Call `window.innerWidth * devicePixelRatio` — DPR is fixed at 1 in the renderer.
- Use `ScrollTrigger` or `requestAnimationFrame` loops — render engine controls time.
- Use 5+ distinct colors in one scene; information density drops, not rises.

## 动效 Motion

Motion serves pacing, not decoration. Cut any animation that doesn't reveal
information or guide the eye.

| Easing token | GSAP string | When to use |
|--------------|-------------|-------------|
| `{motion.ease_default}` | `power2.out` | Default; safe for 90 % of tweens |
| `{motion.ease_entrance}` | `back.out(1.7)` | Scene entrances, logo reveals |
| `{motion.ease_exit}` | `power2.in` | Element / scene exits |
| — | `sine.inOut` | Finite pulse loops, breathing animations |
| — | `power3.inOut` | Cinematic slow-in / slow-out camera moves |

**Beat:** one visual beat every `{motion.beat_interval}` (social); 3–5 s (corporate/
cinematic). <1 s feels chaotic; >5 s loses social viewers. Stagger siblings by
`{motion.stagger_default}`; tighten to 0.1 s for 5+ items.

**vocabulary.md mood → easing quick-ref** (subset):

| Mood | Easing | Beat |
|------|--------|------|
| minimal / 极简 | `power2.out`, no bounce | 3–5 s |
| energetic / 活力 | `back.out(2)`, SplitText burst | 1.5 s |
| cinematic / 电影感 | `power3.inOut`, letterbox | slow pan |
| corporate / 商务 | `power2.inOut`, chart reveals | 3 s |

## 旁白 Voice

**Tone:** Measured, clear — confident colleague, not sales pitch. Match `{voice.tone}` to
each scene. **Speed:** `{voice.speed}`; 1.1–1.2 energetic; 0.85–0.9 deliberate; 0.4 s
pause after headings. **Provider guide:**

| Condition | Provider |
|-----------|----------|
| Standard narration | `{voice.provider}` (kokoro) |
| High expressiveness needed | elevenlabs |
| Chinese-primary content | kokoro `zf` or `zm` |
| English warm | kokoro `af` (default `{voice.voice_id}`) |

**Pronunciation** (`{voice.pronunciation}`): add overrides for brand names, acronyms,
technical terms. Verify with a 10 s test render first.

## 构图与安全区 Aspect & Safe Zones

Default: `{aspect.default}` at `{aspect.width}` × `{aspect.height}` px. Scale sizes
proportionally when switching aspect ratios.

| Edge | Margin | Reason |
|------|--------|--------|
| Top | `{aspect.safe_zones.top}` | Platform chrome (progress bars, live badges) |
| Bottom | `{aspect.safe_zones.bottom}` | Caption reserve (220 px on 9:16) |
| Sides | `{aspect.safe_zones.sides}` | 5 % edge crop on YouTube / LinkedIn |

**Platform rules:**
- **YouTube 16:9:** Respect side margins; thumbnails crop to 16:9.
- **TikTok / Reels 9:16:** Bottom margin → 220 px; font sizes × 1.3; beat → 1.5 s.
- **Instagram 1:1:** Side margins → 120 px; center key visual; letterbox bars allowed.

**Letterboxing (cinematic):** 2.39:1 bars (~110 px top + bottom on 1080 p) as solid
`#000000` divs above the clip layer — never a CSS filter. No content behind bars.
