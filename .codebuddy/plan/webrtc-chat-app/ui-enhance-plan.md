# UI 视觉升级方案 — Mica 毛玻璃 + 酷炫动画 + 可定制背景

> **版本**：v1 （MVP 范围，2026-05-08 起草）
> **决策组合**：B（Windows 11 Mica 毛玻璃） + C（全界面统一酷炫动画） + Ⅰ→Ⅲ 分阶段
> **当前阶段**：**MVP（批次 1-4 + 批次 8）**，后续再评估是否扩展到 Ⅲ
> **总开关**：✅ 提供 `设置 → 外观 → 启用毛玻璃` 开关（默认 ON，低端设备/减弱透明度偏好自动降级）

---

## 目录

1. [目标与非目标](#1-目标与非目标)
2. [设计原则与红线](#2-设计原则与红线)
3. [总体架构](#3-总体架构)
4. [设计令牌扩展](#4-设计令牌扩展-tokenscss)
5. [毛玻璃分层规范](#5-毛玻璃分层规范)
6. [动画规范](#6-动画规范)
7. [自定义背景规范（完整版 / 后续阶段）](#7-自定义背景规范完整版--后续阶段)
8. [MVP 范围（当前实作）](#8-mvp-范围当前实作)
9. [后续阶段（MVP 之后）](#9-后续阶段mvp-之后)
10. [进度跟踪 Checklist](#10-进度跟踪-checklist)
11. [验收标准](#11-验收标准)
12. [风险与缓解](#12-风险与缓解)
13. [变更日志](#13-变更日志)

---

## 1. 目标与非目标

### ✅ 目标

- 全界面引入 **Windows 11 Mica 风毛玻璃**（薄、有颗粒感、与主题色混合）
- 全界面统一 **酷炫动画语言**（光晕 glow、shimmer、spring 进退、流光扫边、呼吸动效）
- 支持用户 **自定义整体背景**（预设/纯色/渐变/上传图片/模糊/遮罩/双主题背景）
- 不破坏现有无障碍能力（`prefers-reduced-motion` / `prefers-reduced-transparency` / WCAG AA）
- 不拉低 theater 模式帧率（视频播放区严禁叠加昂贵滤镜）

### ❌ 非目标

- 不引入第三方重量级 UI 组件库（thaw / leptonic 等）
- 不替换现有 cascade-layer CSS 架构
- 不重写任何现有 Rust 业务逻辑
- 本次不改动 backend / signaling / webrtc 任何代码

---

## 2. 设计原则与红线

### 2.1 原则

| # | 原则 | 说明 |
|---|---|---|
| P1 | **令牌优先** | 所有新样式必须消费 `--glass-*` / `--anim-*` / `--bg-*` 令牌，不写魔法数字 |
| P2 | **分层复用** | 毛玻璃/动画以工具类（`.glass-l2/l3/l4` / `.anim-*`）形式提供，组件 CSS 引用 |
| P3 | **降级优先** | 先写降级方案（无 backdrop-filter / 减弱动画），再写增强效果 |
| P4 | **组件内聚** | 新组件单文件单组件，命名 PascalCase，文件 kebab-case |
| P5 | **纯英文注释** | CSS / Rust 新增注释一律英文 |

### 2.2 性能红线

| 场景 | 毛玻璃 | 动画 | 理由 |
|---|---|---|---|
| `.app-shell` 全屏 | ❌ 禁止 | ❌ 禁止持续动画 | 影响视频解码 |
| `.top-bar` / `.sidebar` / `.drawer` | ✅ L2 (blur 16px) | ✅ hover/focus 过渡 | 常驻，适度 |
| `.modal` / `.popover` / `.toast` | ✅ L3 (blur 20px) | ✅ spring 进退 | 短暂浮层 |
| `theater subtitle/danmaku settings` | ✅ L4 (blur 20px) | ✅ 氛围 glow | 面积小 |
| 消息气泡 / 按钮 / input | ❌ 不加毛玻璃 | ✅ 微交互 | 高频刷新 |
| `<video>` 播放器覆盖层 | ❌ 不加 | ⚠ 仅必要控件过渡 | GPU 敏感 |

### 2.3 总开关与自动降级

```
设置 → 外观 → 视觉效果
  ├─ [ 切换 ] 启用毛玻璃效果（默认 ON）
  └─ [ 切换 ] 启用酷炫动画（默认 ON）
```

自动降级条件（即便开关为 ON 也强制降级）：

- `prefers-reduced-transparency: reduce` → 毛玻璃降级为实色
- `prefers-reduced-motion: reduce` → 所有非必要动画关闭
- 浏览器不支持 `backdrop-filter` → 降级为半透明背景
- 视口宽度 < 640px → blur 值降一档（省 GPU）

---

## 3. 总体架构

### 3.1 DOM 层级（加入 AppBg 后）

```
<html data-theme=... data-font-scale=... data-glass=on|off data-motion=on|off>
  <body>
    <!-- ▼ 全局 overlay，位置不变 -->
    <ErrorToastContainer />
    <ReconnectBanner />

    <!-- ▼ 新增：应用背景层（position: fixed, z-index: -1） -->
    <AppBg>
      <div class="app-bg__image"/>     <!-- image / gradient / solid -->
      <div class="app-bg__overlay"/>   <!-- contrast compensation -->
    </AppBg>

    <!-- ▼ 现有 app shell，背景变透明，让 AppBg 透出 -->
    <div class="app">
      <Sidebar />          <!-- glass-l2 -->
      <main>
        <TopBar />         <!-- glass-l2 -->
        <HomePage/> …
      </main>
      <SettingsPage />     <!-- glass-l2 drawer -->
      <ToastContainer />   <!-- glass-l3 -->
      <ModalManager />     <!-- glass-l3 -->
      <CallOverlay />
      <IncomingInviteModal />
      <DebugPanel />
    </div>
  </body>
</html>
```

### 3.2 CSS Cascade Layers（扩展后）

```
reset, tokens, base, components, utilities, effects
                                              ↑
                                     新增最高优先级层，
                                     承载 glass / animations / background 工具类
```

`main.css` 新增：
```css
@layer reset, tokens, base, components, utilities, effects;

@import url("./glass.css")       layer(effects);
@import url("./animations.css")  layer(effects);
@import url("./background.css")  layer(effects);
```

### 3.3 文件拓扑（新增/修改）

```
frontend/
├─ styles/
│  ├─ tokens.css             [MODIFY]  +glass/anim/bg tokens
│  ├─ main.css               [MODIFY]  +effects layer + imports
│  ├─ glass.css              [NEW]     .glass-l2/.glass-l3/.glass-l4 工具类
│  ├─ animations.css         [NEW]     keyframes + .anim-* 工具类
│  ├─ background.css         [NEW]     .app-bg / .app-bg__image / .app-bg__overlay
│  └─ components/
│     ├─ top-bar.css         [MODIFY]  接入 glass-l2
│     ├─ sidebar.css         [MODIFY]  接入 glass-l2
│     ├─ drawer.css          [MODIFY]  接入 glass-l2
│     ├─ modal.css           [MODIFY-后续]
│     ├─ toast.css           [MODIFY-后续]
│     └─ theater.css         [MODIFY-后续]
└─ src/
   ├─ app.rs                 [MODIFY]  插入 <AppBg/>，应用 data-glass/data-motion 属性
   └─ components/
      ├─ app_bg.rs           [NEW]     背景渲染组件
      ├─ mod.rs              [MODIFY]  导出 AppBg
      └─ settings_page/
         └─ appearance_section.rs  [MODIFY]  新增 glass/motion 两个开关
```

MVP 阶段 **不涉及** IndexedDB / BackgroundSection / blob 上传（留给后续）。

---

## 4. 设计令牌扩展 (tokens.css)

在现有 `:root` 后新增以下块（全部采用 rgb/a 而非 hex，便于 color-mix）：

```css
/* ── Glass (Windows 11 Mica) ───────────────────────── */
:root {
  --glass-tint-r: 255;
  --glass-tint-g: 255;
  --glass-tint-b: 255;

  --glass-bg-l2: rgb(var(--glass-tint-r) var(--glass-tint-g) var(--glass-tint-b) / 0.55);
  --glass-bg-l3: rgb(var(--glass-tint-r) var(--glass-tint-g) var(--glass-tint-b) / 0.70);
  --glass-bg-l4: rgb(var(--glass-tint-r) var(--glass-tint-g) var(--glass-tint-b) / 0.78);

  --glass-border: rgb(255 255 255 / 0.18);
  --glass-highlight: rgb(255 255 255 / 0.35);   /* top 1px inner highlight */
  --glass-noise-opacity: 0.04;                  /* Mica grain */

  --glass-blur-sm: 12px;
  --glass-blur-md: 16px;
  --glass-blur-lg: 20px;
  --glass-saturate: 180%;
}

[data-theme="dark"] {
  --glass-tint-r: 15;
  --glass-tint-g: 23;
  --glass-tint-b: 42;
  --glass-bg-l2: rgb(var(--glass-tint-r) var(--glass-tint-g) var(--glass-tint-b) / 0.60);
  --glass-bg-l3: rgb(var(--glass-tint-r) var(--glass-tint-g) var(--glass-tint-b) / 0.72);
  --glass-bg-l4: rgb(var(--glass-tint-r) var(--glass-tint-g) var(--glass-tint-b) / 0.82);
  --glass-border: rgb(255 255 255 / 0.08);
  --glass-highlight: rgb(255 255 255 / 0.10);
  --glass-noise-opacity: 0.06;
}

/* ── Animation (Cool) ──────────────────────────────── */
:root {
  --dur-instant: 75ms;
  --dur-fast: 150ms;
  --dur-base: 220ms;
  --dur-slow: 360ms;
  --dur-glow: 2400ms;     /* ambient glow loop */
  --dur-shimmer: 1800ms;

  --ease-out-expo: cubic-bezier(0.16, 1, 0.3, 1);
  --ease-spring: cubic-bezier(0.34, 1.56, 0.64, 1);
  --ease-smooth: cubic-bezier(0.4, 0, 0.2, 1);

  --glow-primary: 0 0 0 1px rgb(59 130 246 / 0.35),
                  0 0 16px 2px rgb(59 130 246 / 0.45);
  --glow-soft:    0 0 24px 4px rgb(59 130 246 / 0.25);
  --shimmer-gradient: linear-gradient(
    100deg,
    transparent 20%,
    rgb(255 255 255 / 0.18) 50%,
    transparent 80%
  );
}

/* ── Background (后续扩展用，MVP 只读默认值) ────────── */
:root {
  --app-bg-mode: preset;           /* preset | solid | gradient | image */
  --app-bg-solid: transparent;
  --app-bg-image: none;
  --app-bg-overlay: rgb(0 0 0 / 0.20);
  --app-bg-blur: 0px;
  --app-bg-gradient: radial-gradient(
    circle at 20% 0%, rgb(59 130 246 / 0.18), transparent 60%
  ), radial-gradient(
    circle at 80% 100%, rgb(236 72 153 / 0.14), transparent 55%
  ), linear-gradient(180deg, #f8fafc, #ffffff);
}

[data-theme="dark"] {
  --app-bg-gradient: radial-gradient(
    circle at 15% 0%, rgb(59 130 246 / 0.22), transparent 55%
  ), radial-gradient(
    circle at 85% 100%, rgb(168 85 247 / 0.18), transparent 50%
  ), linear-gradient(180deg, #0b1120, #0f172a);
}
```

---

## 5. 毛玻璃分层规范

### 5.1 工具类（styles/glass.css）

```css
/* L2 — 常驻面板（top-bar, sidebar, drawer） */
.glass-l2 {
  background: var(--glass-bg-l2);
  backdrop-filter: blur(var(--glass-blur-md)) saturate(var(--glass-saturate));
  -webkit-backdrop-filter: blur(var(--glass-blur-md)) saturate(var(--glass-saturate));
  border: 1px solid var(--glass-border);
  box-shadow: inset 0 1px 0 0 var(--glass-highlight), var(--shadow-md);
}

/* L3 — 浮层（modal, popover, toast, menus） */
.glass-l3 {
  background: var(--glass-bg-l3);
  backdrop-filter: blur(var(--glass-blur-lg)) saturate(var(--glass-saturate));
  -webkit-backdrop-filter: blur(var(--glass-blur-lg)) saturate(var(--glass-saturate));
  border: 1px solid var(--glass-border);
  box-shadow: inset 0 1px 0 0 var(--glass-highlight), var(--shadow-xl);
  border-radius: var(--radius-2xl);
}

/* L4 — 剧场专属（subtitle/danmaku settings） */
.glass-l4 {
  background: var(--glass-bg-l4);
  backdrop-filter: blur(var(--glass-blur-lg)) saturate(var(--glass-saturate));
  -webkit-backdrop-filter: blur(var(--glass-blur-lg)) saturate(var(--glass-saturate));
  border: 1px solid var(--glass-border);
  box-shadow: inset 0 1px 0 0 var(--glass-highlight), 0 10px 40px -10px rgb(0 0 0 / 0.55);
}

/* Mica grain — add subtle noise overlay via pseudo-element */
.glass-l2::before,
.glass-l3::before,
.glass-l4::before {
  content: '';
  position: absolute;
  inset: 0;
  pointer-events: none;
  opacity: var(--glass-noise-opacity);
  background-image: url("data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' width='120' height='120'><filter id='n'><feTurbulence type='fractalNoise' baseFrequency='0.9' numOctaves='2'/></filter><rect width='100%' height='100%' filter='url(%23n)' opacity='0.7'/></svg>");
  mix-blend-mode: overlay;
  border-radius: inherit;
}

/* Fallback: no backdrop-filter support */
@supports not (backdrop-filter: blur(1px)) {
  .glass-l2, .glass-l3, .glass-l4 {
    background: color-mix(in srgb, var(--bg-primary) 88%, transparent);
  }
}

/* User override: transparency off */
html[data-glass="off"] .glass-l2,
html[data-glass="off"] .glass-l3,
html[data-glass="off"] .glass-l4 {
  backdrop-filter: none;
  -webkit-backdrop-filter: none;
  background: var(--bg-elevated);
}

/* A11y: reduced transparency */
@media (prefers-reduced-transparency: reduce) {
  .glass-l2, .glass-l3, .glass-l4 {
    backdrop-filter: none;
    background: var(--bg-elevated);
  }
}

/* Small screens: reduce blur cost */
@media (max-width: 640px) {
  :root {
    --glass-blur-md: 10px;
    --glass-blur-lg: 14px;
  }
}
```

### 5.2 组件接入清单

| 组件 CSS | 层级 | 动作 |
|---|---|---|
| `top-bar.css` | L2 | 在 `.top-bar` 上加 `.glass-l2` class 或复用其样式块 |
| `sidebar.css` | L2 | `.sidebar` 接入 L2 |
| `drawer.css` | L2 | `.settings-drawer` / `.drawer` 接入 L2 |
| `modal.css` | L3 | 后续 |
| `toast.css` | L3 | 后续 |
| `conversation-menu.css` | L3 | 后续 |
| `theater.css` subtitle/danmaku | L4 | 后续 |

---

## 6. 动画规范

### 6.1 关键帧（styles/animations.css）

- `fade-in` / `fade-out`
- `slide-up-in` / `slide-down-out`（modal 进退）
- `scale-pop-in`（toast / popover spring 感）
- `shimmer`（loading skeleton、新消息闪光）
- `glow-pulse`（按钮 focus / 在线点 / 剧场氛围）
- `stream-line`（sidebar 选中项 1px 渐变流光）
- `breathe`（头像状态呼吸）
- `bg-drift`（app 背景渐变缓慢漂移）

### 6.2 工具类

```css
.anim-fade-in     { animation: fade-in var(--dur-base) var(--ease-out-expo) both; }
.anim-pop-in      { animation: scale-pop-in var(--dur-slow) var(--ease-spring) both; }
.anim-slide-up    { animation: slide-up-in var(--dur-slow) var(--ease-out-expo) both; }
.anim-glow-pulse  { animation: glow-pulse var(--dur-glow) ease-in-out infinite; }
.anim-shimmer     { position: relative; overflow: hidden; }
.anim-shimmer::after {
  content: ''; position: absolute; inset: 0;
  background: var(--shimmer-gradient); background-size: 200% 100%;
  animation: shimmer var(--dur-shimmer) linear infinite;
  pointer-events: none;
}
.anim-stream-line::before {
  content: ''; position: absolute; left: 0; top: 0; bottom: 0; width: 2px;
  background: linear-gradient(180deg, transparent, var(--color-primary), transparent);
  background-size: 100% 300%;
  animation: stream-line 3s linear infinite;
}

html[data-motion="off"] [class*="anim-"],
@media (prefers-reduced-motion: reduce) {
  *, *::before, *::after {
    animation-duration: 0.01ms !important;
    animation-iteration-count: 1 !important;
    transition-duration: 0.01ms !important;
  }
}
```

### 6.3 应用矩阵

| 场景 | 动画 |
|---|---|
| 路由切换（home/chat/theater/settings） | View Transitions API（已有，微调时长） |
| Modal 进入 | `anim-pop-in` + backdrop fade |
| Modal 退出 | scale + fade |
| Toast 入场 | `anim-slide-up` + `anim-shimmer` 一次 |
| Sidebar 选中项 | `anim-stream-line`（持续） |
| 头像在线点 | `anim-glow-pulse`（持续 breathe） |
| 按钮 hover | lift + glow（过渡，非循环） |
| 新消息气泡入场 | `anim-fade-in` + 12px translateY |
| AppBg 渐变 | `anim-bg-drift`（conic-gradient 60s 一圈） |

---

## 7. 自定义背景规范（完整版 / 后续阶段）

> **MVP 不实作**；仅保留入口与令牌，让升级可无痛叠加。

### 7.1 数据结构

```rust
// settings/types.rs
pub struct BackgroundSettings {
  pub mode: BackgroundMode,          // Preset / Solid / Gradient / Image
  pub preset_id: Option<String>,     // "aurora" / "mica-blue" / …
  pub solid_color: Option<String>,   // "#0f172a"
  pub gradient: Option<GradientSpec>,
  pub image_blob_key: Option<String>,// IDB key into background_image store
  pub blur_px: u8,                   // 0..=40
  pub overlay_alpha: f32,            // 0.0..=0.8
  pub theme_aware: bool,             // 是否为暗亮各存一套
}
```

### 7.2 持久化

- `localStorage`：`settings_background`（JSON，不含 blob 本体）
- IndexedDB：新增 object store `background_image`，key = `user_bg_light` / `user_bg_dark`，value = `Blob`
- 加载顺序：`main.ts` 启动 → `AppBg` 组件挂载 → 异步拉取 blob → `URL.createObjectURL` → 写入 `--app-bg-image`

### 7.3 UI（BackgroundSection）

位于 `appearance_section.rs` 下方新增一节，包含：

1. Mode Radio：`预设 / 纯色 / 渐变 / 自定义图片`
2. 预设缩略图网格（6 张）
3. Color picker（纯色/渐变起止色）
4. 文件上传（限制：≤ 4 MB，自动压缩到 2560×1440，WebP）
5. Blur slider（0–40 px）
6. Overlay slider（0–80%）
7. 暗亮各存一套开关

---

## 8. MVP 范围（当前实作）

> ⭐ **这是本次立刻动手的内容**，约 600 行改动，含 5 个文件新增 + 7 个文件修改。

### 8.1 批次 1 — Tokens 扩展

- 在 `styles/tokens.css` 追加 §4 定义的 `--glass-*` / `--dur-*` / `--ease-*` / `--glow-*` / `--app-bg-*` 令牌
- 暗色主题对应覆盖块

### 8.2 批次 2 — glass.css

- 新建 `styles/glass.css`，实现 §5.1 全部工具类
- 在 `main.css` 注册 effects layer 并 import

### 8.3 批次 3 — animations.css

- 新建 `styles/animations.css`，实现 §6.1 全部关键帧与 §6.2 工具类
- 在 `main.css` import
- 移除 `main.css` 末尾已有的 `@keyframes fade-in-up / fade-in / slide-in-left / slide-in-bottom`（迁移过去）

### 8.4 批次 4 — background.css + AppBg 组件

- 新建 `styles/background.css`：`.app-bg` / `.app-bg__image` / `.app-bg__overlay` + `@keyframes bg-drift`
- 新建 `src/components/app_bg.rs`：根据 CSS 变量渲染三层 div，接入 theme-aware 默认渐变
- `src/components/mod.rs` 导出 `AppBg`
- `src/app.rs` 在 `Show` 块内的 `.app` 之前插入 `<AppBg/>`
- `.app` 的 `background-color` 改为 `transparent`（让 AppBg 透出）

### 8.5 批次 8 — L2 组件接入（top-bar / sidebar / drawer）

- `styles/components/top-bar.css`：`.top-bar` 背景改为 `var(--glass-bg-l2)` + 引用 `.glass-l2` 风格
- `styles/components/sidebar.css`：`.sidebar` 同上 + 选中项加 `.anim-stream-line`
- `styles/components/drawer.css`：`.settings-drawer` 同上

### 8.6 批次 8b — 总开关

- `appearance_section.rs` 新增：
  - `✨ 启用毛玻璃`（Switch → 写 `html[data-glass=on|off]`，存 `settings_user.glass_enabled`）
  - `🎬 启用酷炫动画`（Switch → 写 `html[data-motion=on|off]`，存 `settings_user.motion_enabled`）
- `settings/types.rs` 新增两个 `bool` 字段（默认 `true`）
- `settings/state.rs` sanitised 里 noop（bool 无需 clamp）
- `app.rs` 新增 Effect 同步两个属性到 `<html>`
- i18n：三语言 JSON 补 `settings.visual_effects` / `settings.enable_glass` / `settings.enable_motion`

### 8.7 批次 8c — 三项门禁

严格按 `用户规则` 顺序执行：

```bash
# 1. cargo check
rm -f /tmp/cargo-check-done /tmp/cargo-check-output.txt
(cargo check 2>&1 | tee /tmp/cargo-check-output.txt; echo $? > /tmp/cargo-check-done) &
while [ ! -f /tmp/cargo-check-done ]; do sleep 10; done

# 2. cargo clippy
rm -f /tmp/cargo-clippy-done /tmp/cargo-clippy-output.txt
(cargo clippy -- -D warnings 2>&1 | tee /tmp/cargo-clippy-output.txt; echo $? > /tmp/cargo-clippy-done) &
while [ ! -f /tmp/cargo-clippy-done ]; do sleep 10; done

# 3. cargo test
rm -f /tmp/cargo-test-done /tmp/cargo-test-output.txt
(cargo test 2>&1 | tee /tmp/cargo-test-output.txt; echo $? > /tmp/cargo-test-done) &
while [ ! -f /tmp/cargo-test-done ]; do sleep 10; done
```

---

## 9. 后续阶段（MVP 之后）

### 9.1 批次 9 — L3 浮层接入

- `modal.css` / `toast.css` / `conversation-menu.css` / `reaction-picker` / `sticker-panel` 接入 `.glass-l3`
- Modal 进退改 `anim-pop-in` / `anim-slide-up`

### 9.2 批次 10 — Theater L4 氛围

- `subtitle-settings-panel` / `danmaku-settings-panel` / `copyright-notice` tooltip 接入 `.glass-l4`
- 剧场页加 `anim-glow-pulse` 氛围（仅非视频区）

### 9.3 批次 11 — 微交互下沉

- 按钮（buttons.css）全量 hover-lift + glow
- 列表项 hover `.anim-stream-line`
- 头像在线点 `.anim-glow-pulse`
- 消息气泡新入场 `.anim-fade-in`

### 9.4 批次 5-7 — 完整自定义背景（Ⅲ 方案）

- `BackgroundSettings` 类型 + 持久化 + IDB `background_image` store
- `BackgroundSection` 完整面板 + 预设/纯色/渐变/上传/模糊/遮罩/暗亮双背景
- 图片压缩 worker（可选）

---

## 10. 进度跟踪 Checklist

### MVP 批次

- [x] 批次 0：方案文档化（本文件）
- [x] **批次 1**：tokens.css 扩展 glass/animation/background 令牌
- [x] **批次 2**：新建 styles/glass.css（含 Mica grain + fallback + 降级）
- [x] **批次 3**：新建 styles/animations.css（关键帧 + 工具类 + motion-off 降级）
- [x] **批次 4**：新建 styles/background.css + src/components/app_bg.rs + 接入 app.rs
- [x] **批次 8**：接入 top-bar / sidebar / drawer 三个 L2 组件 + sidebar 激活项流光
- [x] **批次 8b**：settings/types.rs 增字段 + appearance_section UI + app.rs 属性同步 + i18n
- [x] **批次 8c**：三项门禁（check ✅ → clippy ✅ → test ✅ server 711 条 / ⚠ frontend lib 受 Apple ld bug 阻挡，同历史惯例跳过）

### 后续批次（暂不开启）

- [x] 批次 9：L3 浮层接入（modal / toast / error-toast / conversation-menu / sticker-panel — glass bridge + spring enter）
- [x] 批次 10：Theater L4 氛围（subtitle-settings / danmaku-settings 接入 L4；copyright tooltip 升级为暗色 L4 + spring；theater-page__stage 氛围 glow halo，fullscreen 自动隐藏）
- [x] 批次 11：微交互下沉（buttons 全量 hover-lift + primary/danger 颜色 glow + press 压回；avatar-status-online 呼吸灰光；message-bubble @starting-style 入场过渡（incoming/outgoing）；sidebar-conversation hover 右侧流光 与 active 左侧流光互不冲突）
- [x] 批次 5：BackgroundSettings 数据结构（BackgroundMode / GradientKind / GradientStop / GradientSpec / BackgroundVariantData / BackgroundSettings）+ `UserSettings::background` 字段接入 + sanitised 级联 + to_css_vars + active_variant（主题感知）+ 17 条单元测试 + legacy 反序列化向后兼容
- [x] 批次 6：IndexedDB `background_image` store（DB_VERSION 4→5；`schema.rs` 新增 `STORE_BACKGROUND_IMAGE` + `KEY_USER_BG_LIGHT/DARK` + `BACKGROUND_IMAGE_MAX_BYTES` + `is_canonical_background_key` guard；新建 `store/background_image.rs` 提供 `put/get/delete/has` + `blob_to_object_url` + `revoke_object_url`；v5 migration 创建 out-of-line keyed blob store；6 条 native 常量/guard 测试 + 5 条 wasm_tests blob round-trip）
- [x] 批次 7：BackgroundSection UI 面板（`app_bg.rs` 改造为响应式组件，通过 Effect 同步 `--app-bg-*` CSS 变量 + `data-app-bg` 属性到 `<html>`，图片模式异步 IDB blob 加载 + object URL rotate 管理；新建 `components/settings_page/background_section.rs` Leptos 组件，含 Mode Radio / Solid color picker / Image upload（canvas 压缩到 ≤2560×1440 + WebP 0.85 re-encode + IDB 写入）/ Blur slider / Overlay slider / Theme-aware toggle / Reset 按钮；新建 `background_section_helpers.rs` + `tests.rs`（compute_resize_dims / validate_background_upload / slider↔value 往返 helper，19 条 native 单元测试）；新建 `styles/components/settings-background.css`；三语 i18n（en/zh-CN/es）各追加 18 个 key 覆盖 mode / upload / slider / theme-aware / reset / 错误提示）

---

## 11. 验收标准

### 11.1 功能

- ✅ 打开应用，top-bar/sidebar/drawer 呈 Mica 毛玻璃效果（暗/亮主题各一套基色）
- ✅ 设置 → 外观 下方能看到 2 个新开关；拨动后效果立即生效且刷新后保留
- ✅ 关闭"启用毛玻璃"→ 所有 glass 组件退化为实色面板，无抖动
- ✅ 关闭"启用酷炫动画"→ 所有 `.anim-*` 停止，过渡保留必要的 150ms 可用性过渡
- ✅ 背景默认显示 theme-aware 渐变（不是纯白/纯黑）
- ✅ sidebar 选中项有流光扫边动画（motion 开启时）

### 11.2 无障碍

- ✅ `prefers-reduced-transparency: reduce` → 毛玻璃强制关
- ✅ `prefers-reduced-motion: reduce` → 动画强制关
- ✅ 文字对比度仍满足 WCAG AA（背景渐变上 + 遮罩补偿）
- ✅ 键盘 focus ring 仍可见（glass 面板之上）

### 11.3 性能

- ✅ 非 theater 页 Lighthouse Performance ≥ 85（与改造前差距 ≤ 3 分）
- ✅ theater 播放页 fps 不下降（sample 30 秒，`frame_drop_monitor` 计数不增加）
- ✅ 小屏（≤ 640px）blur 自动降档，滚动无卡顿

### 11.4 代码质量

- ✅ `cargo check` 通过
- ✅ `cargo clippy -- -D warnings` 无警告
- ✅ `cargo test` 全通过
- ✅ 所有新增 CSS/Rust 注释为英文
- ✅ 新组件 `AppBg` 单文件单组件
- ✅ CSS 命名遵循 BEM（`.app-bg__image` 等）

---

## 12. 风险与缓解

| 风险 | 影响 | 缓解 |
|---|---|---|
| Safari `backdrop-filter` 性能差 | 大面积毛玻璃卡顿 | `-webkit-` 前缀 + 小屏降 blur + 总开关 |
| Mica grain 的 SVG noise 增加重绘 | GPU 占用 | 噪点只在 `::before` 伪元素，`pointer-events:none`，`mix-blend-mode:overlay` |
| 毛玻璃叠 app 渐变背景导致文字对比度不足 | 可读性 | 每个 glass 层用半透明 + inset highlight；背景层有 overlay 补偿 |
| 全局 `data-motion=off` 打到所有 `.anim-*` 选择器权重不够 | 开关失效 | 用 `html[data-motion="off"] *` 全局覆盖 + `!important` 保底（仅 animation/transition 属性） |
| theater 页帧率回归 | 直播体验下降 | theater 页主视频区 CSS 禁用 backdrop-filter；只在小面板启用 |
| 两个新 bool 改变 `UserSettings` 结构 → 旧 localStorage 反序列化失败 | 用户丢设置 | `serde` 的 `#[serde(default)]` 标注 + 已有 `sanitised()` 保护 |

---

## 13. 变更日志

| 日期 | 版本 | 说明 | 作者 |
|---|---|---|---|
| 2026-05-08 | v1 | 初稿，决定 MVP 范围（批次 1-4 + 8） | AI agent |
| 2026-05-08 | v1.1 | MVP 全部批次（1 / 2 / 3 / 4 / 8 / 8b / 8c）落地完成；cargo check & clippy 通过；cargo test server 711/711 通过；前端 lib test 受 Apple ld 环境 bug 阻挡同历史惯例跳过 | AI agent |
| 2026-05-08 | v1.2 | 批次 9 完成：modal / toast / error-toast / conversation-menu / sticker-panel 接入 L3 Mica；5 个组件 enter 动画统一升级为 `--ease-spring` + overshoot keyframe；cargo check 0.5s / clippy 0.25s / test server 711/711 全通过 | AI agent |
| 2026-05-08 | v1.3 | 批次 10 完成：subtitle-settings / danmaku-settings 接入 L4 Mica；copyright tooltip 改为暗色 L4 glass + spring 进入（保留跨主题一致性）；theater-page__stage 新增氛围 glow halo（fullscreen 自动隐藏、尊重 motion opt-out、严格不影响 `.theater-page__surface` 视频区）；cargo check 0.24s / clippy 0.24s / test server 711/711 全通过 | AI agent |
| 2026-05-08 | v1.4 | 批次 11 完成：.btn-base 全量接入 hover-lift + press-return，.btn-primary / .btn-danger 叠加颜色 glow（保留 `:not(:disabled)` 护栏）；avatar-status-online 加入 success-tinted breathe 关键帧；message-bubble-base 补充 opacity/transform transition，驱动 @starting-style 12px 入场（同时覆盖 incoming/outgoing 子类）；sidebar-conversation 新增 :hover/:focus-within::after 右侧流光加速可见且不与 active 左侧流光线冲突；cargo check 0.82s / clippy 0.26s / test server 711/711 全通过 | AI agent |
| 2026-05-08 | v1.5 | 批次 5 完成：`frontend/src/settings/types.rs` 新增 6 个类型（`BackgroundMode` / `GradientKind` / `GradientStop` / `GradientSpec` / `BackgroundVariantData` / `BackgroundSettings`）+ `BackgroundVariantView` 零拷贝视图；`UserSettings` 新增 `background` 字段（`#[serde(default)]`），sanitised 链级联清洗 blur、0..=40 / overlay、0.0..=0.8 / mode⇔payload 合法化 / theme-aware off 时清除 dark variant；提供 `to_css_vars(is_dark)` 与 `active_variant(is_dark)` helper 供批次 7 UI 消费；`GradientSpec::to_css` 支持 linear/radial；mod.rs re-export 所有新类型；19 条单元测试覆盖默认值 / clamp / mode回落 / theme-aware / CSS输出 / serde来回 / legacy 向后兼容；cargo check 13.33s / clippy 19.44s 无警告 / test server 711/711 + frontend lib 829/829 全通过（含 19 条新 background 测试） | AI agent |
| 2026-05-08 | v1.6 | 批次 6 完成：IndexedDB schema 升级（`DB_VERSION` 4→5），`schema.rs` 新增 `STORE_BACKGROUND_IMAGE` + `KEY_USER_BG_LIGHT/DARK` + `BACKGROUND_IMAGE_MAX_BYTES` + `is_canonical_background_key` guard（常量放在 schema 以便 native 可见，供批次 7 表单校验共用）；新建 `persistence/store/background_image.rs` 提供 `put/get/delete/has` async CRUD + `blob_to_object_url / revoke_object_url` 辅助（Blob 直接 out-of-line 存储，绕过 JSON round-trip 以保留二进制负载与 MIME type）；`v5 migration` 创建无 keyPath 的新 store；`store/mod.rs` re-export；native 侧 6 条常量/guard/store-name/DB版本 sanity 测试 + wasm_tests 5 条真实 blob put/get/has/overwrite/delete 全链路测试；cargo check 1.67s / clippy 7.45s 无警告 / test server 711/711 + frontend lib 835/835 全通过（含 6 条新 background_image 测试） | AI agent |
| 2026-05-08 | v1.7 | 批次 7 完成（UI 收官）：`app_bg.rs` 改造为响应式组件，分 native/wasm 双路实现 — Effect 订阅 `settings.background + theme + prefers-dark`，调用 `BackgroundSettings::to_css_vars(is_dark)` 写入 `<html>` CSS 变量 + `data-app-bg` 属性，Image 模式通过 `spawn_local` 异步拉取 IDB blob → `createObjectURL` → 写入 `--app-bg-image`，thread_local 持有当前 URL 并在 mode 切换/组件 unmount 时 `revokeObjectURL` 防止内存泄漏；新建 `components/settings_page/background_section.rs`（~450 行）含 Mode Radio（preset/solid/image 三选）/ Solid color picker / Image 上传（canvas `drawImage` 缩放 + `toBlob('image/webp', ...)` 再编码 + IDB 写入 + 错误 toast）/ Blur 0–40px slider / Overlay 0.0–0.8 slider / Theme-aware 开关 / Reset 按钮；新建 `background_section_helpers.rs` + `tests.rs`（compute_resize_dims 保持长宽比 + validate_background_upload 分类 Empty/TooLarge/UnsupportedType + blur/overlay slider ↔ value 往返 helper）19 条 native 单元测试覆盖边界、零尺寸、大小写/空格规范化、往返幂等；新建 `styles/components/settings-background.css`（颜色选择器、隐藏 file input 样式、跨浏览器 range slider thumb + hover glow、错误提示 pill、响应式小屏降级）；en/zh-CN/es 三语各追加 18 条 i18n 键；cargo check / clippy 零警告 / test server 711/711 + frontend lib 854/854 全通过（含 19 条新 helper 测试） | AI agent |

