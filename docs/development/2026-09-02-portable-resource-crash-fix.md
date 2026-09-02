# 2026-09-02 跨 Mac 资源装载崩溃修复与 0.1.1 分发包

- **需求：** `REQ-20260902-015`
- **登记提交：** `759e157`
- **核心修复提交：** `5c72aff`
- **产品版本：** `0.1.1`（build `2`）
- **目标：** Apple Silicon `arm64`、macOS 14.0+

## 问题与影响

首个 `0.1.0` MacBook 分发包解压后会在另一台 Mac 启动时立即崩溃。用户提供的诊断指向 `NSBundle.module → FloatingStripView.body`，且包内 `AI-Meter_AIMeterApp.bundle` 缺少自己的 `Info.plist`。由于应用设置了 `LSUIElement = true`，正常情况下只显示菜单栏和桌面浮岛；崩溃发生在界面建立前，因此用户看到的是“完全没有响应”。

`0.1.0` ZIP 已判定为不可迁移交付物，不应继续安装；对应文件已从 `dist/` 移除，历史构建事实保留在原日志中。

## 根因证据

SwiftPM 为 `Bundle.module` 生成的访问代码包含两个候选位置：

1. `Bundle.main.bundleURL/AI-Meter_AIMeterApp.bundle`；
2. 构建机临时目录中的绝对路径，例如 `/var/folders/.../com.millerpan.AIMeter-build/.../AI-Meter_AIMeterApp.bundle`。

旧打包脚本却把资源包复制到第三个位置：`AI Token Meter.app/Contents/Resources/AI-Meter_AIMeterApp.bundle`。该目录又缺少可供 `Bundle(path:)` 初始化的 `Info.plist`。在构建机上，第二个绝对路径仍存在，因而掩盖了发布缺陷；复制到 MacBook 后，该绝对路径不存在，`Bundle.module` 触发致命错误。

旧 ZIP 清单、生成的 `resource_bundle_accessor.swift` 和包内资源结构共同证明了这一点。此前的 ZIP 完整性与签名检查只验证文件未损坏，不能证明运行时资源解析成功。

## 测试驱动修复

先添加两组真实行为测试，并观察到失败：

- `AppResourceLocatorTests` 在生产资源定位器尚不存在时编译失败；它要求发布版只从自己的主 App Bundle 读取资源，资源缺失时也不得调用 SwiftPM 的构建机回退路径。
- `PortableAppBundleVerifierTests` 在发布资源验证器尚不存在时失败；它要求标准主 Bundle 资源布局通过、旧嵌套 SwiftPM 布局失败。

随后完成最小修复：

- 新增 `AppResourceLocator`，开发/测试环境仍可按需使用 SwiftPM 资源，但已打包 `.app` 绝不再回退到构建机路径；资源缺失时安全使用现有 Logo 或玻璃背景降级。
- `ProviderLogo` 与 `FloatingStripBackgroundAsset` 统一通过该定位器加载。
- `build-app.sh` 将 SwiftPM 构建出的资源内容直接复制到标准 `Contents/Resources/Logos` 与 `Contents/Resources/Backgrounds`，不再把无效 `.bundle` 嵌入 App。
- 新增 `scripts/verify-app-resources.sh`，每次 Release 构建都强制检查主 `Info.plist`、三个 Logo 和深海背景；任何资源缺失都会使构建失败。
- 主应用、Widget 元数据和 Codex app-server clientInfo 同步升级为 `0.1.1`（build `2`）。

## 自动化验证

- 全量测试：**312 个测试、63 个测试组、0 失败**；真实账户相关门控测试按环境设计跳过。
- 文档一致性检查：新增本日志后 **106 份 Markdown** 通过。
- Release 构建：`AI_METER_INCLUDE_WIDGET=0 scripts/build-app.sh` 通过。
- App 资源门禁：`scripts/verify-app-resources.sh` 通过。
- App 签名：`codesign --verify --deep --strict --verbose=2` 通过。
- 可执行文件：Mach-O 64-bit `arm64`；最低系统版本 macOS 14.0。
- ZIP：`unzip -t` 全部通过，且不再包含 `__MACOSX` 元数据目录。

## 脱离构建机路径的启动验收

最终 ZIP 被解压到独立目录 `/private/tmp/ai-token-meter-portable.goMAwU`。验收时临时隐藏 SwiftPM 构建资源目录，再通过 Launch Services 启动解压出的 App：进程在 5 秒观察期内持续存活，随后由测试主动退出。该步骤直接覆盖了旧包在另一台 Mac 上失败的条件。

解压出的 App 同时满足：

- 包内不存在 `AI-Meter_AIMeterApp.bundle`；
- 四项运行时资源均位于主 Bundle 标准资源目录；
- 严格签名验证通过；
- 无新增启动崩溃报告。

## 0.1.1 交付物

- ZIP：`dist/AI-Token-Meter-0.1.1-macOS-arm64.zip`
- 校验文件：`dist/AI-Token-Meter-0.1.1-macOS-arm64.zip.sha256`
- ZIP 大小：约 2.2 MiB
- SHA-256：`1b2cf19bcbdb8cbaa866aeb850df14ca30fc3f6cb135806ab562976a8f09fa72`

`dist/` 是受 `.gitignore` 排除的可再生交付目录；Git 保存源码、构建方法、测试和本日志，不保存二进制 ZIP。

## 安装与签名边界

本包不含 Widget，使用 ad-hoc 签名，没有 Developer ID、Team ID 或 Apple 公证。它适合本人在 M4 Max MacBook Pro 上安装，但 Gatekeeper 仍可能提示开发者无法验证；应在 Finder 中右键应用选择“打开”，或在“系统设置 → 隐私与安全性”中确认“仍要打开”。这类提示与本次资源崩溃不同。

另一台 Mac 上的 Claude Code/OpenAI Codex 登录、DeepSeek Keychain、WebKit 会话和显示偏好不会随应用包迁移，需要在新机器独立配置。Widget 的证书与真实桌面验收继续由 `REQ-20260901-003` 管理。

## 安全与隐私

打包、测试、日志和校验文件均不包含 API Key、OAuth Token、Cookie、CLI 凭证或账户原始响应。修复只改变静态图片资源的定位与分发结构，不改变服务采集和凭证边界。
