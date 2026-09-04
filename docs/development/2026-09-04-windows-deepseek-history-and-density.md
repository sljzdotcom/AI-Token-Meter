# Windows DeepSeek 历史窗口与界面密度修复

**需求：** `REQ-20260904-006`

**日期：** 2026-09-04

**状态：** 已并入 `main`；自动化与独立复审完成，`0.3.0-preview.3` 发布后等待 Windows 11 真机确认

## 背景与范围

Windows 真机同时暴露了四类问题：点击 DeepSeek 详情会隐式创建空白官网窗口，该窗口无法可靠关闭；详情内的 **Sync official history** 没有可见反馈；三个 Provider 详情与 Settings 字号偏大；Display font 原生下拉列表出现白底白字。确认采用方案 A：查看详情只打开原生详情，只有用户显式点击同步才创建托管官网窗口；Windows 使用独立紧凑密度和可读的浅色原生控件，macOS 源码与视觉保持不变。

本修复阶段本身没有改版本号、创建 tag 或发布 Release；修复分支随后通过 PR #6 合入 `main`。已公开的 `0.3.0-preview.2` 不包含本修复，`0.3.0-preview.3` Release Notes 明确列出 `REQ-20260904-006`，发布完成后可用于真机验收。

## 根因

1. `show_provider_detail` 在 DeepSeek 历史为空时同时调用官网同步，混淆了“查看详情”和“发起网页登录”两个用户动作。
2. 官网 WebView2 的窗口、分片组装、超时与关闭状态分散，缺少单一会话代次和唯一终态所有权；迟到回调、重复点击或窗口动作失败可能污染重开的会话。
3. React 详情页吞掉同步命令错误，也没有订阅固定同步状态；用户看不到 opening/active/failed，自动隐藏还可能在同步中关闭详情。
4. Windows 详情与 Settings 沿用较松的默认字号和间距；原生 `select/option` 没有明确浅色 scheme、前景与背景，在系统主题组合下可能得到白底白字。

## 测试驱动证据

| 阶段 | RED | GREEN | Git 检查点 |
| --- | --- | --- | --- |
| 查看详情边界 | 新用例观察到空历史详情会产生 `starts_official_history_sync: true` | 同一定向测试 8/8；详情只执行显示窗口与发布当前详情 | `5390840` |
| 窗口生命周期 | 状态机模块和 `LoadTimedOut` 行为尚不存在，定向测试分别以缺失导入/枚举失败 | 初版 15/15、完整 Rust 138/138 | `2fdc31a` |
| 会话竞态加固 | 新协调器先使旧运行时导入失败；生产突变分别复现旧回调污染、页面完成误激活、终态争用和清理失败误报 | 生命周期/安全 18/18、完整 Rust 141/141 | `1f252e8` |
| 前端同步反馈 | 新用例因缺少 opening/active/failed、禁用按钮和倒计时暂停而失败；迟到 reject 与监听清理再分别复现竞态 | 前端由 19/19 增至 22/22 | `7e06b04`、`a03e9ce` |
| 紧凑密度 | 组件缺少紧凑语义类；将详情正文从 14px 突变到 15px 时真实浏览器门禁失败；移除 Settings 系统字体规则同样失败 | 前端 24/24；Chrome 计算样式固定为详情 14/20/24/13/18px、Settings 14/20/13/32px，select/option 深字白底 | `e3d699c`、`7eec606` |
| 浏览器门禁韧性 | 原脚本可能遗留 Vite 子进程且浏览器无硬超时；Node 用例先因受管生命周期模块不存在失败 | 初版 4/4 进程生命周期测试；Unix 进程组 TERM→KILL、浏览器 15 秒超时与 finally 清理 | `16b0f5e`、`fc0f80d` |
| 最终四项 Important | 有效首片后无独立停滞期限；旧初始查询/命令拒绝可覆盖活跃会话；已有图表时清理失败无恢复入口 | generation 绑定 20 秒停滞期限、统一 attempt 接收边界、活跃会话保留、图表与失败重试共存；DeepSeek Rust 34 项和前端 43 项覆盖 | `fd90008`–`5612923` |
| Windows 浏览器 runner | Windows jsdom 可见性树超时；Node 不能直接 spawn `npm.cmd`；Vite ANSI 输出、stdout 关闭时序、Chrome profile 复用和后台 `requestAnimationFrame` 依次使门禁失败 | 测试去除无关样式遍历；Node 直启 Vite；ANSI 解析、`close` 收流、Windows 独立临时 profile 与同步计算样式采样；Node 生命周期 12/12，Windows Chrome production 密度门禁通过 | `e3dc723`–`1309d30` |

RED 均来自缺失或错误的生产行为，不使用沙箱端口限制作为失败先行证据。任务 1–4 的完整命令、突变与输出保存在同一计划的执行报告中。

## 窗口生命周期

| 状态 | 进入条件 | 可见动作 | 终止与恢复 |
| --- | --- | --- | --- |
| `idle` | 没有官网同步会话 | DeepSeek 详情可正常自动隐藏 | 显式点击同步后进入 `opening` |
| `opening` | 用户点击 **Sync official history** | 创建唯一隐藏 WebView2，显示 Opening；暂停详情自动隐藏；重复 IPC 继续复用但不得提前显示空壳 | 只有 nonce 绑定的官方 ready 回调可继续；30 秒未就绪转 `failed` |
| `active` | 桥已安装，且官方页出现结构化登录表单，或精确官网 usage 成功信号与已渲染应用主界面同时存在 | 以 provider/revision token 暂停原 DeepSeek 详情，显示并聚焦官网窗口；重复请求只聚焦现有 active 窗口 | 用户关闭转 `cancelled`；完整聚合转 `completed`；错误转 `failed`；用户选择其他详情会使旧恢复 token 失效 |
| `completed` | 唯一会话取得终态所有权并成功应用/发布聚合 | 销毁官网窗口，恢复更新后的详情 | 恢复正常自动隐藏 |
| `cancelled` | 当前代次收到关闭或销毁 | 清理组装器和计时器，恢复详情 | 可再次显式同步 |
| `failed` | 构建、加载、聚焦、解析、应用、发布或清理失败 | 尽力销毁官网窗口、恢复详情并显示固定重试提示 | 可点击 **Try again**；恢复正常自动隐藏 |

每次会话都有单调代次和随机 nonce。导航、ready、分片、关闭、销毁与超时回调都携带创建时的代次；旧代次只能得到 `Ignored`，不能改变新会话。`completed` 会先占有终态，再把聚合交给应用层，避免关闭或超时在数据发布途中抢占。

## Windows 界面密度

- Provider 详情：正文 14px、身份标题 20px、主数值 24px、区块标题 13px、卡片关键数字 18px。
- Settings：基准 14px、标题 20px、控件 13px、最小控件高度 32px；继续强制使用 `Segoe UI Variable`/`Segoe UI`，不继承用户选择的展示字体。
- Settings 明确 `color-scheme: light`；Display font 的 `select` 和 `option` 固定 `#151821` 前景、`#fff` 背景，并保留既有 `:focus-visible`。
- 密度门禁先用独立 Vite production build 生成 fixture，再由 Vite preview/Chrome 读取产物并执行 `getComputedStyle()`；同时证明 meter/详情仍可使用 Antonio，而 Settings 被系统字体隔离。

这些是 Windows 专属 React/CSS 变化；没有修改 `Sources/`、`Tests/` 或 macOS AppKit/SwiftUI 样式。

## 安全边界

- 官网导航只接受精确 `https://platform.deepseek.com`、标准 443 端口、无用户名和密码；不增加其他 host、scheme 或重定向例外。
- WebView2 使用公开固定路径语义 `%LOCALAPPDATA%\AI Token Meter\WebView2\DeepSeek` 的独立 profile，不读取 Edge、Chrome 或其他浏览器会话。
- ready 与历史分片只经私有回调位置进入 Rust；要求当前代次、32 位随机十六进制 nonce、官方 origin、有界分片/总大小、有效期与严格 DTO。
- 状态事件和只读查询只包含 `{ generation, status }`；`status` 只可能是 `idle/opening/active/completed/cancelled/failed` 六个固定小写词，前端忽略未知 payload 与非当前代次。
- 缓存只保存日期、成本、请求、Token 和更新时间等标准化聚合。API Key、Cookie、Authorization、DOM 文本、登录字段、网页原始响应、路径和个人账号标识不会进入事件、日志或业务缓存。
- 本文、执行报告和测试输出只记录固定脱敏状态与公开仓库路径，不含个人 key、Cookie 或账号。

## 整分支最终审查修复

最终审查在 `de2bc50` 基线上把九项发现作为同一修复合同处理；每一项先以缺失接口、错误状态或可控交错得到 RED，再修改生产实现并回到 GREEN：

| 项 | RED 证据 | GREEN 结果 |
| --- | --- | --- |
| 登录与分片期限 | 长登录后的第一片在旧 click-to-expiry 模型中得到 `Expired`；15 分钟精确边界未终止 | 交互会话独立限制为 15 分钟；20 秒只从第一片通过 envelope 校验的分片开始 |
| 原子历史合并 | 带旧非空历史的余额结果稳定覆盖更新历史；历史写失败原先仍可进入 completed | 历史 API 只接收两个历史字段，在刷新发布锁内先耐久写再更新内存；两种发布顺序及缓存内容一致，失败不改内存 |
| generation 状态 | 延迟的 A 代终态会覆盖 B 代 active，迟到 open 响应还会把同代 active 倒退到 opening | 事件、打开返回和查询统一为 `{ generation, status }`；前端只接收当前代次并保证同代状态单调；重试下界过滤排队旧事件，但 authoritative 返回仍可重绑仍存活的同代会话 |
| 状态通道恢复 | 监听注册失败仍可发起同步并永久停在 opening；状态事件丢失没有 UI 恢复路径 | 监听失败须先完成查询握手才启用按钮；同步中轮询只读状态，事件发送失败仍能恢复 |
| 页面就绪 | 空 `#root` 与仅渲染错误面的页面可过早 active | ready 要求桥存在并观察到 credential/submit 登录结构，或同时具备精确官网 usage 成功信号、导航与可交互主界面；仅接口成功的空壳及带 Retry 的错误面仍失败，普通非阻断 alert 不误判整页失败 |
| 销毁失败重试 | `DestroyHistory` 失败后 session 被清空，固定标签残窗阻塞新建 | 保留 cleanup ownership；重试核对 Tauri 注册表、销毁残窗并确认移除后才开放新 generation |
| 详情所有权 | 官网终态无条件重发 DeepSeek，能覆盖新选 Claude/Codex 或明确关闭；原生 show/emit 部分成功还可让物理窗和状态分裂 | token 绑定 provider/revision；检查、恢复与失败回滚在同一锁内，失败归一到新 Closed revision；内部失焦不清 ownership，数据更新不再强制 active detail |
| Unix 进程组 | 父进程先退出时旧等待只看到 leader 已结束并提前返回 | 整个进程组享有完整 TERM 宽限；期满后用负 PGID + signal 0 探测全组，后代仍活时继续 KILL，Windows `taskkill /T /F` 不变 |
| production 密度证据 | 测试文档称 production，脚本实际启动 Vite dev server | `test:density` 先生成独立 production fixture，再启动 Vite preview 并由真实 Chrome/Edge 读取构建产物 |

历史完成先发布独立 `snapshot-updated`，是否恢复详情由 ownership token 另行决定。所有外部错误仍映射为固定文本；新增事件与查询没有网页正文、Cookie、API Key、表单值、账号、电话或本机路径。

## 本机自动化验证

最终验证以本日志同提交前的新鲜输出为准：

- `npm --prefix windows test`：2 个文件，43/43；
- `npm --prefix windows run test:density`：先构建 production fixture，Node 生命周期 12/12，Vite preview + Chrome 计算样式通过；首次沙箱运行仅因禁止绑定 `127.0.0.1` 失败，获准使用回环后同命令通过；
- `npm --prefix windows run build`：TypeScript 与 Vite production build 通过；
- `cargo test --locked --all-targets --all-features --manifest-path windows/src-tauri/Cargo.toml`：169/169；既有 HTTP fixture 使用获准回环；
- `cargo fmt --check --manifest-path windows/src-tauri/Cargo.toml`：通过；
- `cargo clippy --locked --all-targets --manifest-path windows/src-tauri/Cargo.toml -- -D warnings`：通过、零警告；
- `bash scripts/test.sh`：macOS 主测试 375/375（71 组）与独立 PTY 12/12（1 组）；随后跨平台合同 4 份 fixture、合同可移植性、Windows 发布资产标准化、release feed 回归、148 份 Markdown 和公开安全扫描全部通过；
- `AI_METER_INCLUDE_WIDGET=0 bash scripts/build-app.sh`：生成并验证无 Widget、ad-hoc 签名的 Apple Silicon App；资源与 Sparkle bundle 检查通过；
- `plutil`、`verify-app-resources.sh`、严格 `codesign`、`verify-update-bundle.sh`、Mach-O 与 AppIcon 检查全部通过，主程序为 arm64。

PR #6 的 [Windows workflow 33878105470](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/33878105470) 已在 `windows-latest` 通过 43 项前端、12 项密度生命周期、真实 Chrome 计算样式、严格 Rust 格式/Clippy、169 项原生测试、Release Tauri/NSIS、PE GUI subsystem 和安装器上传；[macOS workflow 33878105480](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/33878105480) 同时全绿。它们是源码与构建门禁，不替代交互式 Windows 11 真机登录、窗口焦点和原生下拉弹层证据。

## Windows 11 真机验收（待用户确认）

真机验收的代码门禁已经开放。最终定向复审的四项 Important 已关闭：

1. 从第一片有效分片开始建立 generation 绑定的 20 秒传输停滞计时器；停滞自动失败并允许重试。
2. 为前端初始状态查询建立 attempt/epoch 所有权，旧查询不得覆盖用户随后启动的新会话。
3. 命令返回失败且后续查询失败时，保留同一次尝试中已由事件确认的活跃会话；查询得到的所有合法终态继续走 generation 感知的统一接收路径。
4. 即使已有历史图表，官网窗口销毁失败也必须提供可达的清理/重试动作，或由窗口销毁事件自动核销逻辑所有权。

独立复审在 `5612923` 未发现剩余 Critical/Important；其后改动只加固 CI 测试基础设施。PR #6 双平台 CI 和本机完整门禁均已通过。并入 `main` 后，以下步骤仍需安装 `0.3.0-preview.3`，在交互式 Windows 11 中执行：

1. 安装 `0.3.0-preview.3`，完全退出旧进程后重新启动；确认现有 DeepSeek Key 遮罩与余额仍可用，不在报告中提交 Key 或账号截图。
2. 依次打开 Claude Code、OpenAI Codex、DeepSeek 详情，确认标题、主数字、卡片和区块标题明显更紧凑且没有截断；同时确认 macOS 外观没有变化。
3. 在 DeepSeek 没有新历史时点击圆环：只应出现原生详情，不得自动创建官网窗口。
4. 点击 **Sync official history**：详情立即显示 Opening 且按钮禁用；官网窗口只在官方页 ready 后出现并获得前台焦点，不应出现第二个空白窗口。
5. 用标题栏关闭官网窗口：窗口应关闭，DeepSeek 详情恢复并可再次同步；active 期间的重复请求只聚焦同一个窗口，opening/activating 期间不得提前显示尚未 ready 的空壳，也不得新建重复窗口。
6. 使用真实 DeepSeek 账号完成官网登录并等待用量/费用聚合：同步期间详情不得被自动隐藏；完成后官网窗口销毁，详情恢复并显示最近 30 天图表。
7. 模拟网络失败或在 opening 阶段等待超时：详情应恢复、显示固定重试提示，点击 **Try again** 可重新开始。
8. 打开 Settings 的四个 Tab，确认整体字号与控件高度紧凑；展开 Display font，下拉列表在未悬停、悬停和键盘焦点下都应为白底深色文字，所有选项可辨认，Settings 仍使用 Windows 系统字体。

上述登录、关闭、复用聚焦、真实聚合和原生下拉弹层依赖交互式 Windows 11/WebView2 会话。本机 Chrome 计算样式与跨平台 Rust 测试不能替代这些证据，因此需求保持 `待用户确认`，不得标为 `已完成`。

## Git 节点

- `5390840`：查看详情不再隐式同步；
- `2fdc31a`、`1f252e8`：窗口生命周期与会话竞态加固；
- `7e06b04`、`a03e9ce`：前端同步反馈与竞态加固；
- `e3d699c`、`7eec606`、`16b0f5e`、`fc0f80d`：紧凑密度、真实浏览器门禁与进程回收；
- 任务 5 文档与本机验证检查点：`5f7a141`；
- `4953206`、`5982d5c`：第一轮整分支审查修复与证据记录；随后定向复审重新打开 4 项 Important，形成下一轮失败先行基线；
- `fd90008`–`5612923`：关闭四项 Important，并完成无 Critical/Important 的最终独立复审；
- `e3dc723`–`1309d30`：修复 Windows runner 的前端性能、Vite 启动/ANSI、浏览器输出、profile 复用与后台帧调度差异；
- PR #6：[Windows CI 33878105470](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/33878105470) 与 [macOS CI 33878105480](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/33878105480) 全绿；`0.3.0-preview.2` 不含本修复，`0.3.0-preview.3` 为首次交付版本。
- PR #6 已于 2026-09-04 合并为 `e62193c`；需求保持待用户确认，直到 `0.3.0-preview.3` 完成 Windows 11 真实登录、关闭、聚合和原生下拉验收。
- 合并记录提交 `d520752` 的 [Windows main CI 33880527388](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/33880527388) 与 [macOS main CI 33880527365](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/33880527365) 均全绿；Windows 再次完成真实 Chrome、169 项原生测试、Release NSIS 和 PE GUI subsystem 验证。
