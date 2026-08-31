# AI Meter

![macOS 14+](https://img.shields.io/badge/macOS-14%2B-111111?logo=apple)
![Swift 6](https://img.shields.io/badge/Swift-6-F05138?logo=swift&logoColor=white)
![Version 0.1.0](https://img.shields.io/badge/version-0.1.0-2ea44f)
![Tests 166](https://img.shields.io/badge/tests-166%20passed-2ea44f)

AI Meter 是一款原生 macOS 菜单栏应用，把 Claude、Codex 和 DeepSeek 的账户使用状态集中到一个轻量的桌面悬浮条中。数据留在本机，常用信息一眼可见，详细额度、重置时间、充值券和近 30 天 API 用量则在点击后展开。

> 项目状态：个人本地工具，当前应用版本为 `0.1.0`（build `1`）。`0.1.0` 之后已经合入但尚未正式发布的改动统一记录在 [Unreleased](CHANGELOG.md#unreleased)。

<p align="center">
  <img src="docs/assets/ai-meter-floating-strip.jpeg" width="108" alt="AI Meter 贴边浮岛，依次显示 Claude、Codex 和 DeepSeek 图标及用量环">
</p>

## 主要功能

- 原生 macOS 菜单栏 App，无 Electron、无常驻浏览器窗口。
- 贴边浮岛只显示三个经过光学校正的品牌 Logo 与用量环；内部使用低亮度黑蓝「深海波纹」背景，左右贴边时背景会随轮廓镜像，但 Logo 和进度方向保持不变。
- Claude、Codex、DeepSeek 分别使用黄橙、玫红紫、薄荷紫强调色；警告、严重、缓存和不可用状态仍使用统一语义色。
- 浮岛会记住显示器、侧边和垂直位置，详情始终朝桌面内部展开。
- 统一的深色玻璃详情页和无文字仪表指针 App Icon，兼顾浅色、深色与高对比度桌面。
- 全局显示字体可在 System Default、Antonio 和 DIN Condensed 之间即时切换；默认仍为 macOS 系统字体，并可一键恢复。
- Claude：读取当前会话与周额度，显示重置时间。
- Codex：读取官方通用速率限制和重置额度，并在详情中补充本机近 30 天 Token、连续使用天数与最长会话。
- DeepSeek：读取账户余额；以可配置余额基准（默认 ¥100）显示已消耗比例。
- DeepSeek 详情页：通过隔离的官方网页会话获取最近 30 天成本、请求数、Token 数和每日成本图表。
- 点击屏幕空白处关闭详情；详情可在 3、5、8、15 或 30 秒后自动收起，悬停、键盘焦点、VoiceOver 与登录操作期间暂停倒计时。
- 每 5 分钟自动刷新，支持手动刷新、离线缓存和 70% / 90% 阈值通知。
- API Key 存入 macOS Keychain；缓存前会清理常见 Token 与密钥形态。

## 数据来源一览

| 服务 | 数据来源 | 主要显示内容 | 首次准备 |
| --- | --- | --- | --- |
| Claude | 已登录的 Claude Code CLI，隔离工作区内执行 `/usage` | 当前会话、周额度、重置时间 | 安装并登录 Claude Code；首次可能需批准 AI Meter 私有工作区 |
| Codex | Codex CLI 官方 `app-server` JSON-RPC + 本机 Codex 状态库的聚合列 | 通用用量窗口、重置额度、近 30 天本机活动 | 安装并登录 Codex CLI |
| DeepSeek | 官方余额 API + App 内隔离的 `platform.deepseek.com` WebKit 会话 | 余额、基准消耗环、近 30 天成本/请求/Token 图表 | 在设置中保存 API Key；历史图表首次需登录官网 |

详细的数据口径、降级行为与限制见 [服务与指标说明](docs/user-guide/providers.md)。

## 系统要求

- Apple Silicon Mac（M1 或更新机型）。
- macOS 14 Sonoma 或更新版本。
- 从源码构建时需要 Xcode Command Line Tools 与 Swift 6 工具链。
- Claude 与 Codex 是可选服务；需要监控哪项服务，就安装并登录对应 CLI。
- DeepSeek 是可选服务；余额需要 API Key，30 天用量图表需要在 App 的隔离网页中登录 DeepSeek 平台。

## 从源码安装

```bash
git clone <repository-url>
cd AI-Meter
bash scripts/build-app.sh
open "dist/AI Meter.app"
```

构建脚本会生成完整尺寸的仪表指针 App Icon、组装 `dist/AI Meter.app`、执行 ad-hoc 本机签名并验证签名。当前产物面向 Apple Silicon 本机使用，不是经过 Developer ID 签名和 Apple 公证的公开发行包。

日常使用时，退出 AI Meter 后把应用移到 `/Applications`，再从“应用程序”启动。如果移动了应用路径，请重新开关一次“登录时启动”。

完整步骤见 [安装与首次使用](docs/user-guide/getting-started.md)。

## 第一次使用

1. 启动 AI Meter。屏幕右侧出现贴边浮岛和三个用量环，菜单栏出现 AI Meter 图标。
2. 点击菜单栏图标，再点击齿轮，或按 `⌘,` 打开设置。
3. 确认 Claude Code 与 Codex CLI 已分别登录；如 Claude 提示工作区设置，点击一次性设置按钮并在终端批准。
4. 如需 DeepSeek，在设置中保存 API Key，并把“Balance baseline”设为希望参考的余额（默认 ¥100）。
5. 点击 DeepSeek 圆环，在详情页登录官方平台以启用近 30 天用量图表。
6. 按需开启 70% / 90% 提醒、登录时启动，选择详情自动隐藏时间、浮岛侧边模式和显示字体。

Antonio 与 DIN Condensed 必须先安装到 macOS 才能选择；AI Meter 不下载、内置或分发字体文件。缺失字体的选项会禁用，已保存字体临时不可用时会安全回退到 System Default。详见[设置参考](docs/user-guide/settings.md#display-font)。

## 如何理解圆环

- **Claude / Codex**：圆环表示官方额度已经使用的比例，越接近一整圈，剩余额度越少。
- **DeepSeek**：圆环表示参考余额已经消耗的比例。基准为 ¥100、余额为 ¥77.99 时，圆环约为 `22.01%`。
- 圆环中的 Logo 只表示服务；百分比、余额、重置时间和明细在点击后的详情中显示。
- 菜单栏汇总采用可用额度指标中的最高使用比例；余额金额本身不直接触发额度提醒。

## 项目结构

```text
AI-Meter/
├── Sources/
│   ├── AIMeterApp/              # SwiftUI App、系统集成与界面
│   └── AIMeterCore/             # 采集、领域模型、缓存、安全与协调逻辑
├── Tests/
│   ├── AIMeterCoreTests/        # 领域、解析器、协调与真实 CLI 冒烟测试
│   └── AIMeterAppTests/         # 浮岛输入、启动和 AppKit/SwiftUI 边界测试
├── docs/
│   ├── user-guide/              # 安装、服务配置、设置和排障
│   ├── architecture/            # 架构与源码目录说明
│   ├── development/             # 开发、测试、发布和逐日开发日志
│   └── design/                  # 历史设计规格与实施计划
├── scripts/                     # 可移植测试、Release 构建、App 打包与签名验证
├── CHANGELOG.md                 # 面向版本的变更记录
├── CONTRIBUTING.md              # 贡献规范
└── SECURITY.md                  # 安全问题报告方式
```

完整目录职责见 [代码库结构](docs/architecture/repository-structure.md)。

## 开发与验证

```bash
bash scripts/test.sh
bash scripts/build-app.sh
codesign --verify --deep --strict "dist/AI Meter.app"
```

普通测试不会主动读取本机账户状态。真实 CLI 冒烟测试必须显式开启对应环境变量，具体方法见 [测试指南](docs/development/testing.md)。

## 文档

- [文档总览](docs/README.md)
- [安装与首次使用](docs/user-guide/getting-started.md)
- [服务与指标说明](docs/user-guide/providers.md)
- [设置参考](docs/user-guide/settings.md)
- [故障排查](docs/user-guide/troubleshooting.md)
- [架构概览](docs/architecture/overview.md)
- [代码库结构](docs/architecture/repository-structure.md)
- [隐私与安全](docs/security-and-privacy.md)
- [开发环境](docs/development/setup.md)
- [测试指南](docs/development/testing.md)
- [发布流程](docs/development/release-process.md)
- [提交历史](docs/development/commit-history.md)
- [开发日志](docs/development/README.md)
- [版本变更](CHANGELOG.md)
- [贡献指南](CONTRIBUTING.md)

## 隐私与安全摘要

- Claude 与 Codex 凭证由各自 CLI 管理，AI Meter 不读取或保存它们的凭证文件。
- Codex 本机活动只读取线程表中的 Token 数与创建/更新时间，不读取标题、预览、提示词或回复。
- DeepSeek API Key 使用 `AfterFirstUnlockThisDeviceOnly` 级别保存在 macOS Keychain，不随 iCloud Keychain 同步。
- DeepSeek 历史用量使用 App 自己的隔离 WebKit 会话，不读取 Safari、Chrome 或其他浏览器 Cookie。
- 历史页面的原始响应不会进入业务缓存；AI Meter 只保存标准化后的逐日成本、请求数和 Token 总数。
- 缓存、状态消息与通知会先经过敏感文本清理。

完整说明见 [隐私与安全](docs/security-and-privacy.md)。发现安全问题请按 [SECURITY.md](SECURITY.md) 私下报告。

## 版本与许可

- 当前应用版本：`0.1.0`（build `1`）。
- 完整变更：见 [CHANGELOG.md](CHANGELOG.md)。
- Git 关键节点：见 [提交历史](docs/development/commit-history.md)。
- 本仓库目前尚未声明开源许可证。源代码可见不等于获得复制、修改或再分发授权；在许可证确定前请勿擅自分发。
