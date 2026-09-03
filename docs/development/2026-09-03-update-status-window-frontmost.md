# Sparkle 安装状态窗口置前与 v0.2.2 真实升级验收

**关联需求：** `REQ-20260903-002`、`REQ-20260902-019`

**日期：** 2026-09-03
**版本：** `0.2.2`（build `6`）
**结果：** 已修复、公开发布并通过隔离真实升级验收

## 背景与目标

从 Settings → About 启动 Sparkle 更新时，下载和 EdDSA 验证实际已经完成，但 SwiftUI Settings 仍位于窗口栈前方，遮住了 Sparkle 的 `Ready to Install` 窗口。用户只能从 Window 菜单手动找回安装窗口，界面因此看起来长期停在 Preparing。

修复目标是让 Sparkle 标准交互自动处于前台，同时保持手动检查、双确认、EdDSA 信任链、错误回调、原位替换和自动重启不变。

## 影响范围

- 新增 `SoftwareUpdateWindowPresenter`，负责在展示标准更新交互前让 Settings 临时让位；
- `SparkleUpdateEngine.presentAvailableUpdate()` 在调用 Sparkle 标准 UI 前执行 presenter；
- 只匹配 SwiftUI Settings 的窗口标识 `com_apple_SwiftUI_Settings_window`，不隐藏普通窗口或详情面板；
- 不改变更新源、下载地址、签名验证、安装权限或状态机语义。

## 根因与实现决定

1. 真实 `0.2.0 → 0.2.1` 安装证明 ZIP 下载、EdDSA 校验、替换和重启链路均正常；阻断点是 `Ready to Install` 窗口被 Settings 遮挡，而不是网络或签名失败。
2. presenter 通过最小协议读取窗口集合，便于在无窗口服务器的测试环境中验证选择逻辑。
3. 仅对精确标识的 Settings 调用 `orderOut(nil)`，随后调用一次 `NSApp.activate(ignoringOtherApps: true)`。
4. Sparkle 仍负责全部下载、验证、用户确认、安装和重启；应用不实现自定义下载器或静默替换。

## 测试驱动证据

- 先添加 presenter 缺失时失败的测试，再实现最小窗口选择与激活逻辑；
- 测试确认 Settings 被隐藏、无标识窗口和普通窗口保持可见、应用只激活一次；
- 更新引擎注入测试锁定 presenter 在 Sparkle 标准 UI 之前调用；
- 完整本机回归：362 项测试、71 个测试组，其中 11 项 PTY 系统资源测试独立执行；
- 公开 PR CI、PTY 稳定性 PR CI 和最终 `main` CI 均通过；最终 CI 为 [33702415007](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/33702415007)。

## 发布资产验证

- GitHub Release：[v0.2.2](https://github.com/sljzdotcom/AI-Token-Meter/releases/tag/v0.2.2)；
- ZIP：`AI-Token-Meter-0.2.2-macOS-arm64.zip`，3,469,343 字节；
- ZIP SHA-256：`9d7c38b2f9420ac7814bfb15493d1391215f5bfaf95689e4e9245af152f241d9`；
- 包内版本：`0.2.2 (6)`；
- 重新下载资产的 SHA-256、版本、build、严格深层签名与 appcast enclosure 均和本地正式资产一致；
- Sparkle EdDSA 验证通过，追加字节的篡改副本按预期被拒绝。

## 隔离真实 GUI 验收

验收使用 `/private/tmp` 中从正式 `v0.2.1` ZIP 解压的副本，未替换 `/Applications/AI Token Meter.app`。

1. 启动隔离 `0.2.1 (5)` 并保持 Settings → About 打开；
2. 点击 `Check for Updates`，About 从 Checking 切换为 `Version 0.2.2 is available`，`Update Now` 正确启用；
3. 点击 `Update Now` 后，Settings 自动隐藏，Sparkle `Software Update` 窗口立即成为焦点；
4. 点击 `Install Update`，下载窗口显示进度；下载完成后 `Ready to Install` 与 `Install and Relaunch` 自动位于最前方，不需要 Window 菜单或手工置前；
5. 点击 `Install and Relaunch` 后，隔离副本完成原位替换并自动重新启动；
6. 落盘 Info.plist 为 `0.2.2 (6)`，严格深层签名验证通过；
7. 升级后主二进制 SHA-256 为 `120cf2cac468f624a5e7f1e5503c8acbabd3202fd79630077963873aee65aac1`，与公开 Release 解压后二进制完全一致。

## 安全与隐私

- 没有读取、记录或传输 Claude Code、OpenAI Codex、DeepSeek 凭据；
- 生产 EdDSA 私钥仍只在维护者登录钥匙串中，仓库、日志、appcast 和 Release 只包含公开验证信息；
- 真实 GUI 验收只操作隔离副本，正式安装和用户数据未被覆盖；
- 更新仍必须由用户依次触发检查、更新和最终安装，不新增后台检查或静默安装。

## Git 节点

- `4092d1f`：记录窗口置前设计；
- `6057b3e`：实现并测试 Settings 让位与应用激活；
- `2317aa9`、`4460dab`：准备并固化 v0.2.2 发布元数据；
- `f14f14a`：移除详情超时测试中的固定墙钟假设；
- `c11da0a`：移除 PTY fixture 的额外进程扇出，并作为 `v0.2.2` 标签目标；
- [最终 main CI 33702415007](https://github.com/sljzdotcom/AI-Token-Meter/actions/runs/33702415007)：完整公开验证通过。

## 已知限制

当前公开包为 Apple Silicon、ad-hoc 签名且未公证；首次在另一台 Mac 打开时仍可能需要 Finder 右键“打开”。这不影响 Sparkle 在已经信任并运行的 `0.2.0+` 安装之间执行 EdDSA 验证更新。
