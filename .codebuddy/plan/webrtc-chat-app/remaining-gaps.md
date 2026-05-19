# 剩余 4 个 gaps 的落地方案

> 截至 commit `09c11d2` 时点，e2e-coverage-plan.md 中尚未关闭的 gap 共 4 项：
> **G21**（删除会话）、**G26**（avatar 上传）、**G27**（theater 视频 fixture）、
> **G28**（nickname 服务端持久化）。本文档为每个 gap 给出**最小可行方案**、
> **代码触点清单**、**测试覆盖策略**与**风险/工作量评估**，每项均可作为一个独立
> commit 落地，不互相阻塞。
>
> 排序按"实施成本从低到高": **G28 → G21 → G27 → G26**。

---

## G28 — Nickname 服务端持久化

### 现状（已确认）
- `server/src/auth/mod.rs:56` — `UserSession.nickname: String` 字段已存在；
- `server/src/auth/mod.rs:324-328` — `AuthSuccess` 已经从 `UserSession.nickname` 读取
  下发（注释 W1 还预告"future task"会改）；
- `server/src/ws/room/mod.rs:942` — `handle_nickname_change` 调用
  `ws_state.room_state.set_nickname(...)`，但**只写 room-scoped `MemberInfo`，
  从不触碰 `UserStore`**；
- `server/src/ws/state` 持有 `Arc<UserStore>`（auth 共用同一实例），handler 内可访问。

### 方案：服务端持久化新 nickname

#### 修改点（≤ 30 行）

1. **`server/src/auth/mod.rs`** — 给 `UserStore` 加一个公开方法：
   ```rust
   /// Update the persisted nickname for `user_id`. Returns `true`
   /// when the row was found and the nickname differed (i.e. an
   /// actual write happened). Validation (length, charset) is the
   /// caller's responsibility — `RoomState::set_nickname` already
   /// performs it.
   pub fn set_nickname(&self, user_id: &UserId, new_nickname: &str) -> bool {
     if let Some(mut session) = self.users.get_mut(user_id) {
       if session.nickname != new_nickname {
         session.nickname = new_nickname.to_string();
         return true;
       }
     }
     false
   }
   ```

2. **`server/src/ws/room/mod.rs:942`** — 在 `room_state.set_nickname` 成功后，
   也写一次 `user_store`：
   ```rust
   match ws_state.room_state.set_nickname(&nickname_change) {
     Ok(()) => {
       // G28: persist on the global User table so AuthSuccess
       // after reload returns the canonical nickname.
       ws_state
         .user_store
         .set_nickname(user_id, &nickname_change.new_nickname);
       // ... existing broadcast unchanged ...
     }
   }
   ```

3. **不需要修改协议** — `AuthSuccess.nickname` 已是 wire 上的真相源。

### 测试覆盖

| 层 | 测试 | 触点 |
|---|---|---|
| Rust unit | `UserStore::set_nickname` 三态：missing user、相同值、新值 | `server/src/auth/tests/...` |
| Rust integration | `handle_nickname_change` 后 `user_store.get(...).nickname` 已更新 | `server/src/ws/room/tests/...` |
| E2E | `profile.spec.ts` 把 in-session 断言扩展到 reload：edit → save → `page.reload()` → 顶栏/UserInfoCard 的 nickname 仍是新值 | `e2e/specs/profile.spec.ts` |

### 风险与边界
- 用户**不在任何 room** 时无法 set nickname — `room_state.set_nickname` 会返回
  `UserNotInRoom`。这是产品设计选择（昵称改动必须广播到当前 room）。如果未来要
  做"settings 内独立改昵称"，需要新增 `SignalingMessage::UpdateProfile` 走单独
  路径直接打 `UserStore`，本 gap 不覆盖。
- 多设备登录暂时按 single-device，无 cross-session 推送问题。

### 工作量
**0.5 天**。代码 ≤ 30 行，测试 ≤ 80 行，e2e 增量 ≤ 10 行。

---

## G21 — 删除会话

### 现状（已确认）
- 持久化层**已具备删除能力**：`frontend/src/persistence/store/messages.rs:169`
  `delete_conversation(db, conv_key)` 已实现并被 `manager/wasm.rs:268` 包装为
  `PersistenceManager::delete_conversation`；
- `AppState` 已有 `toggle_pin/mute/archive`，但**没有 `delete_conversation`
  方法**；
- `SidebarConversationMenu` 当前 3 个 menu item（Pin/Mute/Archive），没有
  Delete 项；
- 没有 i18n key、没有 testid、没有确认对话框。

### 方案：软删除（"从列表移除"）+ 二次确认

采用"软删除"语义而非完全擦除，理由：
1. 已加密的端到端历史从对方那里仍然存在；删自己 IDB 后，重新打开 1:1 会话
   会从 peer ack 中重新拉到记录，无法真正"消失"；
2. 与现有 archive（隐藏到 archived section）形成清晰的层级：
   - **Archive**: 移到 archived section，可恢复；
   - **Delete**: 从 `app_state.conversations` 移除 + IDB 删消息 + 标 tombstone，
     新消息到来时按"unknown peer → 新建会话"重新出现。

#### 修改点（约 200 行 + 测试）

1. **`frontend/src/state/mod.rs`** — 新增 `delete_conversation` 方法：
   ```rust
   /// Hard-remove `conversation_id` from the sidebar list and
   /// asynchronously delete its persisted messages + flags.
   /// New inbound traffic from the same peer will re-create the
   /// conversation as if it were never seen before.
   ///
   /// Returns `true` when a conversation actually existed and was
   /// removed.
   pub fn delete_conversation(&self, conversation_id: &ConversationId) -> bool {
     let mut removed = false;
     self.conversations.update(|convs| {
       let before = convs.len();
       convs.retain(|c| &c.id != conversation_id);
       removed = convs.len() < before;
     });
     if removed {
       // Drop messages signal so the chat view unmounts cleanly.
       self.messages_by_conversation.update(|m| {
         m.remove(conversation_id);
       });
       // Persist deletion async — fire-and-forget on a fallback
       // PersistenceManager handle (same pattern as flags).
       schedule_delete_in_idb(conversation_id.clone());
     }
     removed
   }
   ```

2. **`frontend/src/state/persistence.rs`** — 新增辅助函数：
   ```rust
   fn schedule_delete_in_idb(conv: ConversationId) {
     wasm_bindgen_futures::spawn_local(async move {
       if let Some(pm) = try_use_persistence_manager() {
         let _ = pm.delete_conversation(&conv).await;
         let _ = pm.delete_conversation_flags(&conv).await;
       }
     });
   }
   ```
   注：`delete_conversation_flags` 需要在 `manager/wasm.rs` 中新增（仿 messages 删除）。

3. **`frontend/src/components/sidebar/sidebar_conversation_menu.rs`** — 新增第 4
   个 menu item，外加 props 上加 `on_delete: Callback<()>` 让父行触发确认对话框：
   ```rust
   <button
     type="button"
     class="sidebar-conversation-menu__item sidebar-conversation-menu__item--danger"
     role="menuitem"
     data-testid="sidebar-conversation-menu-delete"
     on:click=move |_| {
       open.set(false);
       on_delete.run(());
     }
   >
     <Icon icon=i::LuTrash2 />
     <span>{move || t_string!(i18n, sidebar.delete)}</span>
   </button>
   ```

4. **`frontend/src/components/sidebar/sidebar_conversation_item.rs`** — 接住
   `on_delete`，通过共享的 `Dialog`（confirm 类型）弹二次确认：
   ```rust
   let on_delete = Callback::new(move |_| {
     dialog_manager.confirm(
       t_string!(i18n, sidebar.delete_confirm_title),
       t_string!(i18n, sidebar.delete_confirm_body),
       move || app_state.delete_conversation(&id),
     );
   });
   ```

5. **i18n keys**（en + zh-CN，最少 4 条）：
   - `sidebar.delete` = "Delete" / "删除"
   - `sidebar.delete_confirm_title` = "Delete conversation?" / "删除会话？"
   - `sidebar.delete_confirm_body` = "Messages on this device will be removed. The other side keeps their copy." / "本设备上的消息将被移除，对方仍保留副本。"
   - `sidebar.delete_confirm_ok` = "Delete" / "删除"

6. **CSS**（`styles/components/sidebar.css`）— `--danger` 修饰符红色文字：
   ```css
   .sidebar-conversation-menu__item--danger { color: var(--color-danger); }
   .sidebar-conversation-menu__item--danger:hover { background: var(--color-danger-soft); }
   ```

### 测试覆盖

| 层 | 测试 | 文件 |
|---|---|---|
| Rust unit | `delete_conversation` 三态：存在/不存在/重复删 | `frontend/src/state/tests/wasm_interactions.rs` |
| Rust unit | 删除后再 `auto_unarchive` 同 id 不 panic | 同上 |
| Rust unit | dirty set 不再持有已删 id（避免下轮 persist 写僵尸行） | 同上 |
| Rust unit | IDB 持久化层 `delete_conversation_flags`（仿 messages 删除测） | `frontend/src/persistence/store/conv_flags.rs` |
| E2E | `conv-list-management.spec.ts` test 5：右键打开 menu → 点 Delete → 确认 dialog → row 消失；reload 后仍消失 | `e2e/specs/conv-list-management.spec.ts` |
| E2E | test 6（可选）：删除后对方发来一条新消息，会话以"unread"状态重新出现 | 同上 |

### 风险与边界
- **回填问题**：删除后 ack_queue 重放可能把已删消息再写回。需要在
  `chat::manager::inbound::push_incoming` 入口检查 conversation 是否在
  `app_state.conversations` 中；不在则按"新会话"路径走（已自然成立，因为新
  消息本就会走 `ensure_conversation`）。
- **会话搜索 deps**：搜索索引（IDB `STORE_SEARCH_INDEX`）有
  `delete_search_index_for_conversation`，要一并调用，否则搜索会出僵尸命中。
  已存在的函数，加一行调用即可。

### 工作量
**1.5 天**。这是最重的一项：要同时改 state、UI、persistence、i18n、CSS、tests。

---

## G27 — Theater 视频源测试 fixture

### 现状（已确认）
- `theater.spec.ts` 当前覆盖 owner-side source picker + URL 校验失败路径；
- 缺：codec-compatible 的微型视频文件 + 一个 readiness oracle；
- `<video>` 元素本身被 `loadedmetadata` 事件 gate（不触发就不 mount），常规
  pixel buffer 不行。

### 方案：放一个 < 5 KB 的 WebM/VP8 stub + 用 `videoWidth>0` 作 readiness

#### 选择 fixture 格式
- **首选 WebM/VP8**：Chromium 一定支持，且 ffmpeg 可生成 ~2 KB 的 1×1 单帧 webm。
- 备选 MP4/AVC1：体积更小（~800 字节）但 Chromium headless build 的 codec
  支持有时被 strip。

#### 生成命令（一次性，结果 check 进仓）
```bash
# 1×1 像素，1 帧，1 fps，VP8 编码 → ~1.4 KB
ffmpeg -hide_banner -loglevel error \
  -f lavfi -i color=black:s=1x1:r=1 \
  -t 1 -c:v libvpx -b:v 10k -f webm \
  e2e/assets/tiny.webm
```
- 写入 `e2e/assets/tiny.webm`（与现有 `tiny.png` 同目录）。
- 在 commit message 中记录"由 ffmpeg N.N.N 生成，单帧 VP8，无音轨"。

#### 测试 readiness oracle
Playwright 等待 `<video>` 元素满足：
```ts
await page.locator('[data-testid="theater-video"]').evaluate(
  (el: HTMLVideoElement) =>
    new Promise<void>((resolve) => {
      const tick = () => {
        if (el.videoWidth > 0 && el.readyState >= 2 /* HAVE_CURRENT_DATA */) {
          resolve();
        } else {
          requestAnimationFrame(tick);
        }
      };
      tick();
    }),
  { timeout: 10_000 }
);
```
不依赖 `play()` — 单帧 webm `<video autoplay muted>` 的 `loadedmetadata` 触发
后 `videoWidth` 立即为 1。

#### 新增 E2E 断言（3 项）
1. **owner-select-local-video**: owner 在 source picker 选 `tiny.webm` →
   `theater-video` 元素挂载 → `videoWidth > 0`；
2. **viewer-join-and-see-frame**: viewer 加入同一房间 → `theater-video` 元素
   挂载（通过 SDP 协商 + replaceTrack）→ `videoWidth > 0`；
3. **cross-peer-danmaku**: owner 发送一条弹幕 → viewer 的弹幕 overlay 出现
   同 testid 节点。

> #2 和 #3 依赖 webrtc track replace 协商，本身已有 `replace_track` 路径，
> P0-5 的 A/V call 测试已验证过该机制；这里仅是数据源换成"本地 file URL"
> 而非 mic/cam。

#### 修改点

1. **fixture**: `e2e/assets/tiny.webm`（二进制，~1.4 KB）；
2. **selectors**: `e2e/utils/selectors.ts` 加 `theaterVideo`、`theaterDanmakuItem`；
3. **frontend testids**:
   - `theater/video_stage.rs` 的 `<video>` 加 `data-testid="theater-video"`；
   - `theater/danmaku/...` 单条弹幕加 `data-testid="theater-danmaku-item"`；
4. **theater.spec.ts** 扩展 3 个测试用例，复用现有 `pageA/pageB` 模式。

### 风险与边界
- **Chromium headless codec drift**：CI 运行环境换 image 后可能突然不支持
  VP8。缓解：fixture commit 时同时存一个 `tiny.mp4`（H.264），运行时探测：
  ```ts
  const vid = document.createElement('video');
  const can = vid.canPlayType('video/webm; codecs="vp8"');
  ```
  优先 webm，回退 mp4。
- **viewer 端 SDP 协商时延**：现有 P0-5 显示 1-3 s 是稳定的，给 10 s 超时
  足够。

### 工作量
**1 天**。fixture 生成 + 3 个 e2e 测试 + 2 个 frontend testid patch + readiness
helper。无 Rust 改动。

---

## G26 — Avatar 上传

### 现状（已确认）
- `message/src/types/structs.rs:30` — `UserInfo.avatar_url: Option<String>`
  已在协议里，且各处 `serde(skip_serializing_if = "Option::is_none")` 设置
  正确（向后兼容）；
- `server/src/auth/mod.rs:97` — `UserSession::to_user_info` 硬编码
  `avatar_url: None`；`UserSession` 里**没有** `avatar_url` 字段；
- 客户端 `UserInfoCard` 渲染 `<img>` 但 src 始终为空（identicon 兜底）；
- 没有 protocol message 用来推送 avatar 变更；
- 没有 UI picker。

### 方案：分两阶段，本项目只做"前端选图 + base64 内嵌"路径

完整的"上传到对象存储 → 拿 URL → 持久化"需要后端 HTTP 上传端点 + S3/MinIO
依赖，超出当前 e2e gap 的修复范围。**实务建议两步走**：

#### Phase A（本 gap 落地范围）— Data URL 直存
- 协议字段已经是 `Option<String>`，不规定 URL scheme；
- 前端在 settings drawer 中加 `<input type="file" accept="image/*">`，
  resize 到 ≤ 64×64 + 转 webp（用 OffscreenCanvas）→ 输出 ~3-8 KB 的
  `data:image/webp;base64,...`；
- 客户端校验：解码后维度 ≤ 64×64、字节数 ≤ 16 KB；
- 通过新增 `SignalingMessage::AvatarChange { user_id, avatar_url }`
  推送，结构镜像 `NicknameChange`；
- 服务端在 `UserSession` 加 `avatar_url: Option<String>` 字段，
  `handle_avatar_change` 写入 + 广播 + `to_user_info` 返回非 None；
- `AuthSuccess` 加 `avatar_url: Option<String>` 字段，reload 持久化。

#### Phase B（未来 ticket，**不在本 gap**）
- 后端加 `POST /api/avatar` 端点，上传到 object store，返回 CDN URL；
- 前端切到上传路径，data URL 作为兜底。

#### 修改点（Phase A，约 250 行）

1. **`message/src/signaling/moderation.rs`** — 新增 `AvatarChange`：
   ```rust
   #[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
   pub struct AvatarChange {
     pub user_id: UserId,
     /// New avatar URL or data URL. `None` clears the avatar.
     pub avatar_url: Option<String>,
   }
   ```

2. **`message/src/signaling/mod.rs`** — `SignalingMessage::AvatarChange(AvatarChange)`
   variant + discriminator + 单测三件套（roundtrip + discriminator + enum 顺序）。

3. **`server/src/auth/mod.rs`**:
   - `UserSession` 加 `pub avatar_url: Option<String>`；
   - `to_user_info` 改读真实字段；
   - `UserStore::set_avatar(user_id, Option<String>) -> bool`，仿 G28 的
     `set_nickname`；
   - `AuthSuccess` 加同名字段，从 `users.get(...)` 读出下发。

4. **`server/src/ws/room/mod.rs`** — 新增 `handle_avatar_change`，仿
   `handle_nickname_change`：校验 self → 写 `user_store` → 广播给
   "可见" peers（已有 active connection 的 user list）。

5. **`frontend/src/components/room/avatar_editor.rs`**（新文件）— 仿
   `nickname_editor.rs`：
   - 文件 input + 客户端裁切；
   - canvas resize 到 64×64 → `toBlob('image/webp', 0.85)` → base64；
   - testid: `avatar-input`、`avatar-preview`、`avatar-save-btn`；
   - 校验失败（>16 KB / 非 image / 解码失败）显示 alert。

6. **`frontend/src/components/settings_page/page.rs`** — Account section
   挂载 `<AvatarEditor />`，位置在 NicknameEditor 上方。

7. **`frontend/src/components/sidebar/user_info_card.rs`** — 已渲染
   `<img>`，确认它跟 `app_state.users` 里的 avatar_url 联动（应已自然
   reactive）。

8. **i18n keys**（en + zh-CN）：
   - `settings.avatar` / `settings.avatar_pick`
   - `settings.avatar_too_large` / `settings.avatar_invalid_format`
   - `settings.avatar_save` / `settings.avatar_saved`

### 测试覆盖

| 层 | 测试 |
|---|---|
| Rust message | `AvatarChange` bitcode/serde roundtrip、discriminator 唯一、`SignalingMessage` enum 顺序匹配 |
| Rust server | `UserStore::set_avatar`、`handle_avatar_change` 拒绝越权（A 给 B 改 avatar）、广播命中 active peers |
| Rust server | `AuthSuccess` reload 带回 avatar_url |
| Frontend WASM | `AvatarEditor` 文件大小/类型校验单元测试 |
| E2E | `profile.spec.ts` test 5：选小图 → save → user-info-card 的 `<img>` src 变成 data url；reload 后仍带 |
| E2E | test 6：选超大文件 → 显示 size error，state 不变 |

### 风险与边界
- **bitcode payload 膨胀**：avatar 即便 8 KB，每次 AvatarChange 广播给
  online 用户都是 N×8 KB。可在 server 端做 dedup（同 URL 不重发）+ 仅广播
  给"在线的会话对端"；
- **Data URL 在 IDB 里巨**：用户头像走 `app_state.users` 已不持久化，没问题；
  但 `AuthSuccess` 重新下发会占内存。可以接受；
- **未来切到 CDN URL**：协议字段就是 `Option<String>`，无需 break change，
  Phase B 升级零摩擦。

### 工作量
**2 天**。最大的一项：跨 3 个 crate（message / server / frontend）+ 文件
处理 + i18n + tests。

---

## 总表

| Gap | 工作量 | 跨 crate | 协议变更 | 是否阻塞其他工作 |
|---|---|---|---|---|
| G28 nickname 持久化 | 0.5 天 | server only | 无 | 解锁 profile.spec reload 断言 |
| G21 删除会话 | 1.5 天 | frontend only | 无 | 解锁 conv-list-management.spec test 5/6 |
| G27 视频 fixture | 1 天 | e2e only (+ 2 testid) | 无 | 解锁 theater.spec 3 个断言 |
| G26 avatar 上传 | 2 天 | message + server + frontend | **新增** `AvatarChange` | 解锁 profile.spec test 5/6 |

**合计：5 天**，可拆 4 个独立 commit。**推荐顺序 G28 → G21 → G27 → G26**，理由：

1. **G28 是最小、收益最高**：30 行代码把 nickname 完整闭环，解锁已有 e2e 的
   reload 断言；
2. **G21 是纯前端**：不需要后端配合，IDB 删除已具备，最大风险是 UX 设计
   （软删 vs 硬删）— 本方案给出明确选择；
3. **G27 测试 only**：不动产品代码（仅 testid 增量），ffmpeg fixture 一次
   性生成；
4. **G26 最重，最后做**：协议+服务端+前端三处都动，且涉及 file picker，
   留到最后做能复用前三项站稳的回归网。

## 全部完成后

- e2e-coverage-plan.md 的 4 个 `~` 状态全部 → `[x]`；
- Wave P2 完整收口；
- Req 16 AC 覆盖率：~94% → ~98%（剩 ~2% 是 NFR / 工具链相关，明确
  out-of-scope）。
