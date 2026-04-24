---
name: three-js-rules
purpose: Required patterns for Three.js composition templates to work with the capture loop
tags: [three.js, 3d, webgl, determinism]
---

# Three.js 規則 / Three.js Rules

## Renderer Configuration

**RULE 1**: Every `THREE.WebGLRenderer({...})` MUST include `{ preserveDrawingBuffer: true }`.

```javascript
const renderer = new THREE.WebGLRenderer({ preserveDrawingBuffer: true, antialias: true });
```

The capture loop calls `drawWindow()` to snapshot the framebuffer. Without this flag, the buffer clears after presentation, leaving snapshots blank. **Enforced by**: `nf/three-renderer` (ERROR)

## Register with Capture Loop

**RULE 2**: Every renderer MUST be pushed to `window.__threeRenderers`.

```javascript
window.__threeRenderers = window.__threeRenderers || [];
window.__threeRenderers.push({ renderer, scene, camera });
```

The render driver re-draws all registered scenes each frame. Unregistered renderers show stale pixels or disappear. **Enforced by**: `nf/three-register` (WARNING)

## AnimationMixer Determinism

**RULE 3**: Use `mixer.setTime(t)`, NOT `mixer.update(delta)`.

```javascript
// ❌ WRONG: uses wall-clock, nondeterministic
mixer.update(delta);

// ✅ CORRECT: deterministic timeline
mixer.setTime(t);
```

`mixer.update(delta)` computes elapsed time and is vulnerable to frame fluctuations. The render clock is external and deterministic; `setTime(t)` positions animation at the exact timeline moment. **Enforced by**: `nf/mixer-settime` (WARNING)

## Geometry & Assets

- **Geometry**: `BoxGeometry`, `IcosahedronGeometry`, `TextGeometry` for starters
- **GLTF loading**: register promises in `window.__readyPromises`
- **All assets** from `assets/` folder, NOT external URLs

```javascript
window.__readyPromises = window.__readyPromises || [];
window.__readyPromises.push(new Promise((resolve) => {
  new THREE.GLTFLoader().load('assets/model.glb', (gltf) => {
    scene.add(gltf.scene);
    resolve();
  });
}));
```

## Forbidden APIs

**DO NOT use**: `OrbitControls`, `DragControls`, `renderer.setAnimationLoop(...)`, `Stats.js`, or WebXR. Compositions are deterministic playback, not interactive. **Enforced by**: `nf/forbidden-apis` (ERROR)

## Lighting

Use `AmbientLight + DirectionalLight` for deterministic output. Avoid `SpotLight` (shadows nondeterministic without fixed frame time); bake lighting instead.

## Camera & Timeline

All camera motion MUST use GSAP timelines (with `paused: true`) or `timeline.onFrame()`:

```javascript
const tl = gsap.timeline({ paused: true });
tl.to(camera.position, { x: 10, y: 5, z: 20, duration: 2 });
window.__timelines = window.__timelines || [];
window.__timelines.push(tl);
```

Do NOT use mouse input, `requestAnimationFrame` loops, or `CameraControls`.

## Scene Persistence

Keep scenes and renderers in memory throughout render. DO NOT dynamically dispose/recreate or clear `window.__threeRenderers` mid-composition.

## Minimal Template

```html
<canvas id="webgl"></canvas>
<script>
  const scene = new THREE.Scene();
  const camera = new THREE.PerspectiveCamera(75, innerWidth / innerHeight, 0.1, 1000);
  camera.position.z = 5;

  const renderer = new THREE.WebGLRenderer({
    canvas: document.getElementById('webgl'),
    preserveDrawingBuffer: true,  // REQUIRED
  });
  renderer.setSize(innerWidth, innerHeight);

  window.__threeRenderers = window.__threeRenderers || [];
  window.__threeRenderers.push({ renderer, scene, camera });

  const geometry = new THREE.BoxGeometry(1, 1, 1);
  const material = new THREE.MeshBasicMaterial({ color: 0x00ff00 });
  const cube = new THREE.Mesh(geometry, material);
  scene.add(cube);

  const tl = gsap.timeline({ paused: true });
  tl.to(cube.rotation, { x: Math.PI * 2, z: Math.PI * 2, duration: 4 });
  
  window.__timelines = window.__timelines || [];
  window.__timelines.push(tl);
</script>
```
