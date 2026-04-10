# 实施计划 — E2E Messaging Test (Requirement 16)

> **前置条件**: Phase 1-3 的核心功能已实现并通过各自的测试门禁；信令服务器可编译为独立二进制文件；前端可通过 Trunk 构建为静态资源。
>
> **测试框架**: Playwright (TypeScript)，仅 Chromium，headless 模式
>
> **文件结构约定**:
> ```
> e2e/
> ├── playwright.config.ts          # Playwright 配置
> ├── package.json                  # 依赖管理
> ├── tsconfig.json                 # TypeScript 配置
> ├── fixtures/
> │   ├── server.ts                 # 信令服务器生命周期管理
> │   ├── test-base.ts              # 扩展 Playwright test fixture（双浏览器上下文）
> │   └── helpers.ts                # registerAndLogin / establishConnection / sendAndVerifyMessage
> ├── assets/
> │   ├── test-image.png            # 测试用图片
> │   ├── test-file.pdf             # 测试用文件
> │   └── test-large-file.bin       # 超限文件（>100MB，gitignore，CI 动态生成）
> ├── specs/
> │   ├── auth.spec.ts              # 注册/登录/会话恢复
> │   ├── invitation.spec.ts        # 连接邀请流程
> │   ├── text-messaging.spec.ts    # 文本消息收发/状态/Markdown/URL
> │   ├── rich-messaging.spec.ts    # 贴纸/语音/图片消息
> │   ├── message-actions.spec.ts   # 上下文菜单/回复/引用/撤回/复制
> │   ├── forward-reaction.spec.ts  # 转发/Reaction
> │   ├── conversation-list.spec.ts # 会话列表/未读计数/@提及
> │   ├── persistence.spec.ts       # 消息持久化/刷新恢复/ACK 重发
> │   ├── file-transfer.spec.ts     # 文件传输
> │   ├── multi-user.spec.ts        # 多人聊天（3 浏览器上下文）
> │   ├── disconnect.spec.ts        # 断开/重连
> │   ├── scrolling.spec.ts         # 消息列表滚动行为
> │   ├── theme-a11y.spec.ts        # 主题切换/无障碍
> │   └── e2ee.spec.ts              # E2EE 加密验证
> └── utils/
>     ├── selectors.ts              # 统一 CSS/data-testid 选择器常量
>     └── wait-helpers.ts           # 自定义等待工具（waitForDataChannel 等）
> ```

---

- [ ] 1. 搭建 Playwright 项目结构与测试基础设施
   - 初始化 `e2e/` 目录，创建 `package.json`（Playwright + TypeScript 依赖）、`tsconfig.json`、`playwright.config.ts`
   - 在 `playwright.config.ts` 中配置：仅 Chromium 项目、headless 默认、30s 超时、2 次重试、HTML reporter、screenshot on failure
   - 实现 `fixtures/server.ts`：信令服务器生命周期管理（随机端口启动、健康检查轮询、SIGTERM 优雅关闭、日志收集附加到报告）
   - 实现 `fixtures/test-base.ts`：扩展 Playwright `test` fixture，自动创建双浏览器上下文（Context A / Context B），设置 Chromium flags（`--use-fake-device-for-media-stream`、`--use-fake-ui-for-media-stream`、`--allow-insecure-localhost`）
   - 实现 `fixtures/helpers.ts`：`registerAndLogin()`、`establishConnection()`、`sendAndVerifyMessage()` 三个核心 helper 函数
   - 实现 `utils/selectors.ts`：统一定义所有 `data-testid` 选择器常量（sidebar、user-list、chat-input、message-bubble 等）
   - 实现 `utils/wait-helpers.ts`：`waitForDataChannel()`、`waitForMessageStatus()`、`waitForOnlineUser()` 等自定义等待工具
   - 实现唯一用户名生成器：`test_user_{testId}_{timestamp}_{random}` 模式
   - 准备测试资源文件：`assets/test-image.png`、`assets/test-file.pdf`
   - 在 `Makefile.toml` 中添加 `test-e2e` 任务：`cd e2e && npx playwright test`
   - 验证：运行一个空的 smoke test（启动服务器 → 打开页面 → 截图 → 关闭），确认基础设施工作正常
   - _需求：16.1.1, 16.1.2, 16.1.3, 16.1.4, 16.1.5, 16.1.6, 16.1.7_

- [ ] 2. 编写用户注册/登录与连接邀请 E2E 测试
   - 创建 `specs/auth.spec.ts`：
     - 测试用例：成功注册并进入主界面（验证 sidebar 用户名、Identicon 头像、在线状态）
     - 测试用例：重复用户名注册失败（验证错误提示、停留在注册页）
     - 测试用例：双用户注册后互相可见（验证在线用户列表双向显示）
     - 测试用例：页面刷新后会话自动恢复（验证无需重新登录、状态保持）
   - 创建 `specs/invitation.spec.ts`：
     - 测试用例：点击用户 → 信息卡片 → 发送邀请 → 对方接受 → 进入聊天（完整 happy path）
     - 测试用例：邀请被拒绝（验证提示信息、按钮恢复可点击）
     - 测试用例：邀请超时（60s 超时，使用 `test.slow()` 标记）
     - 测试用例：双向同时邀请自动合并（验证自动连接、无需手动接受）
   - 验证：所有测试通过，截图断言关键 UI 状态
   - _需求：16.2.1-16.2.4, 16.3.1-16.3.6_

- [ ] 3. 编写文本消息收发 E2E 测试
   - 创建 `specs/text-messaging.spec.ts`：
     - 测试用例：发送纯文本消息（验证双端显示、对齐方向、头像、时间戳）
     - 测试用例：消息状态流转（发送中 → 已送达 ✓✓，使用 `waitForMessageStatus` helper）
     - 测试用例：Markdown 渲染（发送 `**bold** _italic_ \`code\``，验证接收端 HTML 渲染）
     - 测试用例：URL 自动检测（发送含 URL 文本，验证 `<a>` 标签、`target="_blank"`、`rel="noopener"`）
     - 测试用例：快速连续发送多条消息（验证顺序正确、无丢失/重复）
     - 测试用例：已读回执（滚动消息到可视区域，验证发送端状态变为蓝色 ✓✓）
     - 测试用例：输入状态指示器（输入文字 → 对端显示 "typing..."，停止输入 → 消失）
   - 验证：所有测试通过
   - _需求：16.4.1-16.4.6_

- [ ] 4. 编写富媒体消息（贴纸/语音/图片）E2E 测试
   - 创建 `specs/rich-messaging.spec.ts`：
     - 测试用例：打开贴纸面板（验证网格布局、分类标签、搜索栏）
     - 测试用例：发送贴纸消息（点击贴纸 → 双端显示、面板关闭、图片加载正常）
     - 测试用例：贴纸搜索（输入关键词 → 过滤结果、无结果提示）
     - 测试用例：发送语音消息（模拟录制 → 双端显示波形 + 时长、播放按钮可用）
     - 测试用例：语音消息播放（点击播放 → 按钮变暂停、进度更新、播放完毕恢复）
     - 测试用例：通过文件选择器发送图片（验证双端缩略图显示、宽高比正确）
     - 测试用例：剪贴板粘贴图片（模拟粘贴 → 预览确认弹窗 → 确认发送 → 对端显示）
     - 测试用例：图片全屏预览（点击图片 → modal 打开、缩放控件、Escape 关闭）
   - 验证：所有测试通过
   - _需求：16.6.1-16.6.3, 16.7.1-16.7.3, 16.8.1-16.8.3_

- [ ] 5. 编写消息操作（上下文菜单/回复/撤回/转发/Reaction）E2E 测试
   - 创建 `specs/message-actions.spec.ts`：
     - 测试用例：右键消息显示上下文菜单（验证菜单项：Reply/Quote/Copy/Forward，自己消息含 Revoke）
     - 测试用例：回复消息（选择 Reply → 预览栏显示 → 发送回复 → 双端显示引用块）
     - 测试用例：点击引用块跳转原消息（验证滚动 + 高亮闪烁动画）
     - 测试用例：引用消息（选择 Quote → 输入框插入 blockquote 格式 → 发送）
     - 测试用例：撤回消息（2 分钟内，确认弹窗 → 双端显示 "已撤回" 占位）
     - 测试用例：超时消息无撤回选项（验证菜单中无 Revoke）
     - 测试用例：复制文本（验证剪贴板内容 + toast 提示）
     - 测试用例：撤回消息后回复引用更新（原消息撤回 → 回复中引用块显示 "已撤回" 灰色文字）
   - 创建 `specs/forward-reaction.spec.ts`：
     - 测试用例：转发消息（Forward → 目标选择 modal → 确认 → 目标会话显示 "Forwarded from" 头部）
     - 测试用例：转发消息禁止链式转发（转发消息的菜单无 Forward 选项）
     - 测试用例：添加 Reaction（hover → emoji picker → 选择 → 双端显示 pill + 计数）
     - 测试用例：双方添加相同 Reaction（计数变为 2、双端高亮）
     - 测试用例：取消 Reaction（再次点击 → 计数减少、高亮取消）
     - 测试用例：添加不同 Reaction（多个 pill 并排显示）
   - 验证：所有测试通过
   - _需求：16.9.1-16.9.7, 16.10.1-16.10.3, 16.11.1-16.11.5, 16.15.1_

- [ ] 6. 编写会话列表/未读计数/@提及 E2E 测试
   - 创建 `specs/conversation-list.spec.ts`：
     - 测试用例：收到消息时 sidebar 显示未读徽章（验证计数 = 1、最后消息预览、时间戳）
     - 测试用例：多条未读消息计数递增（1 → 2 → 3）
     - 测试用例：点击会话清除未读徽章（验证徽章消失、未读分隔线显示）
     - 测试用例：@提及自动补全（输入 "@" → 下拉列表 → 选择用户 → 插入格式化文本）
     - 测试用例：@提及消息高亮与通知（发送含 @mention → 对端高亮显示 + 特殊通知）
   - 验证：所有测试通过
   - _需求：16.12.1-16.12.3, 16.13.1-16.13.2_

- [ ] 7. 编写消息持久化/刷新恢复与文件传输 E2E 测试
   - 创建 `specs/persistence.spec.ts`：
     - 测试用例：刷新页面后聊天记录恢复（交换消息 → 刷新 → 验证 IndexedDB 恢复、顺序/内容正确）
     - 测试用例：刷新期间连接恢复流程（刷新 → "恢复连接中" 提示 → WebRTC 重建 → 继续收发）
     - 测试用例：刷新期间消息不丢失（B 在 A 刷新时发消息 → A 恢复后收到、无重复）
   - 创建 `specs/file-transfer.spec.ts`：
     - 测试用例：发送小文件（进度条 → 完成 → 对端文件卡片 + 下载按钮 → 内容校验）
     - 测试用例：超限文件拒绝（>100MB → 错误提示、不发送）
     - 测试用例：危险扩展名警告（.exe → 安全警告弹窗 → 确认后发送 + ⚠️ 标识）
   - 验证：所有测试通过
   - _需求：16.5.1-16.5.3, 16.14.1-16.14.4_

- [ ] 8. 编写多人聊天与断开/重连 E2E 测试
   - 创建 `specs/multi-user.spec.ts`（需要第三个浏览器上下文）：
     - 测试用例：三人聊天建立（A 邀请 B 和 C → 双方接受 → 聊天头部显示参与者数 3）
     - 测试用例：多人消息广播（A 发消息 → B 和 C 均收到、送达状态 "2/2"）
     - 测试用例：成员离开（C 离开 → A/B 收到通知 → 后续消息仅 A/B 收到）
   - 创建 `specs/disconnect.spec.ts`：
     - 测试用例：用户断开（B 关闭标签页 → A 的用户列表显示 B 离线 + 断开指示器）
     - 测试用例：用户重连（B 重新打开 → A 显示 B 在线 → 重新建立连接 → 历史记录保留）
   - 验证：所有测试通过
   - _需求：16.16.1-16.16.2, 16.17.1-16.17.3_

- [ ] 9. 编写滚动行为/主题/无障碍/E2EE 验证 E2E 测试
   - 创建 `specs/scrolling.spec.ts`：
     - 测试用例：底部自动滚动（用户在底部 → 新消息 → 自动滚动到底部）
     - 测试用例：阅读历史时不自动滚动（向上滚动 → 新消息 → 不滚动 + "New messages ↓" 徽章 → 点击徽章滚动到底部）
     - 测试用例：无限滚动加载历史（滚动到顶部 → loading spinner → 旧消息加载 → 滚动位置保持）
   - 创建 `specs/theme-a11y.spec.ts`：
     - 测试用例：主题切换（Light → Dark → 验证颜色变化、刷新后保持）
     - 测试用例：键盘导航（Tab 焦点移动、Enter 激活、Escape 关闭弹窗、焦点指示器可见）
     - 测试用例：ARIA 属性（消息容器 `aria-live="polite"`）
   - 创建 `specs/e2ee.spec.ts`：
     - 测试用例：E2EE 状态图标（建立会话后加密图标可见）
     - 测试用例：消息不经过信令服务器（通过网络检查验证 WebSocket 不传输聊天内容）
   - 验证：所有测试通过
   - _需求：16.18.1-16.18.3, 16.19.1-16.19.3, 16.20.1-16.20.2_

- [ ] 10. CI 集成与测试套件最终验收
   - 配置 `Makefile.toml` 中 `test-e2e` 任务的完整流程：`cargo build --release` → `cd e2e && npm ci && npx playwright install chromium && npx playwright test`
   - 编写 `.github/workflows/e2e.yml`（或等效 CI 配置）：安装 Rust + Node.js → 构建服务器 → 运行 E2E 测试 → 上传 HTML 报告 + 失败截图
   - 运行完整 E2E 测试套件，确认所有 spec 文件通过（零失败、零 flaky）
   - 验证 Playwright HTML 报告生成正确，包含：测试结果摘要、失败截图、服务器日志附件
   - 验证测试并行性：不同 spec 文件可并行运行（各自独立服务器实例）
   - 更新项目 README.md：添加 E2E 测试运行说明（`makers test-e2e`）
   - _需求：16.1（Implementation Notes 中的 CI Integration、Test Parallelism、Flakiness Mitigation）_
