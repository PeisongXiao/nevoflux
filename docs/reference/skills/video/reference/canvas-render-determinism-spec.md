# Canvas Render Determinism 规范

> Status: 规范（normative）
> 版本：v1.2
> 日期：2026-04-18
> 关联文档：`hyperframes-integration-design.md` §3.4、§2.4、§5.5

**v1.2 变更**：
- 新增 §3.12 TTS 与音频编码的确定性约束（Kokoro / whisper-tiny / ElevenLabs passthrough）
- §3.3 补充：`repeat: -1` 架构级禁令与有限 repeat 替代模式

**v1.1 变更**：
- §3.8 Window & viewport：在设计文档 v1.2 引入 iframe 尺寸策略后，`window.innerWidth` 不再需要 patch，规则相应放松；新增 `matchMedia` 打桩规则

---

## 1. 目的

NevoFlux composition 渲染出的视频必须**确定性**——相同 composition artifact + 相同 render options + 相同版本的 Canvas runtime，连续渲染两次产出的 MP4 文件应当**byte-identical**（或至少 frame-wise 像素级等价，容忍字体渲染器的 platform 差异）。

这条约束对三件事至关重要：

1. **CI 回归测试**：golden composition 的 SHA256 对比，偏离即失败
2. **用户信任**：用户看到 preview 效果，render 出来不一致会丧失对工具的信心
3. **并发/重试安全**：用户点 render 两次不应得到两个不同版本

确定性不是免费的——浏览器默认行为几乎每一处都违反确定性（random、wall-clock、vsync、async asset、font fallback……）。本规范枚举所有需要 patch 的源头，并固化 Canvas runtime 在 render 模式下对 composition iframe 注入的契约。

---

## 2. 模式区分

| 模式 | 确定性要求 | patch 强度 |
|------|-----------|-----------|
| **preview**（用户交互 scrub） | 不要求 | 不注入任何 patch，保持浏览器默认行为 |
| **render**（产出 MP4） | 强制要求 | 全部 patch 注入，违规即 lint 报错或运行时抛错 |

这意味着 composition 作者写代码时必须假设"render 模式下 `Math.random()` 返回 seeded 值"，**不能依赖 preview 下的实际随机性做视觉效果设计**。

---

## 3. 确定性规则清单

### 3.1 Randomness（随机性）

**规则**：`Math.random` 必须 seeded。

**注入**：
```javascript
// render 模式下，Canvas runtime 注入 composition iframe
const seed = composition.seed ?? 42;  // composition 元数据可指定
const rng = mulberry32(seed);
Object.defineProperty(iframe.contentWindow.Math, 'random', {
  value: rng,
  writable: false,
});
```

**composition 作者约束**：
- 允许调用 `Math.random()`——patched 版本是 deterministic
- 禁止调用 `crypto.getRandomValues()` / `crypto.randomUUID()`——linter 报错
- 禁止引入第三方随机库（如 `nanoid`、`uuid/v4`）——linter 报错

**确定性 `Math.random` 实现**：Mulberry32（Canvas runtime 内置）。不用 Math.sin / Math.floor 式土法 RNG（周期短、分布差）。

### 3.2 Time & clocks（时间与时钟）

**规则**：所有时间源必须指向时间轴当前时刻，不是 wall-clock。

**注入**：
```javascript
// render 循环每帧更新 t（秒）前，patch 所有时间源
const baseWallClock = 1700000000000;  // 任意固定基准，仅用于满足 API 返回 unix ts
iframe.contentWindow.Date.now = () => baseWallClock + (t * 1000);
iframe.contentWindow.performance.now = () => t * 1000;
iframe.contentWindow.Date = new Proxy(iframe.contentWindow.Date, {
  construct: (target, args) => {
    if (args.length === 0) return new target(baseWallClock + t * 1000);
    return new target(...args);
  },
});
```

**composition 作者约束**：
- 允许：`Date.now()`、`performance.now()`、`new Date()`——patched 后返回时间轴时刻
- 禁止：读取时区、Locale（`navigator.language` 除外但必须显式固定）
- 禁止：`setTimeout` / `setInterval` 做动画驱动（要用 timeline 或 GSAP）——linter 报错

### 3.3 GSAP ticker

**规则**：GSAP 内置 ticker 必须交出控制权给 NevoFlux timeline。

**注入**（在 composition 引入 GSAP 后、任何 animation 创建前）：
```javascript
gsap.ticker.fps(composition.fps);        // 固定 fps
gsap.ticker.lagSmoothing(false);          // 关闭延迟平滑（wall-clock 依赖）
gsap.ticker.remove(gsap.ticker._listeners[0]);  // 停用 RAF 自驱动
// 由 timeline adapter 每帧主动推进：
gsap.updateRoot(t);
```

**composition 作者约束**：
- 必须：使用 `gsap.timeline()` 并推入 `window.__timelines` 数组
- 必须：所有 timeline 创建时带 `{ paused: true }`——否则 GSAP 自驱动，破坏 render 确定性
- 禁止：`gsap.ticker.add(myRenderLoop)` 自己接管渲染——linter 报错
- 禁止：使用 `ScrollTrigger` / `ScrollSmoother` / `Observer` / `Draggable`——依赖用户输入，render 无意义
- 禁止：`Physics2D` 插件用 wall-clock delta；如需物理，必须显式 `timeStep: 1/fps` 固定步长
- **禁止：任何 timeline / tween 用 `repeat: -1`**（架构级硬规则）

**关于 `repeat: -1` 的架构级禁令**：

Canvas timeline 驱动 GSAP 的方式是**每帧主动推进 root time**（`gsap.updateRoot(t)`）。如果某个 tween 声明 `repeat: -1`（无限重复），则：

- 该 tween 的内部"结束时间"为无穷大
- timeline 的总 duration 无法被 GSAP 正确计算（返回 Infinity 或未定义）
- 若 render 管线依赖 `timeline.duration()` 做边界检查，将进入**无限循环或 NaN 状态**
- `timeline.seek(t)` 对 `repeat:-1` tween 的行为在 GSAP 内部是未定义的边界情况

**唯一合法的重复动画写法**：计算出确定的 repeat 次数：

```javascript
// ❌ 禁止
gsap.to(el, { rotation: 360, duration: 2, repeat: -1, ease: 'none' });

// ✅ 正确：按 composition.duration 算出有限 repeat
const compDuration = 10;  // 从 stage metadata 读
const cycleDuration = 2;
const repeats = Math.ceil(compDuration / cycleDuration) - 1;
gsap.to(el, { rotation: 360, duration: cycleDuration, repeat: repeats, ease: 'none' });
```

**Linter 规则**：扫描 `repeat:` 后跟 `-1` 或 `Infinity` → ERROR。

### 3.4 Three.js / WebGL

**规则**：WebGL canvas 的内容必须在 `drawWindow()` 捕获时仍然可读；骨骼动画必须基于时间轴而非 delta。

**必须的 Three.js 初始化参数**：
```javascript
const renderer = new THREE.WebGLRenderer({
  preserveDrawingBuffer: true,  // 否则 drawWindow() 拿到空白
  antialias: true,
  alpha: false,  // 建议 false，减少合成层复杂度
});
// 禁止 setAnimationLoop，由 timeline adapter 驱动
```

**必须的 renderer 注册**：
```javascript
// Three.js adapter 通过这个数组发现需要渲染的场景
window.__threeRenderers = window.__threeRenderers || [];
window.__threeRenderers.push({ renderer, scene, camera });
```

**SkinnedMesh 骨骼动画**：
```javascript
// 正确：基于时间轴
const mixer = new THREE.AnimationMixer(model);
const action = mixer.clipAction(animationClip);
action.play();

// 在 timeline.onFrame 里：
window.NevofluxSDK.timeline.onFrame((t) => {
  mixer.setTime(t);  // ✅ 确定性
  // mixer.update(delta) ❌ 错误，依赖 wall-clock delta
});
```

**Asset 加载**：
```javascript
// 所有异步加载必须推入 __readyPromises
window.__readyPromises = window.__readyPromises || [];
const loader = new THREE.GLTFLoader(manager);
window.__readyPromises.push(new Promise((resolve) => {
  loader.load('/assets/model.glb', (gltf) => {
    scene.add(gltf.scene);
    resolve();
  });
}));
// render 循环启动前会 Promise.all(window.__readyPromises)
```

**禁止的 Three.js API**：
- `OrbitControls` / `DragControls` / `TransformControls` / `PointerLockControls`——交互控件
- `renderer.setAnimationLoop(...)`——自驱动渲染
- `Stats.js` / `dat.gui`——调试浮层
- `XRFrame` / WebXR——VR/AR 不支持

Linter 扫描 `window.` 未声明对象、禁止 API 导入、`mixer.update(` 字样。

### 3.5 Media elements（video / audio）

**规则**：`<video>` 和 `<audio>` 元素的 `currentTime` seek 必须 await `seeked` 事件。

**timeline adapter 实现**：
```javascript
async function seekMedia(element, tNow) {
  const target = tNow - parseFloat(element.dataset.start);
  if (target < 0 || target > parseFloat(element.dataset.duration)) {
    element.pause();
    return;
  }
  element.currentTime = target;
  await new Promise((resolve) => {
    const onSeeked = () => {
      element.removeEventListener('seeked', onSeeked);
      resolve();
    };
    element.addEventListener('seeked', onSeeked);
    setTimeout(resolve, 2000);  // 保底超时
  });
}
```

**composition 作者约束**：
- 必须：每个 `<video>` / `<audio>` 有 `data-start` + `data-duration`
- 禁止：DRM 媒体（`<video src="encrypted:...">`）
- 禁止：HLS / DASH 流媒体——只允许 progressive mp4/webm/mp3
- 必须：所有 media 通过 VirtualFS 或 data URL 提供，**禁止外部 URL**

### 3.6 Fonts（字体）

**规则**：Render 启动前必须 `document.fonts.ready`；第一次 frame 捕获前双 RAF。

**注入**：
```javascript
// render 前置
await iframe.contentWindow.document.fonts.ready;
```

**composition 作者约束**：
- 必须：字体通过 `@font-face` 从 VirtualFS 加载，不走 Google Fonts / Adobe Fonts
- 建议：使用 system font stack 或 bundled 开源字体（Inter、Noto Sans SC、思源黑体）
- 注意：不同平台（macOS / Windows / Linux）字体 renderer 不同，会造成 1-2% 像素差异——**golden 测试允许 1% 容差**

### 3.7 Asset loading（资源加载）

**规则**：所有异步资源（图片、视频、3D 模型、字体）必须在 render 启动前 resolve。

**契约**：
```javascript
// composition 主脚本必须遵守：
window.__readyPromises = [];

// 图片：
const img = new Image();
window.__readyPromises.push(new Promise((r) => {
  img.onload = r;
  img.src = 'assets/hero.png';
}));

// 自定义 fetch：
window.__readyPromises.push(
  fetch('assets/data.json').then(r => r.json()).then(storeIt)
);
```

**Canvas runtime 前置**：
```javascript
const promises = iframe.contentWindow.__readyPromises || [];
await Promise.all(promises);
// 再加 500ms buffer 等 decode
await new Promise(r => setTimeout(r, 500));
```

**composition 作者约束**：
- 所有 asset 引用相对路径，走 VirtualFS
- 禁止 `fetch('https://...')` 外部 URL——linter 报错
- CDN 仅限白名单（esm.sh、unpkg 的 pinned 版本），由 Canvas runtime 预加载

### 3.8 Window & viewport（窗口与视口）

**前置条件**：本节规则依赖 `hyperframes-integration-design.md` §2.4（iframe 尺寸策略）——iframe 的物理 CSS 尺寸**始终等于** composition 原生分辨率，UI 通过 `transform: scale()` 做视觉缩放而非改变 iframe 本身。

在该前提下，`window.innerWidth` / `innerHeight` 在 preview 和 render 模式**都是可靠的**——都等于原生分辨率。这是 v1.2 架构修正的红利：不需要 patch innerWidth。

**规则**（放松后）：composition 推荐使用 `#stage` 的 `data-width` / `data-height` 读取画幅（显式 + HyperFrames 兼容），但 `window.innerWidth` / `innerHeight` 也是合法可靠的来源。

**注入**（render 模式）：
```javascript
// 仍需要固定 devicePixelRatio 保证跨平台一致
iframe.contentWindow.devicePixelRatio = 1;

// innerWidth / innerHeight 不需要 patch——§2.4 已保证它们等于原生分辨率
// matchMedia 如果被调用，应返回固定值
iframe.contentWindow.matchMedia = (query) => ({
  matches: false,  // 统一返回 false，避免 media query 响应式分支
  media: query,
  addEventListener: () => {},
  removeEventListener: () => {},
  onchange: null,
});
```

**composition 作者约束**：
- 推荐：`document.getElementById('stage').dataset.width`（显式、HyperFrames 兼容）
- 允许：`window.innerWidth` / `innerHeight`（得益于 §2.4 的 iframe 尺寸策略）
- 禁止：`window.matchMedia` 做响应式分支（返回固定值，分支永不进入）
- 禁止：`window.screen.*`（指向宿主屏幕，与 composition 无关）
- 禁止：媒体查询 CSS `@media (max-width: ...)` 响应式样式——composition 只有一个尺寸，不需要响应式
- 禁止：读取 `devicePixelRatio` 做 hi-DPI 分支（已固定为 1）

**linter 行为**：
- `window.screen.` → ERROR
- `window.matchMedia(` 带返回值分支 → WARN
- CSS `@media (` → WARN
- `devicePixelRatio` 读取 → WARN（因为返回值固定 1，通常意味着作者在解决一个已经不存在的问题）

### 3.9 CSS @keyframes

**规则**：CSS `@keyframes` 动画在 render 中**不确定性高**，不禁止但发 linter 警告。

**原因**：CSS 动画由浏览器合成器驱动，与 JS 主线程异步；scrub 时的状态不可预测。

**推荐**：agent 应优先用 GSAP；`/video` skill 里明确说明。

**允许豁免**：静态元素的 CSS transition（hover 不生效、仅初始状态定义）不受影响。

### 3.10 Physics（物理模拟）

**规则**：任何物理模拟必须固定时间步，禁止 wall-clock delta。

**GSAP Physics2D**：
```javascript
// 错误：
gsap.to(el, { physics2D: { velocity: 500, angle: -60, gravity: 500 } });
// 正确：通过 timeline.onFrame 每帧 step 一次
```

**Matter.js / Planck.js 等第三方**：
```javascript
// 每帧固定 step
const dt = 1 / composition.fps;
window.NevofluxSDK.timeline.onFrame(() => {
  Matter.Engine.update(engine, dt * 1000);
});
```

**composition 作者约束**：
- 物理初始状态必须 seeded（位置、速度全部显式，不可随机）
- 碰撞事件必须 deterministic（依赖 seeded 顺序）

### 3.11 Network & external resources

**规则**：render 期间**不得**发起任何网络请求。

**原因**：网络响应不确定，时序不可控，破坏确定性。

**注入**（render 模式）：
```javascript
iframe.contentWindow.fetch = () => Promise.reject(
  new Error('fetch() 在 render 模式下被禁用；请在 __readyPromises 里预加载')
);
iframe.contentWindow.XMLHttpRequest = class {
  constructor() { throw new Error('XHR 在 render 模式下被禁用'); }
};
iframe.contentWindow.WebSocket = class {
  constructor() { throw new Error('WebSocket 在 render 模式下被禁用'); }
};
```

**composition 作者约束**：
- 所有远程数据必须在 render 启动前由外部注入（例如 agent 把 JSON 作为 `<script id="data" type="application/json">` 嵌入 HTML）
- CDN 脚本由 Canvas runtime 在 iframe 初始化阶段注入，composition 只能 import，不能 fetch

### 3.12 TTS 与音频编码（D11）

**规则**：所有 TTS 生成必须在 render 启动**前**完成；音频 bytes 存入 composition VirtualFS；render 时只做 passthrough 编码。

**三路 TTS 的确定性来源**：

**Kokoro 本地推理**：
- ONNX Runtime 在 CPU 上以固定 seed 推理 → **完全确定性**
- 需要强制：`ort-rs` session 配置 `deterministic_compute_mode = true`
- G2P 阶段（`espeak-ng` / `jieba-rs`）是纯函数，相同输入产出相同音素
- WAV 封装是无损的（24kHz mono PCM → RIFF），byte-identical

**whisper-tiny 转录**：
- 给定 beam_size + temperature=0 → 确定性
- 强制配置：`whisper_decode_options { temperature: 0.0, beam_size: 5, best_of: 1 }`
- 同一个 audio bytes 输入产出相同 transcript（含 word-level timestamps）

**ElevenLabs API**：
- ⚠️ 服务端非确定性——相同 text + voice_id 可能产出略微不同的 audio bytes
- **处理策略**：TTS 结果一旦生成就存入 composition VirtualFS 的 `narration.wav`——render 时用这份本地 bytes，不重新调 API
- composition 确定性由"**VirtualFS 内容不变 → 输出不变**" 保证，不依赖 API 确定性
- 用户改词（改 script）会触发重新 TTS → 新 bytes → 新 artifact，这是预期行为

**用户上传音频**：
- 用户上传的 `.mp3` / `.wav` 本身就是确定性 bytes
- `tts_transcribe` 在相同 bytes 上应产出相同 transcript（whisper 确定性保证）

**render 阶段的音频处理**：
1. 从 VirtualFS 读 `narration.wav` / `bgm.mp3`
2. `AudioContext.decodeAudioData(bytes)` → PCM（Web Audio 规范保证确定性）
3. 按 composition duration 裁剪
4. WebCodecs `AudioEncoder`（AAC）编码 → 容器

**AudioEncoder 配置**（影响输出确定性）：
```javascript
encoder.configure({
  codec: 'mp4a.40.2',  // AAC-LC
  sampleRate: 48000,   // 固定，不用源文件的 rate
  numberOfChannels: 2, // 固定为立体声（单声道 upmix）
  bitrate: 128000,     // 128 kbps 固定
});
```

**linter / lint 规则**：
- 扫描 composition HTML 是否引用 `narration.wav` / `narration.mp3` 但 VirtualFS 里不存在 → ERROR
- 扫描是否同时存在 `<video src="...">` + `<audio src="narration.*">` → ERROR（D7 互斥）
- 转录 `transcript.json` 的 word timestamps 必须在 audio duration 范围内 → WARN（可能暗示 transcribe 失败）

---

## 4. 注入机制

Canvas runtime 在以下时机执行注入：

```
render 启动
  ↓
1. iframe 加载 composition HTML
  ↓
2. iframe "load" 事件触发
  ↓
3. 【注入】Math.random / Date / performance / fetch / XHR / WebSocket patch
   【注入】window.innerWidth/Height / devicePixelRatio override
  ↓
4. 【注入】CDN scripts（gsap, three, plugins）按 composition 声明加载
  ↓
5. composition 主脚本执行（在 step 3+4 后；window.__timelines / __threeRenderers / __readyPromises 已 ready）
  ↓
6. 【注入】gsap.ticker 控制权接管（若检测到 gsap 已加载）
  ↓
7. await Promise.all(window.__readyPromises)
  ↓
8. await document.fonts.ready
  ↓
9. +500ms decode buffer
  ↓
10. 开始渲染循环：每帧 timeline.seek(t) → 双 RAF → drawWindow() → VideoFrame
```

所有 patch 在 iframe 销毁时自然释放（无全局副作用）。

---

## 5. 验证协议

### 5.1 Golden 测试

`nevoflux-testing` 新增 `composition-golden/` 目录：

```
composition-golden/
├── 01-static-title/
│   ├── composition.html
│   ├── expected.sha256        # Linux X11 参考哈希
│   ├── expected-macos.sha256  # macOS 参考哈希
│   └── expected-win.sha256    # Windows 参考哈希
├── 02-gsap-fade-sequence/
├── 03-threejs-cube-spin/
├── 04-split-text-animation/
├── 05-morph-svg-transition/
├── 06-media-video-track/
├── 07-media-audio-sync/
├── 08-multi-layer-compose/
├── 09-physics2d-bounce/
└── 10-shader-uniform-anim/
```

### 5.2 CI 策略

- 每次 PR 触发：Linux X11 上渲染全部 10 个 golden，SHA256 对比 `expected.sha256`——必须完全一致
- 每周 nightly：macOS + Windows 也跑，输出 3 份 sha256，人工审核后更新对应 platform 参考
- **同平台连续两次渲染必须 byte-identical**（严格）
- **跨平台允许 1% 像素差异**（字体 renderer 不同）——通过 frame-wise MSE 对比实现

### 5.3 像素差异度量

```python
# CI 脚本伪代码
def compare_videos(actual_mp4, expected_mp4, tolerance=0.01):
    actual_frames = extract_frames(actual_mp4)
    expected_frames = extract_frames(expected_mp4)
    assert len(actual_frames) == len(expected_frames)
    for i, (a, e) in enumerate(zip(actual_frames, expected_frames)):
        mse = ((a - e) ** 2).mean()
        max_mse = 255 ** 2 * tolerance
        assert mse < max_mse, f"frame {i}: MSE {mse} > {max_mse}"
```

---

## 6. Lint 实现（canvas_lint_composition）

Linter 在 Canvas runtime 端运行（不需要 agent 推理），报告：

| 级别 | 规则 | 检测方式 |
|------|------|----------|
| ERROR | `crypto.getRandomValues` / `crypto.randomUUID` | 字符串扫描 |
| ERROR | `fetch(` 或 `XMLHttpRequest` 无被 `__readyPromises` 包裹 | AST scan |
| ERROR | `mixer.update(` | 字符串扫描 |
| ERROR | `new THREE.WebGLRenderer({ ... })` 未包含 `preserveDrawingBuffer: true` | AST scan |
| ERROR | `OrbitControls` / `DragControls` 等交互控件导入 | import scan |
| ERROR | 第三方 CDN 非白名单 | `<script src>` scan |
| ERROR | 外部 URL 资源（`https://` 开头且非白名单） | asset scan |
| WARN | CSS `@keyframes` | CSS parse |
| WARN | `setTimeout` / `setInterval` 超过 100ms（可能是动画） | AST scan |
| WARN | `window.innerWidth` / `innerHeight` 直接读取 | AST scan |
| WARN | 字体声明含 `https://fonts.googleapis.com` | CSS parse |
| INFO | composition 总时长 > 30s（提示渲染耗时） | metadata |
| INFO | WebGL renderer 数量 > 3（提示性能） | scan |

AST 扫描用 `acorn` 或直接在 Canvas runtime 内用浏览器 AST（Firefox 有 `Reflect.parse`）。

---

## 7. Debug workflow（开发者视角）

当 golden 测试出现"连续两次渲染不一致"时，按以下顺序排查：

1. **首先看帧差异位置**：frame 0 不同 → asset 加载；mid-composition 不同 → timeline 或 adapter；仅末尾不同 → encoder flush 问题
2. **查看 composition 是否用了非白名单 API**：运行 `canvas_lint_composition`
3. **对比两次渲染的 RNG 调用次数**：Canvas runtime 可开启 debug 模式记录 `Math.random` 调用栈
4. **检查是否有 `__readyPromises` 遗漏**：开启 debug 模式记录所有未声明的网络加载
5. **Three.js 场景**：确认所有 renderer 都在 `window.__threeRenderers`；确认 `preserveDrawingBuffer: true`
6. **最后兜底**：bisect composition HTML，二分找到引入不确定性的位置

---

## 8. 版本演进

本规范版本 v1.1 对应 `hyperframes-integration-design.md` v1.2（§2.4 iframe 尺寸策略 + drawSnapshot 修正）。

**已发生的修订**：
- v1.0 → v1.1：§3.8 Window & viewport 放松——iframe 尺寸策略使 `window.innerWidth` 自然正确，不再需要 patch；新增 `matchMedia` 打桩

**未来可能的规范修订**：
- v1.1：加入 headless Firefox 渲染器的确定性规则（与浏览器内 WebCodecs 等价性验证）
- v1.2：加入多 audio track 混音的确定性规则
- v2.0：如引入 GPU-accelerated 视频合成（`VideoEncoder` 硬件编码），硬件差异的处理策略

---

*确定性不是一个开关，是一张清单。每加一条允许的 API、每接一个新的动画库，都要重新走一遍这张清单。*
