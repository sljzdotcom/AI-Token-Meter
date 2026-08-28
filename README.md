# AI Meter

AI Meter 是一个完全在 Mac 本机运行的原生菜单栏 App，用一个界面查看 Claude、Codex 和 DeepSeek 的使用状态。它提供右侧悬浮用量环、菜单栏详情、5 分钟自动刷新、本地缓存，以及 70% / 90% 两级提醒。

## 当前能力

- Claude：通过已安装并登录的 Claude Code CLI 读取当前会话和全局额度。
- Codex：通过 Codex CLI 官方 `app-server` 接口读取主、次额度窗口。
- DeepSeek：调用官方余额接口；API Key 只保存在本机 Keychain。
- 右侧悬浮条：默认开启，可点击任一圆环展开详情，也可在设置中关闭。
- 弱网与故障降级：刷新失败时保留最近一次成功缓存，并显示可行动状态。
- 本地预算：根据 AI Meter 观察到的 DeepSeek 余额减少量累计，充值不会被记为负消费。

## 系统要求

- Apple Silicon Mac（M1 或更新）与 macOS 14 或更新版本。
- Xcode Command Line Tools（从源码构建时需要）。
- Claude Code CLI 与 Codex CLI 已安装；需要查看哪项服务，就先在相应 CLI 中完成正常登录。
- DeepSeek 为可选项；未配置 API Key 时只显示“需要配置”，不会发起余额请求。

## 构建与运行

在项目目录执行：

```bash
bash scripts/build-app.sh
open "dist/AI Meter.app"
```

脚本会执行 release 构建，生成 `dist/AI Meter.app`，进行本机临时签名并验证签名。若希望日常使用，可退出 AI Meter 后把 App 拖入“应用程序”文件夹，再从那里启动。

当前脚本面向这台 Apple Silicon Mac 的本机使用，产物为 arm64 且采用 ad-hoc 临时签名，不是面向互联网公开分发的 Developer ID 版本。若以后需要发给其他用户，应增加 universal 构建（如需 Intel 支持）、Hardened Runtime、Developer ID Application 签名和 Apple 公证。

## 首次配置

1. 启动后，屏幕右侧会出现三个圆环，菜单栏也会出现 AI Meter。
2. 在菜单栏面板点击齿轮，或按 `⌘,` 打开设置。
3. 如需 DeepSeek 余额，在设置中填写 API Key 并保存。输入值写入 macOS Keychain，不进入偏好、缓存或日志。
4. 按需开启 70% / 90% 提醒；macOS 首次会请求通知权限。
5. 按需开启“登录时启动”。本机临时签名版本若移动路径，建议先关闭该开关，移动后重新开启。

## 指标说明

- Claude/Codex 的百分比来自各自 CLI 当前账户返回的额度窗口。
- DeepSeek 的 `Available balance` 是官方账户余额，不是使用百分比。
- DeepSeek 的 `Local monthly budget` 是本机估算值：仅在 AI Meter 成功刷新时比较余额差异。因此 App 未运行期间发生的充值和消费无法被精确拆分，它不等同于官方账单。
- 菜单栏百分比取所有有上限指标中的最高使用率；余额本身不会参与阈值提醒。

## 隐私与安全

- 不接管浏览器 Cookie，不读取 Claude/Codex 凭证文件，也不模拟网页登录。
- Claude 与 Codex 只使用其已登录 CLI 的本机接口。
- DeepSeek API Key 使用 `AfterFirstUnlockThisDeviceOnly` 级别存入 Keychain，不随 iCloud Keychain 同步。
- 缓存仅保存统一用量快照；保存和展示前会再次清除 Bearer Token 与常见 API Key 形态。
- App 不上传分析数据，也不会把原始 CLI 输出、HTTP 错误正文或授权头写入通知。

## 故障排查

### Claude 显示需要登录或未安装

先在终端确认 Claude Code CLI 可以启动，并在官方 CLI 中完成登录。AI Meter 不会替你打开交互式登录，也不会保存 Claude 凭证。

### Codex 显示需要登录、不可用或格式变化

先直接启动 Codex CLI 并完成登录，再回到 AI Meter 手动刷新。如果安装了很旧的 Codex CLI，请先升级；AI Meter 使用结构化 `app-server` 用量接口，而不是解析全屏界面。

### DeepSeek 没有余额

打开设置确认 API Key 已显示“Stored securely in Keychain”。401 会显示需要重新配置，429 会提示稍后再试。API Key 为空时不会联网。

### 悬浮条不见了

菜单栏功能始终保留。打开 AI Meter 设置，重新开启“Show right-side floating meter”。多显示器变化后面板会自动重新定位到当前可见屏幕右侧。

### 查看诊断信息

- 开发过程与每个 Git 节点的验证记录：`docs/development/2026-08-28-development-log.md`。
- macOS 崩溃报告：`~/Library/Logs/DiagnosticReports/AIMeterApp-*.ips`。
- 实时系统诊断：打开“控制台”App，以进程名 `AIMeterApp` 筛选。AI Meter 的设计不会记录凭证或原始账户响应。

## 卸载

1. 从菜单栏面板退出 AI Meter。
2. 若启用了登录启动，建议先在设置中关闭。
3. 删除 `AI Meter.app`。
4. 如需同时删除 DeepSeek Key，可在卸载前从设置中点击 Remove；其他偏好和非敏感缓存可按需保留。

## 开发验证

```bash
swift test
swift build -c release
bash scripts/build-app.sh
codesign --verify --deep --strict "dist/AI Meter.app"
```

部分真实 CLI 与 Keychain 集成测试默认跳过，避免普通测试读取本机账户状态。它们只在显式设置相应测试环境变量时运行。
