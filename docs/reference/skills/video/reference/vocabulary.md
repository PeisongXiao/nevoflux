# /video Vocabulary 参考

> 用户说什么 → agent 写什么。这份文档是 `/video` skill 的补充参考，
> agent 需要完整词汇映射时通过 `skill_read('video/vocabulary.md')` 加载。

---

## 1. 风格映射（mood → visual language）

| 用户说 | 关键视觉元素 | 推荐配色 | 字体 | 缓动 |
|--------|--------------|---------|------|------|
| **minimal / 极简 / 性冷淡** | 大量留白、单色块、无装饰 | 黑白灰 + 1 个 accent | Inter / Noto Sans SC / Helvetica | `power2.out` |
| **energetic / 活力 / 炸裂** | 饱和色、快节奏（每 1.5-2s 一 beat）、粒子 | 荧光黄/品红/青 | 黑体粗体 | `back.out(2)` |
| **corporate / 商务 / 稳重** | 蓝灰主色、均匀布局、图表 | #1a4d7a + #e0e7ee | Helvetica / Inter | `power2.inOut` |
| **cinematic / 电影感** | 上下黑边 letterbox、景深模糊、慢速 pan | 暗色+金色 | serif / Playfair | `power3.inOut` |
| **warm grain / 复古 / 胶片** | 米黄底色、颗粒噪点、边角渐暗 | #f0e9d2 + #7a5c3e | Serif / Noto Serif | `sine.inOut` |
| **TikTok / 抖音风** | 9:16 竖屏、大字体、弹跳 caption | 黑白 + 荧光黄/品红 | 超粗黑体 | `back.out(2.5)` |
| **Swiss grid / 瑞士风格** | 强网格、红黑、克制 | 白/黑 + 一点红 | Helvetica | `power1.out` |
| **neon / 赛博朋克** | 深色底 + 霓虹色发光 | #000 + #00ffcc + #ff00aa | monospace / Orbitron | `power2.out` |
| **warm pastel / 治愈系** | 柔和粉彩、圆角、手写字体 | #ffd8d8 + #d8ecff | rounded sans-serif | `sine.out` |
| **brutal / 野性 / 反设计** | 粗糙、倾斜、大字块叠加 | 黑白反差 | Helvetica Extra Bold | `none` (linear) |

---

## 2. 动作映射（verb → GSAP recipe）

| 用户说 | GSAP 配方 | 备注 |
|--------|----------|------|
| "淡入 / fade in" | `from: { opacity: 0, duration: 0.8, ease: 'power2.out' }` | 基础款 |
| "淡出 / fade out" | `to: { opacity: 0, duration: 0.6, ease: 'power2.in' }` | |
| "弹入 / pop in" | `from: { scale: 0, duration: 0.6, ease: 'back.out(1.7)' }` | 参数越大越弹 |
| "飞入（左）/ slide in from left" | `from: { x: -100, opacity: 0, duration: 0.7, ease: 'power3.out' }` | |
| "飞入（下）/ rise" | `from: { y: 60, opacity: 0, duration: 0.6, ease: 'power3.out' }` | |
| "打字机 / typewriter" | SplitText + `stagger: 0.05, opacity 0→1` | 用 typewriter component |
| "炸开 / burst / explode" | SplitText + `from: { scale: 0, rotation: random(-90,90), stagger: { from: 'center' } }` | |
| "变形 / morph" | `MorphSVGPlugin.convertToPath` + `gsap.to(shape, { morphSVG: "#target" })` | SVG 专用 |
| "跟路径 / follow path / 沿着..." | `MotionPathPlugin` + `{ motionPath: "#path" }` | |
| "抖动 / shake / wiggle" | `{ rotation: -3, duration: 0.1, yoyo: true, repeat: 5 }` | 或用 CustomWiggle |
| "脉冲 / pulse" | `{ scale: 1.08, duration: 0.4, yoyo: true, repeat: Math.ceil(duration/0.8)-1, ease: 'sine.inOut' }` | CTA 按钮常用。注意 repeat 必须是有限值（禁止 -1） |
| "描绘 / draw / reveal stroke" | `DrawSVGPlugin` + `drawSVG: "0% 100%"` | SVG 线条 |
| "镜头推进 / zoom in" | `camera.position.z` 减小（Three.js）或 `scale: 1.2` | 2D/3D 不同 |
| "镜头拉远 / zoom out" | 反之 | |
| "旋转展示 / spin / rotate" | `{ rotation: 360, duration: 4, ease: 'none' }` | 连续 |
| "打光扫过 / light sweep" | 光源 `position.x` 从一端到另一端 | Three.js |
| "破碎 / shatter" | SplitText 拆字符 + 各字符随机 `x, y, rotation` 散开 | |

---

## 3. 场景映射（noun → structure）

| 用户说 | 推荐结构 | 推荐 template/component |
|--------|----------|-----------------------|
| "标题卡 / title card" | 一个 `.clip` 居中大字 + 3 秒停留 + 淡入淡出 | 自由写 |
| "下标三分之一 / lower third" | 左下或左下区块显示姓名 + 头衔 | `lower-third-corporate` 或 `lower-third-minimal` |
| "片头 / intro" | 品牌 logo + 主标题 + 副标题 + 进入主视频 | `product-intro-16x9` 改造 |
| "片尾 / outro" | logo + URL + CTA + 淡出 | 自由写，2-3 秒 |
| "产品介绍" | 标题 → 卖点 → CTA 三段式 | `product-intro-16x9` / `product-intro-9x16` |
| "3D 产品展示" | Three.js 场景 + 旋转 + 光照 | `product-3d-spin` |
| "3D logo 揭幕" | Three.js 文字 + 光扫过 + 相机运动 | `logo-3d-reveal` |
| "TikTok hook" | 强问句 → 答案 → 尾部钩子 | `tiktok-hook` |
| "视频叠加字幕" | `<video>` + `.clip` 字幕 | `video-overlay` + `caption-subtitle` |
| "标注/指出" | 箭头 + 文字 | `annotation-arrow` |
| "水印 / logo 角落" | 角落文字或 logo + 全程显示 | `watermark-animated` |
| "数据柱状图竞赛" | 多条动画条，排名变化 | `data-chart-bar-race` |
| "折线图 / 趋势图" | SVG 线描绘 + 数值滚动 | `data-chart-line` |
| "转场 / transition" | 两场景之间插入 | `flash-through-white` / `crossfade` / `wipe-diagonal` |
| "数字跳动 / number count" | GSAP tween 一个 { n: 0 } 对象 + onUpdate 写 textContent | 配合 `.toLocaleString()` |
| "字幕条 / subtitle / 翻译" | 底部居中的单行文字 | `caption-subtitle` |
| "弹跳字幕 / 社交字幕" | 大字号 + SplitText | `caption-bouncy` 或 `caption-animated-overlay` |

---

## 4. 节奏建议（duration → pacing）

| 视频总时长 | 每个 beat 长度 | 场景数 | 用途 |
|-----------|---------------|--------|------|
| 5-8s | 1-1.5s | 2-3 | TikTok hook、社交媒体 |
| 10-15s | 2-3s | 3-4 | 短广告、产品 teaser |
| 20-30s | 3-5s | 4-6 | 电视广告、release announcement |
| 45-60s | 5-8s | 6-10 | 短纪录片、pitch 视频 |

**beat** = 每个视觉显著变化点（新场景出现、关键信息跳出）。比 beat 短 → 观众感到混乱；比 beat 长 → 观众滑走。

---

## 5. 颜色预设（常用调色盘）

### 5.1 品牌色预设

| 名字 | 主色 | 副色 | 文字 | 适合 |
|------|------|------|------|------|
| NevoFlux Teal | `#1a6b6b` | `#ffcc33` | `#0b1020` | 品牌默认 |
| 品牌红 | `#d72d3a` | `#ffd84d` | `#fff` | 食品、餐饮 |
| 专业蓝 | `#1a4d7a` | `#e8c870` | `#fff` | B2B、金融 |
| 科技紫 | `#6b3dff` | `#00e0ff` | `#fff` | SaaS、AI |
| 米色暖调 | `#d8b589` | `#3e2c1f` | `#1a120b` | 高端、复古 |
| 赛博黑 | `#0a0a0a` | `#00ff88` | `#fff` | 硬核科技、游戏 |

### 5.2 功能色

- 成功/通过：`#00c9a7`
- 警告：`#ffa628`
- 错误/红点：`#ff3b4a`
- 中性灰阶：`#f7f8fa` / `#e8ecef` / `#a0a8b4` / `#4a5568` / `#1a1f2e`

---

## 6. 文字排版建议

### 6.1 字号梯度（1920×1080 为基准）

- 超大标题（hook 问句、hero）：140-200px
- 主标题：72-100px
- 副标题：40-56px
- 正文：32-40px
- 说明/caption：24-32px
- 水印：24-32px

（9:16 竖屏字号通常放大 1.3×，因为手机上看得近）

### 6.2 中英文字体栈

```css
font-family: system-ui, -apple-system,
             'PingFang SC', 'Microsoft YaHei', /* 中文 */
             'Helvetica Neue', Arial, sans-serif;
```

- PingFang SC：macOS 自带（苹方）
- Microsoft YaHei：Windows 自带（微软雅黑）
- Linux 系统用 system-ui fallback 到 Noto Sans CJK SC（通常预装）

### 6.3 英文粗体的中文 fallback

中文没有真正的"粗体"字重——如果用户要"超粗中文"，用：
- `font-weight: 900` + `-webkit-text-stroke: 3px` 模拟
- 或用特定的粗字体家族：Source Han Sans Heavy、方正粗黑

---

## 7. 常见错误（agent 自查）

- ❌ 把用户说的"3D"默认为 CSS 3D transform——实际上用户通常想要真 3D（Three.js）
- ❌ 把"背景音乐"直接加 `<audio>` 而不检查是否已有 `<video>`（v1 互斥规则）
- ❌ 把"箭头指向运动物体"写成固定位置 annotation——物体动箭头不动，会 glitch
- ❌ composition duration 超过源视频时长（视频叠加场景）
- ❌ 字号没考虑中英差异——英文好看的字号中文会挤
- ❌ 一个 timeline push 多次——应该合并进单个 timeline
- ❌ 忘记设 `paused: true`——GSAP 会立刻自驱动，破坏 Canvas timeline 控制
- ❌ 用 `ScrollTrigger` 或 `setAnimationLoop`——视频渲染没用户、没连续 RAF
- ❌ Three.js 用 `OrbitControls`——渲染时会污染相机状态
- ❌ `window.innerWidth` 乘 `devicePixelRatio`——DPR 固定为 1，表达式无意义

---

*此文档与 SKILL.md 主体保持同步。修订时两边都要改。*
