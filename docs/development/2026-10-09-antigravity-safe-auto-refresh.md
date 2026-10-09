# Antigravity 安全恢复边界

**需求：** REQ-20261009-001
**现场基线：** AI Token Meter 0.10.7/build35、Antigravity CLI 1.3.1；用户截图额度时间为2026-10-08 23:26（Asia/Singapore），Gemini两个窗口剩余100%，浮动条仍因先前service/process error暂停。更早的0.10.6/build34诊断属于历史状态，不能作为当前版本说明。
**工作区：** `/private/tmp/AI-Meter-safe-auto-refresh`
**基线提交：** `4158fea`（0.10.7发布证据），安全恢复候选实现`6b014a7`，本次纠正提交在完成后记录。
**状态：** 暂停安全保护已纠正；恢复周期自动刷新仍受环境限制；未发布。

## 结论

候选`6b014a7`在一次登录回执和quota读取成功后清除了持久`suspended`，并让已有定时器恢复后续常规采集。这个转移超出代码和模拟测试所能证明的安全边界。当前纠正使一次显式登录后的成功`agy -p /usage`只更新本地额度缓存；内存及持久暂停不清除，UI明确说明自动刷新仍暂停。

因此这项纠正确保M4 Max现有暂停不会被一次成功检查误解锁，但没有完成“恢复正常周期自动刷新”的目标，也不是一个可发布的自动刷新修复版本。

## 官方能力与适用边界

- Google官方[Headless mode](https://antigravity.google/docs/cli/headless/)写明：`-p`运行单次非交互任务，使用缓存凭据；在“无终端”的非交互环境，未认证会退出并报告`authentication required`而不是等待输入；`/usage`必须作为独立headless调用，因为CLI在Agent stream之外自行处理它。该文档没有明确承诺GUI会话下的headless进程绝不通过LaunchServices/open打开浏览器。
- Google官方[Installation and auth](https://antigravity.google/docs/cli/install/)说明：本机CLI读取系统安全密钥环的有效会话；找不到保存会话时会自动启动默认浏览器。这使上述两页文档之间的本机GUI过期会话行为仍需具体CLI版本的明确保证。
- Google官方[Model Quotas](https://antigravity.google/docs/cli/commands/usage/)说明`/usage`会刷新后端额度。
- `ProcessGroupCommandRunner`把stdin接到`/dev/null`，stdout/stderr接管道，创建独立session/process group，不创建PTY；每条CLI调用有界超时、输出上限、取消和TERM/KILL清理。源码和fake测试能证明本应用不提供终端输入、参数与时限合同、假进程的清理行为；它们不能证明真实`agy`不调用LaunchServices。
- 普通`GeminiCollector.collect()`除了版本和quota，还运行headless `/model`及不带`-p`的`agy models`。此前候选在解锁后会进入该路径；这些补充命令的认证/GUI行为没有被文档或fake替身证明。该路径发现是独立复审补查的结果。
- 版本校验接受主版本1且不低于1.1.28。fake测试覆盖1.1.28和1.2.2；现场截图确认CLI 1.3.1，但本轮未运行真实二进制。代码不拒绝更高的未知1.x版本，因此不能把整个1.x范围描述为已现场兼容。

## 刷新、失败及持久状态

- 安全恢复按钮只在当前待处理token收到成功登录回执后签发一次性协调器授权；恢复查询固定执行`agy --version`及`agy -p /usage --print-timeout 20s`，stdin为空且不创建PTY；它不运行`/model`或`models`。
- 成功、错误、无效响应、取消、并发启动、缓存写入失败和协调器重建后的状态由定向测试覆盖；成功额度只写入Gemini缓存，已有暂停保持，重启后自动/普通手动刷新不调用Gemini collector。
- 普通Gemini collector任一失败会在内存中进入暂停并尝试持久化`suspended`状态，重试为0（fail closed），而不是用指数退避反复重试。`saveBackoffs()`忽略写盘失败，因此重启后的保护取决于暂停记录是否成功落盘；已持久化暂停会从启动、定时、唤醒、Settings及普通手动刷新过滤collector。
- `RefreshCoordinator.inFlight`合并活跃重叠刷新，AppModel `isRefreshing`也抑制并发UI调用；启动先检查账户、唤醒可独立触发刷新，因此源码上仍有两个串行、紧邻采集的竞态窗口。本轮没有增加debounce。
- `saveBackoffs()`忽略写盘失败。此时暂停只在当前进程内生效；重启后若磁盘上没有暂停记录，不能保证collector仍被过滤。这是另一个未关闭的持久化安全缺口，本轮保留为限制，不声称跨重启绝对封锁。

## 独立审查与范围修正

候选6b014a7的第一轮审查聚焦`RefreshCoordinator`/`AppModel`的一次性授权、提交与UI同步，没有沿`AppModel.start()`/唤醒/定时器一路追到`GeminiCollector.collect()`，所以漏掉了解锁后的生产采集路径和补充命令。协调补查后，独立复审指出：自动刷新会继续运行`/model`与非headless `models`、暂停持久化写失败可能跨重启重新执行CLI，且源码未能强制禁止CLI打开浏览器。当前已撤回清除暂停行为，并更正提示/测试。复审修正后的独立结论待附在[审查记录](2026-10-09-antigravity-safe-refresh-review.md)。

## 验证边界

- 本轮红灯测试按预期捕获单次成功后暂停消失、重启继续调用collector、UI解除paused的旧行为。
- 纠正后定向Swift测试：`RefreshCoordinatorTests` 20项通过；`AppModelGeminiRecoveryTests` 6项通过。
- 完整`AI_METER_TEST_BUILD_DIR=/private/tmp/ai-meter-safe-refresh-full-test scripts/test.sh`通过：Swift Core 286项、App 322项、Widget 12项及刷新调度3项，共641项通过；PTY runner 18项通过；跨平台合同/fixture、Windows Release asset normalization fixture、更新源探测、公开发布安全检查及351份Markdown文档检查通过。该脚本包含跳过的Settings localization coverage、部分Hosted/UI交互、已安装Claude/Codex CLI集成等测试；Windows相关结果仅为共享合同和fixture，不是原生Windows GUI、Rust生产构建、签名安装器或真机验证。`swift build --product AIMeterApp -c release --scratch-path /private/tmp/ai-meter-safe-refresh-release-build`通过；`scripts/check-docs.sh`通过（351 Markdown文件）。宿主是macOS，不能冒称原生Windows GUI、Windows Rust生产构建、Windows签名安装器或Windows真机/凭据验收。
- 未执行真实`agy`或读取其认证状态；未访问Google账号、Keychain；未打开Terminal、浏览器、URL opener；未在M4 Max现场运行；未进行Windows真机测试；未推送、合并、签名或发布。

## Git与维护状态

纠正已提交在本地分支`codex/antigravity-safe-auto-refresh`。周期自动恢复仍待具备CLI版本对应的no-browser官方契约/可信隔离边界、补充命令处置、暂停写入失败策略及启动/唤醒全路径证据后重新设计；在此之前继续保持持久暂停。
