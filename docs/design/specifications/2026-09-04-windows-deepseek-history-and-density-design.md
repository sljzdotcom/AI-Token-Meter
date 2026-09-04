# Windows DeepSeek 历史窗口与界面密度修复设计

**需求：** `REQ-20260904-006`

**日期：** 2026-09-04

**状态：** 已确认（用户选择方案 A）

## 问题与证据

Windows `0.3.0-preview.2` 真机已能启动并读取 DeepSeek 余额，但存在四组相互关联的体验缺陷：

1. 点击 DeepSeek 后，详情页之外还会出现标题为 “DeepSeek Usage · AI Token Meter” 的独立白色窗口；该窗口被详情窗口遮挡后无法正常关闭。
2. 详情页中的 “Sync official history” 看似没有响应。
3. Windows 三个 Provider 详情页与 Settings 的固定像素字号明显大于 macOS 的实际视觉尺度，标题尤为突出。
4. Settings 的 Display font 原生下拉框显示白底浅字，只有鼠标悬停的选项可辨认。

源码核对确认：

- `show_provider_detail` 在 DeepSeek 历史为空时会无条件创建官网 WebView，因此普通“查看详情”被错误地等同于“开始官网同步”。
- Provider 详情窗口在可见期间保持 always-on-top；官网 WebView 只是居中构建，没有加载、置前、关闭或失败生命周期，因而会落到详情窗口后方。
- 前端调用 `open_deepseek_history` 时丢弃了成功与失败结果，用户看不到正在打开、失败或可重试状态。
- Windows 样式沿用了为 macOS 视觉校准的较大固定字号；Settings 的原生 `select` 没有明确设置 `color-scheme`、前景色和 `option` 背景色。

## 方案比较

1. **显式同步 + 托管官网窗口生命周期（采用）：** 点击 DeepSeek 只打开详情；只有用户点击同步按钮才建立隔离官网窗口。窗口先隐藏加载，加载完成后置前；关闭、失败和成功均恢复一致状态。Windows 独立使用紧凑字号尺度并修复原生选择框配色。
2. **改用系统浏览器：** 可以避免应用内空白窗口，但隔离会话和自动聚合桥接无法可靠延续，不满足自动获取最近 30 天数据的目标。
3. **移除官网历史同步：** 界面最简单，但会删除已经承诺的 DeepSeek 30 天历史能力，不采用。

## 交互设计

### 查看详情

- 点击 DeepSeek 圆环只显示 DeepSeek 详情，不自动创建第二个窗口。
- Claude Code、OpenAI Codex 和 DeepSeek 的详情显示、自动隐藏及点击外部关闭保持原行为。

### 开始同步

- 历史为空时显示一个主操作按钮。
- 用户点击后按钮进入 `Opening official page…` 状态并禁用重复点击。
- 后端创建唯一的隔离 WebView2 窗口，但初始 `visible(false)`，避免 SPA 尚未渲染时出现大片白色窗口。
- 固定 DeepSeek 官网页面安装采集桥且出现 credential/submit 登录结构，或精确官网 usage 成功信号与已渲染导航/可交互主界面同时存在后，详情窗口才取消临时置顶并隐藏，官网窗口显示、置前和聚焦；仅 `DOMContentLoaded`、仅接口成功的空 SPA 根节点或带 Retry 的错误主界面不能取消 30 秒就绪看门狗。普通非阻断 alert 不得把完整应用误判为失败。Windows 标准标题栏与关闭按钮必须完整可操作。
- 可见登录会话最长 15 分钟；历史分片另有独立 20 秒传输期限，并从第一片通过 nonce、origin 与大小校验的分片开始计时，登录、短信、CAPTCHA 或同意步骤不消耗分片期限。

### 完成、取消与失败

- 聚合桥接接收到完整、校验通过的 30 天数据后：在 `UsageRuntime` 的刷新发布临界区只合并历史字段，耐久写成功后才更新内存；随后发布快照、关闭官网窗口，并仅在仍持有原 DeepSeek 详情 revision 时恢复详情。
- 用户关闭官网窗口后：结束本次会话，恢复原 DeepSeek 详情并显示可再次同步的按钮。
- 窗口构建、页面加载或导航失败后：销毁官网窗口，恢复详情并显示简短、可恢复的错误消息。
- 前端命令失败不得静默吞掉；同一时间只允许一个官网窗口和一个组装会话。
- 官网窗口 active 时再次请求同步，应直接聚焦现有窗口；opening/activating 阶段只能复用并保持隐藏，不得绕过 ready 门槛，也不销毁后重建。销毁失败则保留 cleanup ownership，下一次重试先与真实窗口注册表核对并完成清理，再创建新代次。

## 状态与事件

- 后端状态快照固定为 `{ generation, status }`；`status` 只允许 `idle | opening | active | completed | cancelled | failed`，`generation` 为当前或最近会话代次。
- `open_deepseek_history` 返回同一状态快照，`deepseek_history_status` 提供只读恢复查询。事件发送失败时真实状态仍先写入协调器，因此可由查询恢复。
- 前端只有在事件监听成功，或监听失败后状态查询握手成功时，才允许开始同步；同步中定时查询补偿丢失的状态事件。
- 新同步开始后只接受当前 generation；重试保留上一代下界来拒绝 open 返回前的排队旧事件，同一尝试的 authoritative open/query 仍可重绑后端复用的同代会话；同代次倒退状态不能覆盖新会话。
- 事件与查询不得包含 Cookie、API Key、网页响应正文、路径或账户标识。
- 官网窗口打开期间暂停详情自动隐藏；恢复详情后重新开始正常倒计时。
- 窗口关闭事件必须清理组装会话，避免旧 nonce 或分片污染下一次同步。
- 详情窗口所有权以 provider 与单调 revision 标记；同步期间用户选择 Claude/Codex、重新选择 DeepSeek 或明确关闭详情，都会使旧恢复 token 失效。

## Windows 视觉密度

- 只修改 `windows/` 前端；macOS SwiftUI/AppKit 样式不变。
- Provider 详情采用紧凑 Windows 尺度：正文约 `13–14px`，身份标题 `20px`，主数值 `24px`，区块标题 `12–13px`，卡片关键数字 `17–18px`。
- 卡片间距、内边距和最小高度同步小幅收紧，保留现有层级、颜色、Antonio 字体选择和 440×760 的逻辑窗口上限。
- Settings 永远使用 Segoe UI 系统字体，基准字号 `14px`，主标题约 `20px`，控件约 `13px`，控件高度约 `32px`。
- Settings 显式声明浅色控件主题；`select` 与 `option` 使用深色文字和白色背景，选中、悬停和键盘焦点保持 Windows 原生可访问反馈。

## 安全边界

- 保留现有固定 HTTPS 主机、443 端口、无用户名密码、固定回调 scheme、随机 nonce、分片上限和敏感字段拒绝规则。
- DeepSeek 官网使用独立 WebView2 profile，不读取 Edge/Chrome Cookie，不把 DeepSeek API Key 注入 WebView。
- 不把官网私有接口响应、Cookie 或登录信息写入日志；仅保存已脱敏的按日成本、请求数和 token 聚合。

## 测试与验收

- Rust 策略测试覆盖：查看详情不触发自动同步；官网窗口状态的打开、激活、关闭、成功与失败转移；重复打开只聚焦现有窗口。
- 前端测试覆盖：同步按钮状态、命令错误反馈、同步期间暂停自动隐藏、结束后恢复。
- 浏览器行为测试覆盖：先构建独立 production fixture，再由 Vite preview/Chrome 或 Edge 核对 Windows 详情与 Settings 的紧凑字号变量，以及字体选择框和选项的深色文字/白色背景合同。
- 交错测试覆盖：余额与历史双向发布顺序、旧 generation 事件、详情 select/close/restore、销毁失败重试，以及 Unix 父进程已退出但同组后代仍存活的清理路径。
- 运行 Windows 前端、Rust、rustfmt、严格 Clippy、Tauri 构建，以及 macOS 和文档/公开安全门禁。
- Windows 11 真机最终确认：点击 DeepSeek 不再弹窗；点击同步后官网窗口能加载、置前、关闭；成功后图表更新；三详情与 Settings 字号适中；字体下拉项始终可读。

## 非目标

- 不更改 macOS 详情或 Settings 字号。
- 不改变 DeepSeek 余额、100 元基准或圆环计算方式。
- 不改用手工 ZIP 导入，也不把官网历史伪装成公开 API 能力。
- 不在本次修复中改变浮动条轮廓、背景、位置保存或更新通道。
