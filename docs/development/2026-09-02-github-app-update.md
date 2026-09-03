# GitHub 应用内更新开发与验收记录

**需求：** `REQ-20260902-019`

**版本：** `0.2.0`（build `4`）

**日期：** 2026-09-02
**状态：** 已完成并公开发布；v0.2.2 真实公开升级终验通过

## 交付范围

- Settings → About 增加 `Check for Updates` 与 `Update Now`。
- 只有用户点击检查按钮时才联网；不做后台检查或静默安装。
- 使用 Sparkle `2.9.4`、固定 GitHub appcast 与 EdDSA 签名验证。
- 使用 Sparkle 标准更新窗口完成下载、验证、替换和重新启动。
- 发布入口统一生成 arm64 ZIP、SHA-256 和 `appcast.xml`。
- 生产私钥只保存在 macOS 登录钥匙串中；仓库、构建日志和发布资产都不包含私钥。

## 关键实现

### 应用层

- `SoftwareUpdateState` 统一表示未检查、检查中、已是最新版、发现新版、安装中和失败。
- `SoftwareUpdateCoordinator` 串行化用户动作，阻止重复检查和重复安装。
- `SparkleUpdateEngine` 是唯一第三方适配层；界面不直接依赖 Sparkle 类型。
- 错误只映射为固定安全文案，不把请求 URL、底层调试信息或凭证写入 UI。
- AppDelegate 只创建一个更新协调器，并在应用退出时解除回调。

### 构建与发布

- SwiftPM 以官方 Release URL 和固定 SHA-256 锁定 Sparkle `2.9.4`。
- Release App 完整嵌入 `Sparkle.framework`、Updater、Autoupdate 及两个 XPC helper。
- 签名顺序为内部 helper → framework → 主 App；ad-hoc 构建不错误启用 hardened runtime。
- `verify-update-bundle.sh` 检查 framework、helper、`@rpath`、feed、公钥和严格签名。
- `package-update-release.sh` 执行测试、安全扫描、构建、ZIP、SHA-256、签名及 appcast 生成。
- `verify-update-archive.sh` 验证 appcast 与 ZIP 长度、版本、build 和 EdDSA 签名，并确认篡改副本被拒绝。

## 密钥边界

- 生产签名账户：`com.millerpan.AIMeter`。
- App 只包含公开验证键；公开键不是凭证，可以进入 `Info.plist`。
- 没有运行私钥导出命令，没有生成 `.key`、`.pem` 或其他私钥文件。
- `check-public-release.sh` 额外拦截 `.key` 文件和 Sparkle 私钥导出标记，且错误日志不回显命中内容。

## 自动化验证

完整发布流水线于 2026-09-02 执行成功：

- 最终 360 项测试、70 个测试组通过（发布流水线初次完整执行为 358 项；随后新增更新归档和版本自适应文档门禁回归）。
- 118 份 Markdown 文档门禁通过。
- 当前工作树、完整 Git 历史和最终 ZIP 的公开安全扫描通过。
- arm64 Release 构建、便携资源、Sparkle 嵌套组件、`@rpath` 与严格签名通过。
- ZIP：`AI-Token-Meter-0.2.0-macOS-arm64.zip`。
- 最终 ZIP SHA-256：`81002ba6e577e5ce019e94aa9b8c7305d052ecae1193ebe4cb2169436f639ee1`（3,466,635 字节）。
- appcast enclosure 长度：`3,466,635` 字节。
- ZIP 内版本：`0.2.0`，build `4`。
- 正式 ZIP 的 Sparkle EdDSA 验证通过。
- 对临时副本追加 6 个字节后，Sparkle 验证按预期失败；正式 ZIP 未被修改。

## 隔离真实更新验收

验收只使用 `/private/tmp` 中的测试 App 和仅监听 `127.0.0.1` 的临时 HTTP feed，未替换 `/Applications/AI Token Meter.app`。

1. 从同一 Release App 创建 `0.1.9 (3)` 旧版测试 host。
2. 启动后、点击检查前，本地服务器没有收到 appcast 请求，证明没有后台检查。
3. 点击 `Check for Updates` 后，About 显示 `Version 0.2.0 is available`，`Update Now` 从禁用变为可用。
4. Sparkle 标准窗口显示旧版 `0.1.9` 与新版 `0.2.0`，下载正式签名 ZIP。
5. 用户在动作发生前明确授权安装；点击 `Install and Relaunch` 后完成原位替换并自动重启。
6. 重启后 Info.plist 为 `0.2.0 (4)`，feed 恢复正式 GitHub URL，严格签名验证通过。
7. 更新后主二进制 SHA-256 为 `00839f621d1f229a9ecd71b6b42acb3b4a8be6cd1dc358d1a57a9f2bc9bb7ef6`，与正式 Release App 完全一致。
8. 将最新版副本重新指向同一测试 feed，点击检查后显示 `You’re up to date`，`Update Now` 保持禁用。
9. 停止本地服务器后再次检查，界面显示固定安全文案 `You appear to be offline.`，App 保持可用。

本地服务器观察到的请求顺序是 appcast → 用户发起安装后的 appcast → ZIP；没有在应用启动时出现下载请求。

## 公开发布与补丁复验

- `v0.2.0` 作为首个带更新器的公开版本发布；既有 `0.1.2` 用户需要手动替换一次。
- `v0.2.1`（build 5）继续使用同一固定公钥和手动更新策略，修复 PTY 高负载稳定性；GitHub 最终 CI 33655946917 通过。
- 公开下载 ZIP 为 3,467,098 字节，SHA-256 `39334e1ea706e121c30d5cc85720142ff336b0cd696c8eb544336caa7c8724a0`；重新下载后校验一致，包内版本为 `0.2.1 (5)`。
- 隔离 `0.2.0 (4)` 测试副本从正式 GitHub appcast 发现 `0.2.1`，About 正确启用 `Update Now`；最终原位替换与重启结果记录在 v0.2.1 收尾提交中。

## v0.2.2 公开升级终验

- `v0.2.2`（build 6）修复从 Settings 发起更新时 Sparkle 安装状态窗口被遮挡的问题，公开 Release 与最终 CI 均已完成。
- 正式 ZIP 为 3,469,343 字节，SHA-256 `9d7c38b2f9420ac7814bfb15493d1391215f5bfaf95689e4e9245af152f241d9`；公开重新下载、包内 `0.2.2 (6)`、严格深层签名、EdDSA enclosure 和篡改拒绝全部通过。
- 真实验收从 `/private/tmp` 中的隔离 `0.2.1 (5)` 开始，Settings → About 保持打开；应用从正式 GitHub appcast 发现 `0.2.2`。
- 点击 `Update Now` 后 Settings 自动让位；点击 `Install Update` 并下载完成后，`Ready to Install / Install and Relaunch` 自动位于前台，不需要再从 Window 菜单找回。
- 点击 `Install and Relaunch` 后完成原位替换和自动重启；升级后主二进制 SHA-256 `120cf2cac468f624a5e7f1e5503c8acbabd3202fd79630077963873aee65aac1` 与公开 Release 完全一致。
- 完整窗口、发布与验收证据见 [Sparkle 安装状态窗口置前与 v0.2.2 真实升级验收](2026-09-03-update-status-window-frontmost.md)。

## 已处理故障

### ad-hoc hardened runtime 拒绝 Sparkle framework

初次便携启动时，dyld 因主 App 与 framework 都是无 TeamIdentifier 的 ad-hoc 签名，却启用了 library validation 而拒绝加载。构建脚本调整为：真实开发者证书保留 hardened runtime；ad-hoc 分发不添加该选项。修复后跨目录启动与真实自更新均通过。

### 生成 appcast 后 ZIP 发生重建

ZIP 每次重建都会改变字节长度和签名。正式发布入口因此始终在最终构建之后重新生成 appcast，并立即用独立门禁核对长度、版本和签名，避免提交陈旧 enclosure。

## 首次升级边界

公开版本 `0.1.2` 尚未包含更新器，所以从 `0.1.2` 升级到 `0.2.0` 仍需在 GitHub Release 手动下载并替换一次。从 `0.2.0` 开始，后续稳定版本可在 Settings → About 内完成手动检查和安装。
