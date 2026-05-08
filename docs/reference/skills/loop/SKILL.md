---
name: loop
description: Re-run a prompt or wrapped skill on a time, event, or DOM-state trigger. Each iteration is fresh; persist anything across iterations via the ≤4KB scratchpad.
tools: read,scratchpad-write,event-subscribe
max_iterations: 50
---

# /loop

You are running INSIDE a loop iteration. Behave accordingly.

## Iteration context model

You **do not see previous iterations' messages**. The only memory carried across iterations is the loop's `scratchpad` (≤ 4096 bytes). Treat each iteration as a fresh agent invocation that happens to share a scratchpad with its prior selves.

The system prompt for this iteration begins with a `<LOOP-CONTEXT>` block:

```
<LOOP-CONTEXT>
loop_id=<id>
iteration=<n>
trigger=<trigger expression literal>
fire_reason=<time | event:<topic> | state:tab=…:<selector>>
scratchpad_bytes=<len>
scratchpad:
<verbatim scratchpad content, or "(empty)">
</LOOP-CONTEXT>
```

## Trigger expression quick reference

- `time:<n>{s|m|h|d}` — every interval. Sub-minute rounds up to 1m.
- `time:dynamic` — you choose the next delay; emit a `loop-meta` block (see below).
- `event:<topic-pattern>` — fires when a matching EventBus event publishes.
- `state:tab=current|<id>:<css-selector>:change` — fires when the selector's DOM mutates (deferred — currently no-ops).
- `AND(a,b,…)` / `OR(a,b,…)` — combine; nesting depth ≤ 3.

## Tool permission classes

The loop creator picks an allow-list. Default: `read`, `scratchpad-write`, `event-subscribe`. Anything else (`dom-click`, `nav`, `write`, `net-post`) is opt-in. Tools you call that fall outside the allow-list are rejected with an error — fall back rather than retrying.

## Scratchpad usage

Call `loop.scratchpad.set({ content })` with the **full replacement content**. Bytes ≤ 4096 enforced. `loop.scratchpad.get()` reads it; you can also see it in the system prompt.

Use scratchpad to remember: cursor positions, last-seen IDs, derived state, the next thing you intend to do.

## time:dynamic protocol

Fenced JSON at the end of your output:

\`\`\`loop-meta
{ "next_delay_seconds": 240 }
\`\`\`

Range clamped to [60, 3600]. Missing/unparseable defaults to 300.

## Cancellation and failure

- Three consecutive iteration errors trip auto-cancel (state → `failed`).
- You can self-cancel via `loop.cancel({ loop_id: <your_loop_id_from_LOOP-CONTEXT> })`.
- You **may not** create new loops (`loop.create` is forbidden in iterations) or call `ask_user` (sidebar may be closed; nobody to answer).

## Safety warnings

- The user may not be watching when you fire. **Do not** take irreversible side-effects unless `allowed_tool_classes` was explicitly opened up.
- A force-cancel mid-iteration aborts in-flight tool calls but cannot undo network requests already dispatched.
