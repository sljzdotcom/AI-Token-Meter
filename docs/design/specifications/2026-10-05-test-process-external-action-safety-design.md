# 测试进程外部操作与登录启动隔离规格

**需求：** REQ-20261001-003
**基线：** v0.10.5 / `98dd5c4`（含候选 `71e62d2`）
**状态：** 已授权实现；仅本地修复、验证、审查与Git检查点，不发布

## 问题与证据

发布测试日志中的两个 `GeminiAvailabilityTests` 用例调用 `AppModel.beginSignIn(.gemini)`，却没有注入独立的 `geminiAuthenticationOpenOperation`。`AppModel` 的默认闭包调用 `CLIAuthenticationLauncher`，后者默认把生成的登录脚本交给 `NSWorkspace.open`。这条调用链能解释测试运行期间出现的两条新登录会话；不能证明更早会话的全部来源，也不能证明是否产生账号侧影响。

其他登录测试大多注入了假操作，但这依赖每个调用点都记得注入。CLI安装、安装指南、Claude workspace setup 和品牌链接也存在默认系统打开入口，需要核对并封闭测试进程中的默认路径。

## 目标与边界

- XCTest进程中，未显式注入的外部打开/CLI登录操作默认拒绝，且拒绝发生在写脚本或调用系统打开之前。
- 测试构造的认证脚本和安装脚本只能写入临时目录；真实 `Application Support/AI Meter/Authentication` 不作为测试路径。
- 通过显式注入的假操作进行单元测试仍可记录被请求的URL，不调用真实系统服务。
- 修复遗漏专用注入的两个Gemini测试；审查Claude、Codex、Gemini及安装/浏览器打开路径的测试边界。
- 正常生产进程中，用户显式点击登录、安装或外部链接仍使用原有行为。
- 不运行 `agy`、浏览器OAuth、生成或执行真实认证脚本，不退出/启动已安装App，不读取或修改凭据，不清理用户终端。
- 本次不推送、不创建PR/标签/Release、不修改更新源；已公开的0.10.5保持原状。

## 设计

新增一个小型 `SystemActionPolicy`，根据进程环境与测试Bundle标记识别XCTest进程。策略提供纯函数用于判断默认外部打开是否允许，以及为默认脚本目录选择位置。测试进程的默认打开返回拒绝，默认脚本目录落在唯一临时子目录；普通生产进程继续使用Application Support和`NSWorkspace.open`。

`AppModel`的默认Claude/Codex/Gemini认证与安装/指南操作先经过当前进程策略。低层认证与安装launcher也各自执行拒绝检查，避免未来绕过AppModel的直接调用。所有显式注入的假打开闭包保留其模拟能力。直接使用`NSWorkspace.open`的Claude workspace setup与品牌链接改为可注入，并对默认系统打开应用同一策略。

## 验收

- 合成XCTest环境下，策略判定拒绝默认系统操作；由纯闭包哨兵证明系统打开闭包调用次数为0。
- 同一环境下默认认证/安装脚本目录位于临时目录；拒绝发生时真实launcher不写脚本，也不调用系统打开。
- AppModel在Gemini专用闭包漏注入时，不会调用注入的真实/模拟launcher；Claude与Codex默认路径同样 fail-closed。
- 两个原Gemini测试显式注入专用模拟闭包，并断言预期不发起登录。
- 显式注入的假 `openURL` 在launcher单测中仍被调用，脚本内容/权限合同保持。
- 全面盘点 `NSWorkspace.shared.open` 后，不存在未受测试进程策略保护的生产默认打开入口。
- XCTest、进程组和构建验证期间不出现真实Terminal、浏览器、`agy` 或认证目录副作用；验证不得依赖清理用户会话。
- 生产显式登录实现不更改，且M4 Max现场额度、超时根因和全部历史窗口来源仍单独未验证。
