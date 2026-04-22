# 实施计划 — WebRTC Multi-User Chat Application (Rust Full-Stack)

> **分层策略**: 公共库 → 后端服务 → 前端 → 整合联调
> 每一层完成所有类型的测试后就是停止当前进度， 等待用户确认后。 再进入下一层，确保每层的质量可控。

### ⛔ 全局任务完成门禁（适用于每一个任务项）

**每个任务完成前，必须依次通过以下五项检查，全部零错误/零警告后方可进入下一个任务：**

1. `cargo fmt --check` — 代码格式检查（基于项目根目录 `rustfmt.toml`：2-space indent, 100 char width, edition 2024）
2. `cargo check` — 编译检查零错误
3. `cargo clippy -- -D warnings` — Clippy 零警告（所有警告视为错误）
4. `cargo test` — 所有测试通过
5. **代码注释语言检查** — 所有代码注释、docstrings、错误提示信息必须使用纯英文，禁止中文字符（硬性要求，无例外）

> **⚠️ Rust 编译耗时注意事项**
>
> 由于 Rust 编译时间较长，在执行 shell 命令（如 `cargo check`、`cargo clippy`、`cargo test`）时，
> 可能因超时而无法直接获取到 shell 输出。遇到此情况时，应采用**后台执行 + 标记文件 + 轮询等待**的方式。
>
> ### 执行策略：后台运行 + 完成标记文件
>
> ```bash
> # Step 1: 后台执行命令，将输出重定向到临时文件，完成后写入退出码到标记文件
> (cargo check 2>&1 | tee /tmp/cargo-check-output.txt; echo $? > /tmp/cargo-check-done) &
>
> # Step 2: 轮询等待标记文件出现（表示命令已执行完毕）
> # 每隔几秒检查一次 /tmp/cargo-check-done 是否存在, 特别注意再上次检查没有结束前， 不要再检查
> while [ ! -f /tmp/cargo-check-done ]; do sleep 3; done
>
> # Step 3: 读取退出码，判断是否成功
> cat /tmp/cargo-check-done    # 0 表示成功，非 0 表示失败
>
> # Step 4: 读取完整输出
> cat /tmp/cargo-check-output.txt
> ```
> **当上一个轮询命令没有结束时， 不能进行下一次的轮询**
> 
> ### 三项门禁的完整执行示例
>
> **必须严格串行执行**，因为 Cargo 使用文件锁（`cargo.lock`），多个 cargo 命令不能并行运行，否则会因锁冲突而失败。
>
> ```bash
> # ---- 1. cargo check ----
> rm -f /tmp/cargo-check-done /tmp/cargo-check-output.txt
> (cargo check 2>&1 | tee /tmp/cargo-check-output.txt; echo $? > /tmp/cargo-check-done) &
> # 轮询等待完成
> while [ ! -f /tmp/cargo-check-done ]; do sleep 3; done
> # 检查结果：读取 /tmp/cargo-check-done（应为 0）和 /tmp/cargo-check-output.txt
>
> # ---- 2. cargo clippy（仅在 check 通过后执行）----
> rm -f /tmp/cargo-clippy-done /tmp/cargo-clippy-output.txt
> (cargo clippy -- -D warnings 2>&1 | tee /tmp/cargo-clippy-output.txt; echo $? > /tmp/cargo-clippy-done) &
> while [ ! -f /tmp/cargo-clippy-done ]; do sleep 3; done
> # 检查结果
>
> # ---- 3. cargo test（仅在 clippy 通过后执行）----
> rm -f /tmp/cargo-test-done /tmp/cargo-test-output.txt
> (cargo test 2>&1 | tee /tmp/cargo-test-output.txt; echo $? > /tmp/cargo-test-done) &
> while [ ! -f /tmp/cargo-test-done ]; do sleep 3; done
> # 检查结果
> ```
>
> ### ⚠️ Cargo 文件锁注意事项
>
> - Cargo 在编译时会获取 `target/` 目录下的文件锁，**同一时间只能运行一个 cargo 命令**
> - 如果前一个 cargo 命令尚未结束就启动下一个，后者会阻塞等待锁释放，或直接报错
> - 因此三项门禁检查**必须严格按顺序串行执行**：`cargo check` → `cargo clippy` → `cargo test`
> - 每一步必须确认**标记文件已生成**（即命令已完成）后，才能启动下一步
> - 执行前务必 `rm -f` 清理上一轮的临时文件和标记文件，避免误读旧结果

## 文件的大小需要注意

- 文件要有合理的划分， 不要让文件过大, 要符合rust的最佳实践
- 如果文件中测试数据过大，统一采用 #[cfg(test)] mod tests; 拆分策略

## 前端组件的书写注意
- 每个文件中只能有一个组件，多个组件不能在一个文件中
- html的代码块中标签的缩进使用2个空格

---

## Phase 1: 公共库 (message crate + shared types)

> 公共库是前后端共享的基础，必须最先完成并充分测试。所有消息类型、协议定义、序列化/反序列化逻辑都在此层实现。

- [x] 1. 初始化 Workspace 项目结构与 cargo-make 配置
  - 创建 Rust Workspace，包含 `message`、`server`、`frontend` 三个 crate, 就在当前目录创建，不用嵌套目录创建, 不是crates/backend, crates/message, crates/frontend,就是 backend， message， frontend 三个目录
  - 编写 `Makefile.toml`，定义 `dev`、`build`、`test`、`test-unit`、`test-integration`、`test-wasm`、`test-e2e`、`lint`、`fmt`、`clean`、`docker` 等任务及依赖关系
  - 配置 `message` crate 支持双目标编译（native + `wasm32-unknown-unknown`），使用条件编译 `#[cfg(target_arch = "wasm32")]`
  - **所有 crate 依赖必须使用当前最新稳定版本**（硬性要求，无例外）：每个依赖须查阅 [crates.io](https://crates.io) 确认最新稳定版，版本号使用 caret 语法（如 `tokio = "1.44"`）
  - 验证项目根目录已有 `rustfmt.toml` 配置（2-space indent, 100 char width, edition 2024），确保 `cargo fmt --check` 通过
  - 配置 Clippy pedantic 规则，确保零警告
  - _需求：requirements.md 非功能需求 (Build & Task Management, Crate Dependency Version Policy, Code Formatting)、Req 8 (WASM Compatibility Requirements)_

- [x] 2. 实现核心数据类型与枚举定义
  - 定义所有基础类型：`UserId`、`RoomId`、`MessageId (Uuid)`、`TransferId`
  - 定义所有枚举：`UserStatus`、`RoomType`、`MediaType`、`DanmakuPosition`、`MessageContentType`、`ReactionAction`、`MuteInfo`、`RoomRole (Owner/Admin/Member)`
  - 定义所有结构体：`UserInfo`、`RoomInfo`、`MemberInfo`、`ImageMeta`、`SubtitleEntry`、`NetworkQuality`
  - 为所有类型实现 `bitcode::Encode` + `bitcode::Decode` derive
  - 编写单元测试：每个类型的序列化/反序列化 roundtrip 测试
  - _需求：Req 8.5、Req 8.6、Req 4.1 (RoomType)、Req 15.3 (RoomRole)、Req 3.8b (NetworkQuality)_

- [x] 3. 实现信令消息类型（WebSocket 消息）
  - 实现所有信令消息枚举及其 payload 结构体：
    - 连接认证：`TokenAuth`、`AuthSuccess`、`AuthFailure`、`ErrorResponse`、`UserLogout`、`Ping`、`Pong`
    - 用户发现：`UserListUpdate`、`UserStatusChange`
    - 连接邀请：`ConnectionInvite`、`InviteAccepted`、`InviteDeclined`、`InviteTimeout`、`MultiInvite`
    - SDP/ICE：`SdpOffer`、`SdpAnswer`、`IceCandidate`
    - Peer 追踪：`PeerEstablished`、`PeerClosed`、`ActivePeersList`
    - 房间管理：`CreateRoom`、`JoinRoom`、`LeaveRoom`、`RoomListUpdate`、`RoomMemberUpdate`、`KickMember`、`TransferOwnership`
    - 通话信令：`CallInvite`、`CallAccept`、`CallDecline`、`CallEnd`
    - 剧场信令：`TheaterMuteAll`、`TheaterTransferOwner`
    - 房间管理与资料：`MuteMember`、`UnmuteMember`、`BanMember`、`UnbanMember`、`PromoteAdmin`、`DemoteAdmin`、`NicknameChange`、`RoomAnnouncement`、`ModerationNotification`
  - 实现 Message Type Discriminator 映射（0x00-0x7D）
  - 编写单元测试：所有信令消息的 encode/decode roundtrip，验证 discriminator 值正确
  - _需求：Req 8 (Signaling Message Type Catalog)、Req 8 (Message Type Mapping)_

- [x] 4. 实现 DataChannel 消息类型（P2P 消息）
  - 实现所有 DataChannel 消息枚举及其 payload 结构体：
    - 聊天消息：`ChatText`、`ChatSticker`、`ChatVoice`、`ChatImage`
    - 文件传输：`FileChunk`、`FileMetadata`
    - 消息控制：`MessageAck`、`MessageRevoke`、`TypingIndicator`、`MessageRead`
    - 消息增强：`ForwardMessage`、`MessageReaction`
    - 加密：`EcdhKeyExchange`
    - 头像：`AvatarRequest`、`AvatarData`
    - 剧场：`Danmaku`、`PlaybackProgress`、`SubtitleData`、`SubtitleClear`
  - 实现 DataChannel Message Type Discriminator 映射（0x80-0xC3）
  - 编写单元测试：所有 DataChannel 消息的 encode/decode roundtrip
  - _需求：Req 8 (DataChannel Message Types)、Req 2.13 (ForwardMessage)、Req 2.14 (MessageReaction)、Req 12.4a (SubtitleData/SubtitleClear)_

- [x] 5. 实现二进制协议帧结构与大消息分片
  - 实现 Message Frame 结构：Magic Number (0xBCBC) + Message Type (1 byte) + Payload
  - 实现 `encode_frame()` 和 `decode_frame()` 函数，包含 Magic Number 校验
  - 实现大消息分片协议（>64KB 自动分片）：分片头（message_id + total_size + chunk_index + chunk_data）
  - 实现分片重组逻辑：基于 message_id 的重组缓冲区、chunk bitmap 追踪、30 秒超时清理、最大 10 个并发重组缓冲区
  - 实现文件传输 Chunk Bitmap 格式
  - 编写单元测试：帧编解码、大消息分片/重组、bitmap 操作、超时清理、边界条件
  - _需求：Req 8 (Binary Protocol Specification)、Req 8 (Large Message Chunking Protocol)、Req 8 (File Transfer Protocol)_

- [x] 6. 实现统一错误码系统与 i18n 键映射
  - 定义 `ErrorCode` 枚举，包含所有错误码（SIG001-SYS301、ROM104-ROM108、CHT103-CHT105、THR103-THR104）
  - 实现 `ErrorResponse` 结构体（code、message、i18n_key、details、timestamp、trace_id）
  - 实现错误码到 i18n 键的映射函数
  - 实现输入验证工具函数：用户名验证（字母数字下划线，≤20字符）、昵称验证（中英文+数字+下划线+空格，≤20字符）、房间名验证（≤100字符）、公告验证（≤500字符）、弹幕验证（≤100字符）、消息长度验证（≤10000字符）
  - 编写单元测试：所有错误码映射、所有验证函数的正向/反向测试
  - _需求：requirements.md (Error Code Specification)、requirements.md (Security - XSS protection)、Req 15.1 (Nickname validation)_

- [x] 7. 实现 WASM 兼容层与 wasm-bindgen 接口
  - 为 `message` crate 添加 `wasm-bindgen` feature gate
  - 实现 `#[wasm_bindgen]` 导出的 `encode_message()` 和 `decode_message()` 函数
  - 实现 ArrayBuffer ↔ `Vec<u8>` 的零拷贝转换
  - 实现 JsValue 错误转换（Rust Error → JavaScript-friendly error message）
  - 编写 `wasm-bindgen-test` 测试：所有消息类型的 WASM 环境 encode/decode roundtrip
  - 验证 `bitcode` crate 的 WASM 兼容性
  - _需求：Req 8 (WASM Binary Parsing Implementation)、Req 8 (WASM Interface Requirements)_

- [x] **Phase 1 测试门禁**
  - 运行 `makers test-unit`：message crate 单元测试覆盖率 ≥ 90%
  - 运行 `makers test-wasm`：所有 WASM 测试通过（wasm-pack test --headless --chrome）
  - 运行 `makers lint`：Clippy pedantic 零警告 + cargo fmt 检查通过
  - 验证 message crate 在 native 和 wasm32 双目标下均可编译

---

## Phase 2: 后端服务 (Axum 信令服务器)

> 后端信令服务器负责 WebSocket 连接管理、信令转发、房间管理、用户认证等。不处理聊天消息（聊天走 DataChannel P2P）。

- [x] 8. 实现 Axum 服务器基础框架与 WebSocket 连接管理
  - 搭建 Axum HTTP/WebSocket 服务器，支持环境变量配置（PORT、RUST_LOG、JWT_SECRET、STUN/TURN_SERVERS、TLS 路径、RUST_LOG_FORMAT、LOG_OUTPUT、LOG_ROTATION、LOG_DIR、LOG_MAX_FILES、LOG_MAX_SIZE_MB）
  - 实现 WebSocket 连接处理：二进制模式、bitcode 消息解码/编码（复用 message crate）
  - 实现心跳检测机制（Ping/Pong），超时断开
  - 实现连接断开自动清理（清理信令会话、通知相关 Peer）
  - 实现静态文件服务（前端 dist/ 目录 + Sticker 资源 /assets/stickers/）
  - 实现生产级日志系统（`tracing` + `tracing-subscriber` + `tracing-appender`）：
    - 结构化日志输出：JSON 格式（`RUST_LOG_FORMAT=json`，生产环境）和 pretty 格式（`RUST_LOG_FORMAT=pretty`，开发环境，默认）
    - 日志同时输出到 stdout 和文件（`LOG_OUTPUT`：`stdout`/`file`/`both`，默认 `both`）
    - 日志轮转策略（`tracing-appender::rolling`）：daily（默认）/hourly/never，文件命名 `server.log.YYYY-MM-DD`
    - 日志文件保留与清理：最大文件数（`LOG_MAX_FILES`，默认 30）、最大目录大小（`LOG_MAX_SIZE_MB`，默认 500MB），超限自动清理最旧文件
    - 日志目录配置（`LOG_DIR`，默认 `./logs/`）
    - 日志级别策略：error/warn/info/debug/trace 五级，支持 per-module 级别覆盖（`RUST_LOG=info,backend::ws=debug`）
    - 异步日志写入（`tracing-appender::non_blocking`），持有 `WorkerGuard` 直到关闭
    - 优雅关闭时日志刷新：SIGTERM/SIGINT 信号处理，确保所有日志写入磁盘
  - 实现日志脱敏：JWT token 脱敏（仅显示前 8 后 4 字符）、IP 掩码、密码不入日志、消息仅记录摘要
  - 编写单元测试：WebSocket 连接生命周期、心跳超时、消息编解码、日志轮转配置解析
  - _需求：Req 1.7 (Heartbeat)、Req 1.5 (Connection cleanup)、Req 8.1 (bitcode signaling)、requirements.md (Observability - Backend Logging System)、requirements.md (Security - log desensitization)_

- [x] 9. 实现用户认证与会话管理
  - 实现用户注册/登录：内存存储 `DashMap<UserId, UserSession>`、Argon2 密码哈希
  - 实现 JWT Token 生成与验证（AES 加密用户数据）
  - 实现 `TokenAuth` 信令处理：页面刷新后的无状态认证恢复
  - 实现 `UserLogout` 信令处理：完整登出流程（清理 active_peers、房间成员、广播离线事件）
  - 实现单设备登录策略：`SessionInvalidated` 消息踢出旧设备
  - 实现用户状态管理：在线/离线/忙碌/离开状态广播
  - 编写单元测试：注册/登录流程、JWT 生成/验证、TokenAuth 恢复、单设备策略
  - 编写集成测试：完整的 WebSocket 认证生命周期
  - _需求：Req 10.1-10.4 (Authentication)、Req 10.7 (Multi-device policy)、Req 10.10 (Server restart)_

- [x] 10. 实现用户发现与连接邀请系统
  - 实现在线用户列表管理：用户上下线广播 `UserListUpdate`、`UserStatusChange`
  - 实现连接邀请流程：`ConnectionInvite` → `InviteAccepted`/`InviteDeclined`/`InviteTimeout`
  - 实现邀请频率限制（10次/分钟、50次/小时、5个未应答上限）
  - 实现双向邀请冲突检测与自动合并（Req 9.13）
  - 实现多人邀请 `MultiInvite`：至少一人接受即创建房间，后续接受者自动加入
  - 实现 60 秒邀请超时自动处理（get_timed_out_invitations 方法已实现，后台任务待集成）
  - 编写单元测试：邀请流程、频率限制、双向冲突合并、多人邀请（11个测试全部通过）
  - 编写集成测试：多用户邀请场景
  - _需求：Req 9.1-9.14 (Discovery & Invitation)_

- [x] 11. 实现房间系统与权限管理
  - 实现房间 CRUD：`CreateRoom`、`JoinRoom`、`LeaveRoom`，支持 Chat/Theater 两种类型
  - 实现房间密码保护：加入时密码验证、Owner 修改/清除密码
  - 实现房间成员管理：`RoomMemberUpdate` 广播、成员上限 8 人检查
  - 实现统一权限系统（Owner > Admin > Member）：
    - `KickMember`、`MuteMember`/`UnmuteMember`、`BanMember`/`UnbanMember`
    - `PromoteAdmin`/`DemoteAdmin`、`TransferOwnership`
    - 权限中间件：验证操作者角色 > 目标角色
  - 实现禁言到期自动解除（定时器）
  - 实现 Owner 离开时自动转移所有权（最久 Admin → 最久 Member）
  - 实现空房间自动销毁
  - 实现房间公告管理：`RoomAnnouncement` 广播、`ModerationNotification` 通知
  - 实现昵称管理：`NicknameChange` 广播
  - 编写单元测试：房间 CRUD、密码验证、权限检查、禁言/封禁、所有权转移（16个测试全部通过）
  - 编写集成测试：多用户房间加入/离开/管理场景
  - _需求：Req 4 (Room System)、Req 15 (Profile & Permissions)_

- [x] 12. 实现 SDP/ICE 信令转发与 Peer 追踪
  - 实现 SDP Offer/Answer 精确转发：`SdpOffer`、`SdpAnswer`
  - 实现 ICE Candidate 精确转发：`IceCandidate`
  - 实现 Peer 关系追踪：`PeerEstablished`/`PeerClosed` → 维护 `active_peers: HashSet<UserId>`
  - 实现刷新恢复：`ActivePeersList` 推送（TokenAuth 成功后）
  - 实现通话信令转发：`CallInvite`、`CallAccept`、`CallDecline`、`CallEnd`
  - 实现剧场信令转发：`TheaterMuteAll`、`TheaterTransferOwner`
  - 实现 SDP 并发排队（Req 9.14）：当前 SDP 协商进行中时排队新邀请
  - 编写单元测试：信令转发路由、Peer 追踪增删、ActivePeersList 生成（77个测试全部通过）
  - 编写集成测试：多用户 SDP/ICE 交换、Peer 追踪一致性
  - _需求：Req 1 (Signaling)、Req 10.3 (Connection Recovery)_

- [x] **Phase 2 测试门禁**
  - 运行 `makers test-unit`：server crate 单元测试覆盖率 ≥ 80%
  - 运行 `makers test-integration`：所有集成测试通过（WebSocket 生命周期、多用户房间、SDP/ICE 转发、TokenAuth 恢复）
  - 运行 `makers lint`：Clippy pedantic 零警告
  - 手动验证：服务器启动、WebSocket 连接、静态文件服务

---

## Phase 3: 前端 (Leptos WASM 客户端)

> 前端基于 Leptos 0.8+ CSR + WASM，实现所有用户交互、WebRTC 连接、DataChannel 通信、UI 渲染。

- [x] 13. 搭建前端基础框架与全局状态管理
  - 配置 Leptos 0.8+ CSR 项目（Trunk 构建）、WASM 优化（opt-level=z + LTO）
  - 实现全局状态管理（Leptos Signals）：
    - `auth_state: RwSignal<Option<AuthState>>`（用户认证状态）
    - `online_users: RwSignal<Vec<UserInfo>>`（在线用户列表）
    - `rooms: RwSignal<Vec<RoomInfo>>`（房间列表）
    - `conversations: RwSignal<Vec<Conversation>>`（会话列表，含置顶/免打扰状态）
    - `active_conversation: RwSignal<Option<ConversationId>>`（当前活跃会话）
    - `network_quality: RwSignal<HashMap<UserId, NetworkQuality>>`（网络质量指标）
  - 实现前端日志系统（Client-Side Debug Logs）：
    - 定义日志级别：error/warn/info/debug/trace
    - 非 debug 模式仅输出 error/warn 到 Console；debug 模式（`?debug=true` 或 `localStorage.debug_mode`）输出全部级别
    - 支持 per-module 日志过滤（`localStorage.debug_filter`，如 `"webrtc,signaling"`）
    - 实现内存环形缓冲区（Ring Buffer）：保留最近 1000 条日志（`localStorage.debug_buffer_size` 可配置），每条含 timestamp/level/module/message/data
    - 实现 Debug 面板（`Ctrl/Cmd + Shift + D` 快捷键或 Settings → Data Management → Debug Logs 入口）：可滚动/可过滤的日志查看器，支持按级别/模块过滤、文本搜索、导出 JSON、清空缓冲区
    - 实现诊断报告生成（Settings → Data Management → "Generate Diagnostic Report"）：浏览器信息、连接状态、性能指标、最近 50 条 error 日志、当前配置，不含敏感数据，下载为 `diagnostic-{timestamp}.json`
  - 实现原生 CSS 样式架构（不使用任何第三方 CSS 框架）：
    - 创建 CSS 文件组织结构：`/styles/tokens.css`（设计令牌）、`/styles/reset.css`（CSS Reset）、`/styles/base.css`（基础元素样式）、`/styles/components/*.css`（组件样式）、`/styles/utilities.css`（工具类）、`/styles/main.css`（入口，通过 `@layer` 和 `@import` 组织）
    - 使用 `@layer reset, tokens, base, components, utilities` 组织级联层
    - 使用 CSS Custom Properties 定义所有设计令牌（颜色、语义化反馈色彩、间距、排版、阴影、圆角、Z-Index 层级、动画与动效令牌），在 `:root` / `[data-theme]` 选择器上定义
    - 使用 CSS Nesting（`&` 选择器）编写组件作用域样式
    - 使用 `@container` queries 实现组件级响应式设计
    - 使用 `color-mix(in oklch, ...)` 实现 hover/pressed 状态颜色派生
    - 使用 `:has()` 选择器实现父级条件样式
    - 使用 `@scope` 实现组件级样式封装
    - 使用 CSS Subgrid 对齐嵌套网格项
    - 使用 `@starting-style` 实现动态插入元素的入场动画
    - 使用 CSS Anchor Positioning 实现 tooltip/popover/context menu 定位
    - 使用 View Transitions API 实现页面/视图切换动画
    - 使用 Scroll-driven Animations 实现滚动驱动效果
  - 实现主题系统（Light/Dark/System）：基于 CSS Custom Properties + `[data-theme]` 属性切换，200ms 过渡动画
  - 实现 `leptos-i18n` 国际化框架：加载 `/locales/{locale}.json`、语言切换、浏览器语言检测
  - 实现响应式布局框架：Desktop(≥1024px)/Tablet(768-1023px)/Mobile(<768px) 三档断点，使用 CSS Grid + Flexbox + `@container` + `@media` queries，`clamp()` 流式排版
  - 编写单元测试：Signal 状态管理、主题切换、i18n 语言切换、日志 Ring Buffer 写入/溢出/过滤
  - _需求：Req 14 (UI Interaction Design)、Req 14 Technical Implementation Notes (Native CSS Architecture)、requirements.md (Internationalization)、requirements.md (Performance - WASM bundle)、requirements.md (Observability - Frontend Logging System)_

- [x] 14. 实现 WebSocket 信令客户端与认证系统
  - 实现 WebSocket 连接管理：二进制模式、bitcode 消息编解码（调用 message crate WASM 接口）
  - 实现指数退避自动重连策略（含随机抖动防止惊群效应）
  - 实现用户注册/登录 UI 与流程
  - 实现 JWT Token 持久化（localStorage）与 TokenAuth 自动恢复
  - 实现用户状态管理：在线/离线/忙碌/离开、5 分钟无操作自动切换 Away
  - 实现 `SessionInvalidated` 处理：显示提示 → 清理状态 → 跳转登录页
  - 实现全局错误处理：`ErrorResponse` 解析 → i18n 错误消息展示 → "了解更多" 展开
  - 实现 Identicon 默认头像生成（基于用户名的 SVG/Canvas 算法）
  - 编写单元测试：重连策略（6个）、Identicon（7个）、Token 持久化/恢复、错误码解析（已在 message crate 中覆盖）
  - _需求：Req 10.1-10.2 (Auth)、Req 10.6 (Avatar)、Req 10.9 (State Persistence)、requirements.md (Error Handling)_

- [ ] 15. 实现 WebRTC 连接管理与 E2EE 加密
  - 实现 `RTCPeerConnection` 创建与管理：ICE 配置（STUN/TURN）、SDP Offer/Answer 交换
  - 实现 DataChannel 创建与管理：二进制模式（`binaryType = "arraybuffer"`）、消息编解码
  - 实现 Mesh 拓扑连接管理：多 Peer 并发连接、连接状态追踪
  - 实现 ECDH 密钥交换（Web Crypto API）：`EcdhKeyExchange` 消息、Pairwise 密钥管理
  - 实现 AES-256-GCM 加解密：消息加密/解密、密钥刷新
  - 实现页面刷新连接恢复：读取 localStorage → TokenAuth → ActivePeersList → 限并发重建 PeerConnection（2-3 个并发）→ ECDH 重新协商
  - 实现 `PeerEstablished`/`PeerClosed` 信令通知
  - 编写 WASM 测试：Web Crypto API ECDH 密钥交换、AES-256-GCM 加解密
  - _需求：Req 1 (Signaling)、Req 5 (E2EE)、Req 10.3 (Connection Recovery)_

- [ ] 16. 实现聊天系统核心功能
  - 实现文本消息发送/接收：Markdown 渲染（粗体/斜体/代码块/链接）、URL 自动检测、XSS 过滤
  - 实现消息状态管理：发送中 → 已发送 → 已送达 → 已读 → 发送失败，重发按钮
  - 实现消息 ACK 机制：`MessageAck` 发送/接收、未确认消息队列（IndexedDB 持久化）
  - 实现已读回执：`MessageRead` 批量发送（500ms 窗口）、隐私设置集成
  - 实现消息撤回：`MessageRevoke` 发送/接收、2 分钟时限、"已撤回" 占位显示
  - 实现输入状态指示器：`TypingIndicator` 发送/接收
  - 实现 @提及功能：高亮显示、特殊通知
  - 实现 Sticker 面板：资源清单加载、缩略图网格、分类切换、搜索、Cache API 缓存、版本管理
  - 实现语音消息：Opus 录制（Web Audio API）、Canvas 波形可视化（30fps）、播放进度指示器、120 秒上限
  - 实现图片消息：文件选择/剪贴板粘贴、缩略图生成、点击查看原图（缩放/滑动浏览）
  - 实现消息转发：转发目标选择 Modal、`ForwardMessage` 发送、转发消息展示（"Forwarded from" 头部）、禁止链式转发
  - 实现消息 Reaction：emoji 选择器、`MessageReaction` 发送/接收、toggle 行为、20 种 emoji 上限、IndexedDB 持久化
  - 实现消息回复引用：回复预览栏、`reply_to` 字段、引用消息块展示、点击跳转原消息、已撤回消息处理
  - 实现消息列表滚动行为（Req 14.11）：
    - Auto-Scroll：用户在底部（150px 内）时新消息自动滚动到底部（200ms smooth scroll）；用户发送消息时强制滚动到底部
    - "New messages ↓" 浮动徽章：用户滚动到上方阅读历史时显示，展示未读计数，点击平滑滚动到底部（300ms）
    - Scroll-to-Message 跳转导航：点击回复引用/搜索结果跳转到目标消息，高亮闪烁（1.5s 黄色渐隐），目标不在已加载范围时从 IndexedDB 加载周围消息（前 25 + 后 25）
    - "↓ Back to latest" 浮动按钮：跳转到历史消息后显示，点击重新加载最新 50 条并滚动到底部
    - 未读消息分隔线（Unread Divider）："── {N} New Messages ──" 分隔线，>50 条未读时滚动到分隔线位置而非底部
  - 编写单元测试：消息格式化、XSS 过滤、ACK 队列管理、Reaction 状态管理、滚动位置判断逻辑
  - 编写 WASM 测试：IndexedDB 消息读写、消息编解码
  - _需求：Req 2 (Chat System)、Req 11.3 (Message ACK)、Req 14.11 (Message List Scrolling Behavior)_

- [ ] 17. 实现消息持久化与离线支持
  - 实现 IndexedDB 存储层：消息表（含 Reaction 数据）、头像缓存表、搜索索引表、置顶会话表
  - 实现消息历史加载与虚拟滚动（Req 14.11.2 + 14.11.3）：
    - Virtual Scrolling：>100 条消息激活虚拟滚动，仅渲染可视区域 + overscan buffer（上下各 3 条），DOM 节点回收
    - 消息高度估算与缓存：按消息类型估算高度（text 可变、image 固定比例、voice ~60px、file ~80px、sticker ~120px），实际高度缓存到 `HashMap<MessageId, f64>`，高度修正不可感知
    - 快速滚动占位符：momentum scroll 时使用 skeleton 占位，减速后替换为实际内容
    - Infinite Scroll 历史加载：滚动到顶部加载 50 条旧消息、prepend 后保持滚动位置、防重复加载 debounce、"Beginning of conversation" 分隔线
    - 切换会话时加载最近 50 条消息并滚动到底部
  - 实现消息搜索：当前会话/全局搜索、关键词模糊匹配、分页加载策略（每批 5000 条）、结果高亮、相关性排序
  - 实现轻量级倒排索引（>50000 条消息时自动构建，存储在 IndexedDB 独立 object store）
  - 实现未确认消息队列持久化与自动重发（DataChannel 重建后）
  - 实现消息去重（基于 message_id）
  - 实现 72 小时过期清理（可配置：24h/72h/7天）
  - 实现 IndexedDB 空间不足自动清理
  - 编写 WASM 测试：IndexedDB CRUD、搜索性能（10000 条消息 < 500ms）、去重逻辑、虚拟滚动高度计算
  - _需求：Req 11 (Persistence)、Req 7.6 (Message Search)、Req 14.11.2 (Virtual Scrolling)、Req 14.11.3 (Infinite Scroll)_

- [ ] 18. 实现音视频通话与屏幕共享
  - 实现通话发起/接受/拒绝/结束流程（`CallInvite`/`CallAccept`/`CallDecline`/`CallEnd`）
  - 实现 Mesh 拓扑视频通话：Grid Layout 自适应布局、参与者视频流管理
  - 实现音视频模式切换：语音 ↔ 视频无缝切换（不重建连接）
  - 实现静音/关闭摄像头：本地 track 控制、状态广播
  - 实现 VAD 语音活动检测：当前说话人高亮
  - 实现屏幕共享：`getDisplayMedia()` → 大画面展示 + 其他参与者小画面
  - 实现网络质量监控：`getStats()` 每 5 秒采集、4 级分类、自动降级/恢复策略
  - 实现 PiP 画中画模式
  - 实现来电通知弹窗
  - 实现通话时长统计
  - 实现通话状态刷新恢复（localStorage 持久化 + 恢复确认弹窗）
  - 编写单元测试：通话状态机、网络质量分类算法、降级/恢复逻辑
  - _需求：Req 3 (AV Call)、Req 7.1-7.5 (AV Features)、Req 10.5 (Call Recovery)_

- [ ] 19. 实现文件传输系统
  - 实现文件选择（文件选择器 + 拖拽）与元数据准备（SHA-256 哈希计算）
  - 实现 DataChannel 分片传输：动态 chunk 大小（初始 64KB）、流控（bufferedAmount 监控）
  - 实现传输进度 UI：进度条、传输速度、预估剩余时间
  - 实现断点续传：chunk bitmap 追踪、PeerConnection 重连后自动续传
  - 实现多人串行传输策略：逐 Peer 串行传输、独立进度 + 总体进度
  - 实现文件大小限制：单人 100MB、多人 20MB、剧场本地视频不限
  - 实现危险文件扩展名警告（.exe/.bat/.sh 等）
  - 实现文件消息卡片 UI：文件名、大小、类型图标、下载按钮、危险标识
  - 编写单元测试：分片/重组逻辑、SHA-256 校验、流控算法
  - _需求：Req 6 (File Transfer)_

- [ ] 20. 实现用户发现、连接邀请与黑名单 UI
  - 实现在线用户列表面板：实时更新、搜索/过滤、用户信息卡片
  - 实现连接邀请 UI：发送邀请按钮、"邀请中" 状态、接受/拒绝弹窗、超时处理
  - 实现多人邀请 UI：多选用户、批量发送
  - 实现 "已连接" 状态标识：点击直接进入聊天
  - 实现黑名单功能：拉黑/取消拉黑、localStorage 存储、自动延迟拒绝（30-60 秒随机）
  - 实现黑名单管理面板
  - 编写单元测试：邀请状态机、黑名单逻辑
  - _需求：Req 9 (Discovery)、Req 9.2 (Blacklist)_

- [ ] 21. 实现房间系统与权限管理 UI
  - 实现房间创建 UI：类型选择（Chat/Theater）、名称/描述/密码设置
  - 实现房间列表 UI：房间信息展示、加入/密码输入
  - 实现房间成员列表：角色徽章（👑/⭐）、成员搜索（实时过滤）
  - 实现管理操作 UI：踢出/禁言/封禁/解禁/提升/降级/转让所有权
  - 实现管理操作确认对话框（破坏性操作）
  - 实现房间公告面板：可折叠、富文本编辑、字符计数、预览
  - 实现昵称管理 UI：修改昵称、实时广播更新
  - 实现禁言状态 UI：输入框禁用、倒计时显示
  - 编写单元测试：权限检查逻辑、成员搜索过滤
  - _需求：Req 4 (Room)、Req 15 (Profile & Permissions)_

- [ ] 22. 实现剧场模式（Theater Mode）
  - 实现剧场创建/加入/离开 UI
  - 实现视频源选择：本地文件（MP4/WebM/OGG）、在线 URL、CORS 错误处理
  - 实现 `captureStream()` / `mozCaptureStream()` 视频流捕获
  - 实现 Star 拓扑视频分发：Owner → 各 Viewer 的 PeerConnection 管理
  - 实现播放控制（Owner Only）：播放/暂停、进度条、音量、全屏
  - 实现字幕支持：SRT/WebVTT 解析、字幕同步显示、`SubtitleData`/`SubtitleClear` 广播、外观自定义
  - 实现弹幕系统：Canvas/CSS 渲染、颜色/位置设置、密度控制、透明度/字号/速度设置
  - 实现弹幕批量合并转发（Owner 端 50ms 批次）、负载过高自动降频
  - 实现剧场消息面板：文本消息收发、自动滚动
  - 实现 Owner 断线处理：30 秒等待重连、超时通知
  - 实现 Owner 资源监控：带宽利用率、bufferedAmount 监控、自动降级
  - 实现剧场 UI 布局：桌面端（左视频+右面板）、移动端（上视频+下面板）、全屏模式
  - 编写单元测试：SRT/WebVTT 解析、弹幕渲染逻辑、批量合并算法
  - _需求：Req 12 (Theater)_

- [ ] 23. 实现设置页面
  - 实现音视频设置：默认设备选择、音量调节、视频质量偏好
  - 实现外观设置：主题切换（System/Light/Dark）、语言切换（中/英）、字号调节（小/中/大）
  - 实现隐私安全设置：黑名单管理入口、在线状态可见性、已读回执开关
  - 实现通知设置：消息通知开关、来电通知开关、免打扰时段
  - 实现数据管理：清除聊天记录（选择性）、清除缓存（显示大小）、导出数据（JSON/HTML）、Debug Logs 入口（打开 Debug 面板）、"Generate Diagnostic Report" 按钮（生成诊断报告 JSON 下载）
  - 实现设置页 UI：侧边栏/抽屉布局、即时保存反馈、权限状态显示
  - 实现所有设置项的 localStorage 持久化与 Leptos Signal 响应式更新
  - 编写单元测试：设置项持久化/恢复、数据导出格式、诊断报告生成（验证不含敏感数据）
  - _需求：Req 13 (Settings)、requirements.md (Observability - Frontend Logging System - Diagnostic Report)_

- [ ] 24. 实现 UI 交互细节与无障碍
  - 实现会话置顶/免打扰/归档：置顶排序（按置顶时间）、最多 5 个、IndexedDB 持久化、归档自动取消
  - 实现网络质量指示器 UI：4 格信号图标、hover 详细 tooltip、Poor 质量 toast 通知
  - 实现全局 "连接断开/重连中" Banner
  - 实现浏览器通知（Notification API）：新消息/来电通知
  - 实现消息列表滚动性能优化（Req 14.11.6）：
    - CSS `contain: content` 布局隔离、`content-visibility: auto` 离屏优化
    - `will-change: transform` 仅在滚动时启用，空闲 2 秒后移除释放 GPU 内存
    - 图片/媒体 aspect-ratio 占位防止布局偏移、`loading="lazy"` 懒加载
    - 性能目标：滚动 FPS ≥ 55、新消息渲染 < 8ms、50 条历史 prepend < 50ms、DOM 节点 ≤ 200
  - 实现键盘导航：Tab 焦点移动、Escape 关闭弹窗、方向键列表导航
  - 实现 ARIA 标签：所有交互元素的 aria-label、aria-live 区域（新消息/来电）
  - 实现焦点指示器：所有可聚焦元素的 outline 样式
  - 实现颜色对比度：WCAG 2.1 AA 标准（正常文本 4.5:1、大文本 3:1）
  - 编写单元测试：置顶排序逻辑、通知权限检查
  - _需求：Req 7.7 (Pinning/Archive)、Req 14.10 (Network Quality)、Req 14.11.6 (Scroll Performance Optimization)、requirements.md (Accessibility)_

- [ ] **Phase 3 测试门禁**
  - 运行 `makers test-unit`：前端工具函数覆盖率 ≥ 80%
  - 运行 `makers test-wasm`：所有 WASM 测试通过（IndexedDB、Web Crypto、消息编解码、i18n）
  - 运行 `makers lint`：Clippy pedantic 零警告
  - 手动验证：登录页面、聊天页面、房间列表、剧场页面、设置页面在 Chrome/Firefox/Edge 最新两个版本中正常渲染
  - 手动验证：响应式布局在 Desktop/Tablet/Mobile 三档断点下正常工作

---

## Phase 4: 整合联调与 E2E 测试

> 前后端联调，验证完整的用户流程，确保所有功能端到端可用。

- [ ] 25. 前后端联调与 PWA 配置
  - 联调信令通信：WebSocket 连接 → TokenAuth → 用户列表同步 → 房间管理
  - 联调 WebRTC 连接：邀请 → SDP/ICE 交换 → PeerConnection 建立 → DataChannel 通信
  - 联调聊天功能：文本/Sticker/语音/图片消息端到端收发、消息 ACK、已读回执
  - 联调音视频通话：发起/接受/模式切换/屏幕共享/挂断
  - 联调剧场模式：创建/加入/视频播放/弹幕/字幕/Owner 管理
  - 联调刷新恢复：页面刷新 → TokenAuth → ActivePeersList → 连接重建 → 消息补发
  - 联调权限系统：踢出/禁言/封禁/提升/降级/转让所有权
  - 实现 PWA 配置：`manifest.json`、Service Worker（静态资源 cache-first）、离线 Banner
  - 实现 Dockerfile 多阶段构建 + docker-compose.yml
  - _需求：所有需求的端到端验证_

- [ ] 26. E2E 测试与最终验收
  - 编写 Playwright E2E 测试用例：
    - 用户注册 → 登录 → 连接邀请 → 聊天消息收发 → 登出
    - 多人房间创建 → 加入 → 群聊 → 离开
    - 音视频通话发起 → 模式切换 → 挂断
    - 页面刷新 → 连接恢复 → 消息补发
    - 剧场创建 → 视频播放 → 弹幕收发
    - 消息转发 → Reaction → 回复引用
    - 房间管理：踢出/禁言/封禁/提升/降级
  - 运行完整测试套件：`makers test`（unit + integration + wasm + e2e）
  - 性能验证：FCP < 2s（4G）、WASM bundle < 500KB（gzipped）、消息列表渲染 < 16ms
  - 浏览器兼容性验证：Chrome/Firefox/Edge 最新两个版本
  - 更新 README.md 文档：项目介绍、技术栈、快速开始、开发指南、部署说明
  - _需求：requirements.md (Testing Strategy)、requirements.md (Performance)、requirements.md (Browser Compatibility)_
