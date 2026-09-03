# Sparkle 安装状态窗口置前设计

**需求：** `REQ-20260903-002`  
**日期：** 2026-09-03  
**状态：** 已确认（依据用户对非视觉事项采用推荐方案、无需逐次复核的授权）

## 问题与证据

从公开 `v0.2.0` 隔离副本检查并安装 `v0.2.1` 时，Sparkle 正确完成下载、解压和 EdDSA 验证，但下载完成后的 `Ready to Install` 状态窗口位于 Settings 后方。Settings 因此持续显示 `Preparing version 0.2.1…`，用户无法知道还需要点击 `Install and Relaunch`。

从 Window 菜单手动置前 `Updating AI Token Meter` 后，原位替换、自动重启、`0.2.1 (5)` 版本、严格签名和主二进制哈希全部通过。Sparkle 的代码签名差异日志不是阻断：官方验证器允许旧公钥验证通过的 EdDSA 更新在 Apple 代码签名身份不同的情况下继续安装。

## 目标

- 用户点击 About 中的 `Update Now` 后，Settings 不得遮挡 Sparkle 的标准交互窗口。
- 保留 Sparkle 的标准 `Install Update` 与 `Install and Relaunch` 确认，不做静默安装。
- 不降低 HTTPS、EdDSA、公钥连续性、归档长度、版本和代码签名完整性检查。
- 取消或失败后仍可重新打开 Settings 并再次检查。

## 方案比较

### A. 安装交互开始前隐藏 Settings（采用）

在调用 Sparkle 标准更新 UI 前，只隐藏 SwiftUI Settings 场景窗口，并激活 AI Token Meter。Sparkle 的更新提示和后续状态窗口因此成为当前应用唯一的普通前台窗口。

优点：使用公开 AppKit/Sparkle API；不依赖 Sparkle 私有窗口类、标题或标识；变化小且可测试。代价是取消安装后用户需要从菜单重新打开 Settings。

### B. 持续查找并置前 `SUStatus`

保留 Settings，下载完成后查找 Sparkle 状态窗口并 `orderFront`。这依赖 Sparkle 内部窗口标识和事件时序，升级框架后更容易失效，因此不采用。

### C. 自定义 Sparkle User Driver

完全接管更新 UI，可精确控制层级，但需要复制下载、错误和安装交互，扩大安全与维护范围，不符合当前缺陷的最小修复原则。

## 架构与行为

新增 `SoftwareUpdateWindowPresenter`，只负责：

1. 从传入窗口集合中识别 `com_apple_SwiftUI_Settings_window`；
2. 对这些 Settings 窗口执行 `orderOut`；
3. 激活当前应用；
4. 不触碰浮动条、详情、Sparkle 窗口或其他应用窗口。

`SparkleUpdateEngine.presentAvailableUpdate()` 先调用 presenter，再调用现有 `SPUStandardUpdaterController.checkForUpdates(_:)`。状态机、网络策略和 Sparkle 委托保持不变。

## 测试与验收

- 单元测试验证只选择并隐藏 Settings，保留无标识和非 Settings 窗口，并调用一次激活。
- 源码契约测试验证更新引擎在展示 Sparkle UI 前调用 presenter。
- 完整 Swift、文档和公开安全门禁通过。
- 发布 `v0.2.2`，不改写已公开的 `v0.2.1` 标签或资产。
- 隔离 `0.2.1` 从正式 GitHub appcast 发现 `0.2.2`；在 Settings 打开状态下完成标准双确认、原位替换、自动重启、版本、签名和哈希终验。

## 非目标

- 不自动点击 Sparkle 的安装确认；
- 不增加后台检查或静默更新；
- 不引入 Developer ID、公证或 Widget 签名变更；
- 不修改更新公钥或导出私钥。
