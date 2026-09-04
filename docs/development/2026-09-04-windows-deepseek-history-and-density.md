# Windows DeepSeek 历史窗口与界面密度修复

**需求：** `REQ-20260904-006`

**日期：** 2026-09-04

**状态：** 自动化与本机可执行门禁完成；Windows 11 真实登录、关闭、聚焦和原生字体下拉仍待用户确认

## 背景与范围

Windows 真机同时暴露了四类问题：点击 DeepSeek 详情会隐式创建空白官网窗口，该窗口无法可靠关闭；详情内的 **Sync official history** 没有可见反馈；三个 Provider 详情与 Settings 字号偏大；Display font 原生下拉列表出现白底白字。确认采用方案 A：查看详情只打开原生详情，只有用户显式点击同步才创建托管官网窗口；Windows 使用独立紧凑密度和可读的浅色原生控件，macOS 源码与视觉保持不变。

本阶段没有改版本号、推送分支、创建 tag 或发布 Release。发布版本与正式资产只在 Windows runner 和真机验收完成后另行决定。

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
| 浏览器门禁韧性 | 原脚本可能遗留 Vite 子进程且浏览器无硬超时；Node 用例先因受管生命周期模块不存在失败 | 4/4 进程生命周期测试；Unix 进程组 TERM→KILL、浏览器 15 秒超时与 finally 清理 | `16b0f5e`、`fc0f80d` |

RED 均来自缺失或错误的生产行为，不使用沙箱端口限制作为失败先行证据。任务 1–4 的完整命令、突变与输出保存在同一计划的执行报告中。

## 窗口生命周期

| 状态 | 进入条件 | 可见动作 | 终止与恢复 |
| --- | --- | --- | --- |
| `idle` | 没有官网同步会话 | DeepSeek 详情可正常自动隐藏 | 显式点击同步后进入 `opening` |
| `opening` | 用户点击 **Sync official history** | 创建唯一隐藏 WebView2，显示 Opening；暂停详情自动隐藏 | 只有 nonce 绑定的官方 ready 回调可继续；30 秒未就绪转 `failed` |
| `active` | 官方页 ready 且显示/聚焦成功 | 隐藏详情的临时置顶，显示并聚焦官网窗口；重复点击只聚焦现有窗口 | 用户关闭转 `cancelled`；完整聚合转 `completed`；错误转 `failed` |
| `completed` | 唯一会话取得终态所有权并成功应用/发布聚合 | 销毁官网窗口，恢复更新后的详情 | 恢复正常自动隐藏 |
| `cancelled` | 当前代次收到关闭或销毁 | 清理组装器和计时器，恢复详情 | 可再次显式同步 |
| `failed` | 构建、加载、聚焦、解析、应用、发布或清理失败 | 尽力销毁官网窗口、恢复详情并显示固定重试提示 | 可点击 **Try again**；恢复正常自动隐藏 |

每次会话都有单调代次和随机 nonce。导航、ready、分片、关闭、销毁与超时回调都携带创建时的代次；旧代次只能得到 `Ignored`，不能改变新会话。`completed` 会先占有终态，再把聚合交给应用层，避免关闭或超时在数据发布途中抢占。

## Windows 界面密度

- Provider 详情：正文 14px、身份标题 20px、主数值 24px、区块标题 13px、卡片关键数字 18px。
- Settings：基准 14px、标题 20px、控件 13px、最小控件高度 32px；继续强制使用 `Segoe UI Variable`/`Segoe UI`，不继承用户选择的展示字体。
- Settings 明确 `color-scheme: light`；Display font 的 `select` 和 `option` 固定 `#151821` 前景、`#fff` 背景，并保留既有 `:focus-visible`。
- Vite/Chrome 门禁使用真实 production CSS 的 `getComputedStyle()`，同时证明 meter/详情仍可使用 Antonio，而 Settings 被系统字体隔离。

这些是 Windows 专属 React/CSS 变化；没有修改 `Sources/`、`Tests/` 或 macOS AppKit/SwiftUI 样式。

## 安全边界

- 官网导航只接受精确 `https://platform.deepseek.com`、标准 443 端口、无用户名和密码；不增加其他 host、scheme 或重定向例外。
- WebView2 使用公开固定路径语义 `%LOCALAPPDATA%\AI Token Meter\WebView2\DeepSeek` 的独立 profile，不读取 Edge、Chrome 或其他浏览器会话。
- ready 与历史分片只经私有回调位置进入 Rust；要求当前代次、32 位随机十六进制 nonce、官方 origin、有界分片/总大小、有效期与严格 DTO。
- 状态事件只可能是 `idle/opening/active/completed/cancelled/failed` 六个固定小写词；前端忽略未知 payload。
- 缓存只保存日期、成本、请求、Token 和更新时间等标准化聚合。API Key、Cookie、Authorization、DOM 文本、登录字段、网页原始响应、路径和个人账号标识不会进入事件、日志或业务缓存。
- 本文、执行报告和测试输出只记录固定脱敏状态与公开仓库路径，不含个人 key、Cookie 或账号。

## 本机自动化验证

最终验证以本日志同提交前的新鲜输出为准：

- `npm --prefix windows test`：2 个文件，24/24；
- `npm --prefix windows run test:density`：Node 生命周期 4/4，Chrome production CSS 计算样式通过；首次沙箱运行仅因禁止绑定 `127.0.0.1` 失败，获准使用回环后同命令通过；
- `npm --prefix windows run build`：TypeScript 与 Vite production build 通过；
- `cargo test --locked --manifest-path windows/src-tauri/Cargo.toml`：141/141；既有 HTTP fixture 使用获准回环；
- `cargo fmt --check --manifest-path windows/src-tauri/Cargo.toml`：通过；
- `cargo clippy --locked --all-targets --manifest-path windows/src-tauri/Cargo.toml -- -D warnings`：通过、零警告；
- `bash scripts/test.sh`：macOS 主测试 374/374（71 组）与独立 PTY 12/12（1 组）；随后跨平台合同 4 份 fixture、合同可移植性、Windows 发布资产标准化、release feed 回归、146 份 Markdown 和公开安全扫描全部通过；
- `AI_METER_INCLUDE_WIDGET=0 bash scripts/build-app.sh`：生成并验证无 Widget、ad-hoc 签名的 Apple Silicon App；资源与 Sparkle bundle 检查通过；
- `plutil`、`verify-app-resources.sh`、严格 `codesign`、`verify-update-bundle.sh`、Mach-O 与 AppIcon 检查全部通过，主程序为 arm64。

显式 `x86_64-pc-windows-msvc` Tauri Preview 构建已在 macOS 主机尝试。前端构建通过，随后 `ring` 原生依赖因主机没有 Windows MSVC C SDK/Header，在 `assert.h` 缺失处停止；这不是可在 macOS 上声明通过的 Windows 壳/NSIS 证据。Windows-only 编译、运行测试、PE GUI subsystem 与 NSIS 必须由后续 `windows-latest` workflow 完成。

## Windows 11 真机验收（待用户确认）

1. 安装包含本修复的下一版 Preview，完全退出旧进程后重新启动；确认现有 DeepSeek Key 遮罩与余额仍可用，不在报告中提交 Key 或账号截图。
2. 依次打开 Claude Code、OpenAI Codex、DeepSeek 详情，确认标题、主数字、卡片和区块标题明显更紧凑且没有截断；同时确认 macOS 外观没有变化。
3. 在 DeepSeek 没有新历史时点击圆环：只应出现原生详情，不得自动创建官网窗口。
4. 点击 **Sync official history**：详情立即显示 Opening 且按钮禁用；官网窗口只在官方页 ready 后出现并获得前台焦点，不应出现第二个空白窗口。
5. 用标题栏关闭官网窗口：窗口应关闭，DeepSeek 详情恢复并可再次同步；重复点击同步时只聚焦同一个窗口，不得新建重复窗口。
6. 使用真实 DeepSeek 账号完成官网登录并等待用量/费用聚合：同步期间详情不得被自动隐藏；完成后官网窗口销毁，详情恢复并显示最近 30 天图表。
7. 模拟网络失败或在 opening 阶段等待超时：详情应恢复、显示固定重试提示，点击 **Try again** 可重新开始。
8. 打开 Settings 的四个 Tab，确认整体字号与控件高度紧凑；展开 Display font，下拉列表在未悬停、悬停和键盘焦点下都应为白底深色文字，所有选项可辨认，Settings 仍使用 Windows 系统字体。

上述登录、关闭、复用聚焦、真实聚合和原生下拉弹层依赖交互式 Windows 11/WebView2 会话。本机 Chrome 计算样式与跨平台 Rust 测试不能替代这些证据，因此需求保持 `待用户确认`，不得标为 `已完成`。

## Git 节点

- `5390840`：查看详情不再隐式同步；
- `2fdc31a`、`1f252e8`：窗口生命周期与会话竞态加固；
- `7e06b04`、`a03e9ce`：前端同步反馈与竞态加固；
- `e3d699c`、`7eec606`、`16b0f5e`、`fc0f80d`：紧凑密度、真实浏览器门禁与进程回收；
- 任务 5 文档与本机验证检查点：`docs: prepare Windows DeepSeek density acceptance (REQ-20260904-006)`。
