# 2026-09-02 MacBook Pro M4 Max 分发包

> **已被替代：** 本页记录的 `0.1.0` ZIP 存在跨 Mac SwiftPM 资源装载崩溃，文件已从 `dist/` 移除，请使用[修复后的 0.1.1 分发包](2026-09-02-portable-resource-crash-fix.md)。本页只作为失败证据和发布流程改进历史保留。

- **需求：** `REQ-20260902-014`
- **源码基线：** `main` 的 `7a93c66`
- **产品版本：** `0.1.0`（build `1`）
- **目标：** Apple Silicon `arm64`、macOS 14.0+

## 交付物

- App：`dist/AI Token Meter.app`
- ZIP：`dist/AI-Token-Meter-0.1.0-macOS-arm64.zip`
- 校验和：`dist/AI-Token-Meter-0.1.0-macOS-arm64.zip.sha256`
- ZIP 大小：2,350,115 bytes（约 2.24 MiB）
- ZIP SHA-256：`262f13f9f31ed31e43cfa2e07b673bc4b356e18a64a26e110b930d9586d91783`

`dist/` 是可再生交付目录，受 `.gitignore` 排除；源码、构建方法和验证证据由 Git 保存，二进制包不写入仓库历史。

## 构建与验证

- `scripts/test.sh`：308 项测试、61 个测试组、0 失败；2 个需要真实 Keychain/已安装 CLI 的门控测试按设计跳过；
- 文档检查：104 份 Markdown 通过；
- `AI_METER_INCLUDE_WIDGET=0 scripts/build-app.sh`：Release 构建成功；
- Bundle：`AI Token Meter`、版本 `0.1.0`、最低 macOS `14.0`；
- 可执行文件：Mach-O 64-bit `arm64`，适用于 M4 Max；
- 源 App 和 ZIP 解压后的 App 均通过 `codesign --verify --deep --strict --verbose=2`；
- `unzip -t` 检查所有压缩条目，无损坏。

## 签名边界

当前包使用 ad-hoc 签名，Bundle ID 为 `com.millerpan.AIMeter`，没有 Team ID、Developer ID 或 Apple 公证。它适合个人在另一台 Mac 上使用，但不等于面向公众的已公证发行版。Widget 未包含，继续由 `REQ-20260901-003` 管理。

## MacBook 安装

1. 把 ZIP 拷贝或通过 Dropbox 同步到 MacBook Pro；
2. 解压后把 `AI Token Meter.app` 拖入“应用程序”；
3. 首次启动若被 Gatekeeper 拦截，在 Finder 中右键 App，选择“打开”，再确认一次；
4. 在 MacBook 上分别完成 Claude Code、OpenAI Codex 的 CLI 登录，并在 Settings 中录入 DeepSeek API Key；凭证不会随 ZIP 或旧 Mac 自动迁移；
5. 如果系统仍因隔离属性拒绝启动，先核对 SHA-256，确认文件与本日志一致后再执行：`xattr -dr com.apple.quarantine "/Applications/AI Token Meter.app"`。

## 已知限制

- 不包含 Widget；
- 未公证，因此双击首次启动体验不如 Developer ID 正式包；
- MacBook 上的 CLI 登录、Keychain、缓存、WebKit 登录和显示偏好都是独立环境，需要重新配置；
- 不支持 Intel Mac。
