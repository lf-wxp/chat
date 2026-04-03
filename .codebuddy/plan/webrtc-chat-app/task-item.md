# 实施计划 — WebRTC 多人聊天应用（Rust 全栈）

> 本实施计划基于 [requirements.md](./requirements.md) 需求文档生成。  
> 项目采用 Workspace 多 crate 架构：`message`（共享协议）、`server`（信令服务器 Axum）、`client`（Leptos 前端 WASM）。  

---

- [x] 1. 初始化项目结构与构建系统
  - 在项目根目录创建新的 Workspace，包含 `message`、`server`、`client` 三个 crate
  - 配置根 `Cargo.toml` workspace members 和共享依赖（bitcode、serde、tokio、axum、leptos 等）
  - `client` crate 使用 Leptos 0.8+（CSR 模式），配置 `Trunk.toml` 或 `cargo-leptos` 进行 WASM 构建
  - 创建 `Makefile.toml`，配置 `cargo-make` 任务：`dev-server`（启动信令服务器）、`dev-client`（Trunk serve / cargo-leptos watch 前端）、`build-release`（生产构建）、`lint`（clippy pedantic）、`test`（全量测试）
  - 配置 WASM 优化参数（`opt-level=z`、LTO）
  - 配置 `rustfmt.toml` 和 `.clippy.toml` 统一代码风格
  - _需求：技术栈约束、可维护性_

- [x] 2. 实现 `message` crate — 共享协议与二进制消息类型
  - 定义核心消息枚举 `Message`，包含所有消息变体：`Text`（UTF-8 内容 + Markdown 标记）、`Sticker`（包 ID + Sticker ID）、`Voice`（Opus 编码音频 `Vec<u8>` + 时长 ms）、`Image`（缩略图 `Vec<u8>` + 原图元信息 `ImageMeta { width, height, size, format }`）、`File`（文件名、大小、MIME 类型、分块元信息）
  - 定义信令协议枚举 `SignalMessage`：`SdpOffer`、`SdpAnswer`、`IceCandidate`、`JoinRoom`、`LeaveRoom`、`Heartbeat`/`HeartbeatAck`、`UserListUpdate`、`ConnectionInvite`/`InviteResponse`
  - 定义房间相关类型：`Room`（名称、描述、密码哈希、最大人数、房间类型 Chat/Theater）、`RoomMember`（用户 ID、角色 Owner/Member/Viewer、禁言状态）
  - 定义放映厅专用消息：`TheaterControl`（播放/暂停/跳转/切换视频源）、`Danmaku`（弹幕文字、颜色、位置类型、时间戳）、`TheaterSync`（播放进度同步）
  - 定义用户相关类型：`UserProfile`（用户名、头像、状态 Online/Offline/Busy/Away、签名）、`AuthToken`（JWT 载荷）
  - 为所有类型实现 `serde::Serialize` + `serde::Deserialize`，使用 bitcode 进行二进制编解码
  - 实现高效的二进制分块传输协议：动态分块大小（初始 64KB，根据 DataChannel `bufferedAmount` 自适应调节）、分块位图（bitmap）进度追踪、断点续传支持、流控机制（背压感知）
  - 实现 `Envelope` 包装类型，统一 DataChannel 的消息封装（消息 ID、时间戳、发送者、目标）
  - 编写单元测试：验证所有消息类型的序列化/反序列化往返正确性、分块协议的边界情况
  - _需求：9.1-9.6、2.5_

- [x] 3. 实现 `server` crate — 信令服务器核心
  - 使用 Axum 搭建 HTTP + WebSocket 服务器，支持通过环境变量配置端口、STUN/TURN 地址
  - 实现 WebSocket 连接管理器（`ConnectionManager`）：使用 `DashMap<UserId, WsSender>` 管理所有在线连接，支持按用户 ID 精确路由消息
  - 实现心跳检测机制：服务端定期发送 Ping，客户端回复 Pong，超时 30 秒未响应则断开连接并清理会话
  - 实现 SDP 信令转发：接收客户端的 SDP Offer，精确转发到目标 Peer，收集 SDP Answer 返回；支持 ICE Candidate 的精确转发
  - 实现多人 Mesh 拓扑协调：新用户加入房间时，协调其与房间内每个已有成员交换 SDP，建立 PeerConnection
  - 实现用户认证模块（纯内存存储，无持久化）：注册（用户名 + 密码 Argon2 哈希，存储在 `DashMap<UserId, UserSession>` 内存中，不写入数据库/文件）、登录（验证密码、生成 JWT Token）、WebSocket 连接时验证 Token；服务重启后所有用户数据清空，用户需重新注册/登录
  - 实现在线用户列表广播：用户上线/下线时向所有在线客户端广播 `UserListUpdate` 事件
  - 实现连接邀请转发：转发 `ConnectionInvite` 和 `InviteResponse` 消息，实现 60 秒超时自动过期、频率限制（Rate Limiting）
  - 实现离线邀请暂存：用户离线期间的连接邀请暂存在内存中（`DashMap<UserId, Vec<PendingInvite>>`），上线后推送（注意：聊天消息通过 DataChannel P2P 传输，服务器不中转）
  - 编写单元测试和集成测试：WebSocket 连接建立、信令转发、房间管理、认证流程
  - _需求：1.1-1.8、10.2、10.4、10.7、10.8、10.9、11.1-11.8、12.5_

- [x] 4. 实现 `server` crate — 房间与放映厅管理
  - 实现房间 CRUD：创建房间（名称、描述、密码、最大人数、类型 Chat/Theater）、加入房间（密码验证）、退出房间、销毁房间（所有成员退出时自动销毁）
  - 实现房间成员管理：成员列表维护、加入/退出广播通知、房主权限校验
  - 实现房主管理功能：踢出成员（断开 PeerConnection + 从房间移除 + 加入黑名单）、禁言/解除禁言（标记成员状态 + 广播状态变更）、全体禁言、转让房主
  - 实现放映厅专用逻辑：`TheaterControl` 消息仅房主可发送（权限校验）、`Danmaku` 消息检查禁言状态、`TheaterSync` 进度同步广播
  - 实现房间邀请：生成邀请链接、直接发送邀请通知给目标用户
  - 实现被踢用户黑名单：被踢出的用户尝试重新加入时拒绝
  - 编写单元测试：房间生命周期、权限校验、踢出/禁言逻辑、放映厅控制权限
  - _需求：4.1-4.9、13.1.1-13.1.7、13.3.1-13.3.6、13.6.1-13.6.7_

- [x] 5. 实现前端基础架构 — Leptos 状态管理、路由、主题与布局
  - 使用 Leptos Signals 实现全局响应式状态：`RwSignal<UserState>`（当前用户信息、认证 Token）、`RwSignal<ChatState>`（会话列表、当前会话、消息历史）、`RwSignal<OnlineUsersState>`（在线用户列表）、`RwSignal<RoomState>`（房间列表、当前房间）、`RwSignal<ThemeState>`（主题偏好）、`RwSignal<TheaterState>`（放映厅状态）；通过 `provide_context` / `use_context` 在组件树中共享状态
  - 使用 `leptos_router` 实现路由系统：`/login`（登录/注册）、`/`（主页/聊天列表）、`/chat/:id`（聊天界面）、`/room/:id`（房间）、`/theater/:id`（放映厅）、`/settings`（设置）
  - 实现 CSS 变量主题系统：定义 CSS 变量（`--bg-primary`、`--text-primary`、`--accent` 等），实现亮色/暗色两套变量值，通过 `prefers-color-scheme` 媒体查询自动切换，支持手动切换并持久化到 localStorage，主题切换时使用 CSS transition 平滑过渡
  - 实现响应式布局框架：桌面端（≥1024px）双栏布局（侧边栏 + 主内容区）、平板端（768px-1024px）可折叠侧边栏、移动端（<768px）单栏 + 抽屉式导航
  - 实现通用 Leptos 组件库：`<Button/>`、`<Input/>`、`<Avatar/>`、`<Badge/>`、`<Modal/>`、`<Dropdown/>`、`<Toast/>`、`<Tooltip/>`、`<VirtualList/>`（虚拟滚动）
  - 实现 I18n 国际化（使用 leptos-i18n 或自实现方案，支持中文/英文双语）
  - _需求：7.1-7.8、可维护性、国际化_

- [x] 6. 实现前端网络层 — WebSocket 客户端与 WebRTC 管理
  - 实现 WebSocket 客户端模块（仅用于信令）：连接建立（携带 JWT Token）、bitcode 二进制信令消息收发、心跳响应（Pong）、断线自动重连（指数退避策略：1s → 2s → 4s → 8s → 16s → 30s 上限）、重连后恢复会话状态；使用 Leptos `create_effect` 监听连接状态变化并更新 UI
  - 实现 WebRTC PeerConnection 管理器（`PeerManager`）：管理多个 PeerConnection 实例（`HashMap<PeerId, RtcPeerConnection>`）、SDP Offer/Answer 创建与设置、ICE Candidate 收集与添加、MediaStream 轨道管理（音频/视频添加/移除）
  - 实现 DataChannel 管理（所有聊天消息和文件传输的唯一通道）：创建可靠有序的 DataChannel 用于聊天消息和文件传输、创建不可靠无序的 DataChannel 用于弹幕等实时数据、二进制数据收发（ArrayBuffer）、消息路由（根据 Envelope 目标分发）
  - 实现 P2P 消息发送逻辑：所有聊天消息（文本/Sticker/语音/图片）和文件均通过 DataChannel 发送，WebSocket 不承载任何聊天数据
  - 实现 ICE 配置获取：连接时从服务器获取 STUN/TURN 配置
  - 实现连接状态监控：PeerConnection 状态变化回调、ICE 连接状态监控、网络质量检测（RTCStatsReport）
  - 编写集成测试：WebSocket 连接/重连、PeerConnection 建立、DataChannel 数据传输
  - _需求：1.1-1.8、2.2、9.1-9.4_

- [x] 7. 实现前端聊天功能 — 消息收发、多类型消息与交互
  - 实现登录/注册页面（Leptos 组件）：用户名 + 密码表单、JWT Token 存储到 localStorage、自动登录恢复（检测服务端是否重启，若 Token 无效则引导重新注册/登录）
  - 实现在线用户面板组件：实时在线用户列表（`<VirtualList/>` 虚拟列表渲染）、用户信息卡片弹窗、搜索/过滤（使用 `create_memo` 派生过滤后的用户列表）、「发送连接邀请」按钮（含防重复、超时状态管理）、多选批量邀请
  - 实现邀请通知组件：弹窗展示邀请信息（发起者头像、用户名、附言）、接受/拒绝按钮、60 秒超时自动关闭
  - 实现聊天界面主体：消息列表（`<VirtualList/>` 虚拟滚动）、消息气泡组件（区分自己/他人、显示时间戳和状态）、消息输入栏（文本输入 + 工具栏按钮）
  - 实现文本消息：Markdown 基础渲染（加粗/斜体/代码块/链接）、URL 自动识别生成可点击链接
  - 实现 Sticker 消息：Sticker 选择面板组件（网格布局、分类切换、搜索）、内置默认 Sticker 包（WebP/SVG 资源）、Sticker 以较大尺寸展示
  - 实现语音消息：录音按钮组件（长按录制、上滑取消、松开发送）、录制中实时波形动画 + 时长显示、Opus 编码压缩、语音气泡组件（波形图 + 时长 + 播放按钮）
  - 实现图片消息：文件选择器 + 剪贴板粘贴识别、发送前缩略图预览确认弹窗、聊天中缩略图展示、点击查看原图（缩放 + 左右滑动浏览）、支持 JPEG/PNG/WebP/GIF
  - 实现消息交互功能：消息上下文菜单（回复、引用、撤回、复制）、「正在输入...」状态提示、@ 提及功能（高亮 + 特殊通知）、消息状态显示（发送中/已发送/失败 + 重试）、未读消息计数
  - 实现 IndexedDB 消息持久化：消息存储/读取、历史消息加载、存储空间管理
  - 实现浏览器 Notification API 推送通知
  - _需求：2.1-2.12、10.1-10.12、11.1-11.8、12.1-12.4、8.6-8.8_

- [x] 8. 实现前端音视频通话功能
  - 实现通话发起与接听：通话邀请弹窗（来电者信息 + 接听/拒绝按钮）、通话建立流程（请求媒体权限 → 创建 PeerConnection → 交换 SDP → 添加 MediaStream）
  - 实现多人视频通话 UI（Leptos 组件）：网格布局（Grid Layout）自动根据参与人数调整（1人全屏、2人左右分屏、3-4人 2×2、5-9人 3×3）、本地视频预览小窗；使用 `create_memo` 根据参与者数量动态计算网格布局
  - 实现音视频模式切换：视频→语音（关闭摄像头轨道，保留音频，无需重连）、语音→视频（请求摄像头权限，添加视频轨道，无需重连）
  - 实现通话控制栏：静音/取消静音、开启/关闭摄像头、屏幕共享、挂断按钮、通话时长计时器
  - 实现 VAD（Voice Activity Detection）：检测音频活动，高亮当前说话者
  - 实现网络质量自适应：监控 RTCStatsReport，网络差时自动降低分辨率/帧率，UI 显示网络质量指示器（绿/黄/红）
  - 实现 Picture-in-Picture 浮动小窗模式
  - 实现移动端全屏视频模式
  - 实现通话结束统计（通话时长显示）
  - _需求：3.1-3.9、8.1-8.5、7.8_

- [x] 9. 实现前端文件传输与端到端加密
  - 实现文件选择与拖拽上传组件（Leptos 组件）：文件选择器 + 拖拽区域、文件大小校验（≤100MB）、文件类型图标映射
  - 实现文件传输引擎：基于 DataChannel 的 P2P 分块传输（动态分块大小，初始 64KB，根据 `bufferedAmount` 自适应调节）、流控机制（背压感知，防止缓冲区溢出）、传输进度条 + 速度 + 预计剩余时间 UI、断点续传（基于分块位图 bitmap 追踪已传输分块）
  - 实现文件消息卡片组件：文件名、大小、类型图标、下载按钮、传输进度条
  - 实现图片文件自动缩略图生成（Canvas 缩放）
  - 实现 ECDH 密钥交换：PeerConnection 建立后通过 DataChannel 交换 ECDH 公钥，协商共享密钥
  - 实现 AES-256-GCM 端到端加密：文本消息加密/解密、文件分块加密/解密、加密状态图标显示
  - 实现密钥协商失败处理：通知用户 + 重试选项
  - 编写单元测试：分块传输、加密/解密往返、断点续传
  - _需求：5.1-5.6、6.1-6.8_

- [x] 10. 实现前端共享放映厅功能
  - 实现放映厅列表页面（Leptos 组件）：显示所有放映厅（名称、房主、观众数、播放状态、是否加密）、创建放映厅表单（名称、描述、密码、最大观众数）
  - 实现房主端视频播放器：本地视频文件选择（MP4/WebM/MKV）+ `<video>` 元素加载、在线视频 URL 输入 + CORS 错误处理提示、`captureStream()` 捕获 MediaStream 并通过 PeerConnection 分发给所有观众
  - 实现观众端视频接收：通过 PeerConnection 接收房主分发的 MediaStream、`<video>` 元素渲染远端视频流、新观众加入时自动建立连接并从当前位置开始观看
  - 实现播放控制栏（仅房主可操作）：播放/暂停、进度条拖动、音量控制、全屏切换、当前时间/总时长显示、视频源切换（替换 PeerConnection 中的轨道）
  - 实现观众端只读进度条：每 5 秒通过 DataChannel 接收进度同步信息，进度条只读展示
  - 实现弹幕系统：弹幕输入框 + 颜色选择（预设调色板）+ 位置选择（顶部/底部/滚动）、Canvas 弹幕渲染引擎（`requestAnimationFrame` 驱动、从右向左滚动动画）、弹幕密度控制（>50 条时自动降低密度）、弹幕设置面板（透明度/字体大小/滚动速度）、弹幕开关按钮
  - 实现放映厅消息面板：桌面端右侧 / 移动端下方、聊天气泡消息列表、未读消息计数徽章
  - 实现房主管理面板：观众列表（用户名、在线状态、禁言标记）、踢出/禁言/解除禁言/全体禁言/转让房主操作按钮、被踢用户禁止重新加入
  - 实现放映厅响应式布局：桌面端（左视频 + 右面板）、移动端（上视频 + 下面板切换）、全屏模式（弹幕叠加 + 手势唤出面板）
  - _需求：13.1-13.7_

- [x] 11. 安全加固、性能优化与最终集成测试
  - 实现 XSS 防护：所有用户输入（消息、弹幕、房间名称等）进行 HTML 转义和内容过滤
  - 实现敏感词检测：弹幕和消息内容的基础敏感词过滤
  - 确保 WSS（WebSocket Secure）配置：服务端 TLS 支持
  - 实现连接邀请频率限制：服务端 Rate Limiting（每用户每分钟最多 10 次邀请）
  - 实现放映厅权限校验加固：所有管控操作（踢出/禁言/播放控制）在服务端二次校验权限
  - 性能优化：消息列表虚拟滚动、在线用户列表虚拟列表（>100 人）、弹幕 Canvas 渲染优化、WASM 包体积优化（`opt-level=z` + LTO）
  - 全量 Clippy pedantic 检查并修复所有警告
  - 编写端到端集成测试：完整用户流程（注册→登录→发现用户→邀请→聊天→通话→文件传输→放映厅）
  - 编写 README.md：项目介绍、架构说明、开发环境搭建、构建与运行指南
  - _需求：安全、性能、可维护性_
