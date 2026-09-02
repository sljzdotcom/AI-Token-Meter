# 2026-09-02 OpenAI Codex nvm 发现修复与 0.1.2 分发包

- **需求：** `REQ-20260902-016`
- **登记提交：** `7a5f166`
- **核心修复提交：** `05f1c9d`
- **版本准备提交：** `8fd9233`
- **产品版本：** `0.1.2`（build `3`）
- **目标：** Apple Silicon `arm64`、macOS 14.0+

## 问题与证据

`0.1.1` 在 MacBook Pro M4 Max 上可以正常显示 Claude Code 与 DeepSeek，但 OpenAI Codex 的 Services 和详情页同时显示 `CLI not installed` / `Not installed`。M4 Max 本机复核确认：

- CLI 已安装且终端可用，版本为 `0.148.0`；
- 安装位置是 `~/.nvm/versions/node/v25.2.0/bin/codex`；
- `/opt/homebrew/bin/codex` 和 `/usr/local/bin/codex` 均不存在；
- Codex 是 `#!/usr/bin/env node` 脚本；
- Finder/launchctl 环境不加载 `.zshrc`，模拟后得到 `env: node: No such file or directory`；
- ChatGPT App 内置的原生 Codex 可在无用户 PATH 环境中独立运行。

因此问题不是用户未安装，而是旧版定位器没有枚举 nvm，并且即使只找到脚本，也必须让同版本 Node 对子进程可见。

## 实现与关键决定

- `ExecutableLocator` 保留用户显式 PATH 和 `~/.local/bin` 的最高优先级；
- 自动枚举 `~/.nvm/versions/node/*/bin`，使用数字感知版本排序，不写死 `v25.2.0`；同时覆盖 nvm current、npm-global、Volta、asdf 和 mise 常见目录；
- 以用户/系统 Applications 下的 `ChatGPT.app` 和 `Codex.app` 内置 `codex` 作为后备；
- 所有候选必须通过 macOS 可执行权限检查；
- `CodexAppServerClient` 把所选 `codex` 同目录置于子进程 PATH 首位，使 `/usr/bin/env node` 命中相邻 Node；登录脚本使用同样规则；
- 确实找不到 CLI 时，Services 显示 **Open Install Guide**，只打开 OpenAI 官方说明，不静默安装软件或修改 Shell 配置。

## 测试驱动证据

新增测试先分别因缺少 app-bundle 候选、nvm 枚举、Node 运行 PATH 和安装指南模型动作而失败；实现后定向回归和全量回归通过。覆盖：

- 显式 CLI 优先于 App 内置候选；
- GUI PATH 为空时仍能采用 App 内置 Codex；
- 不可执行资源不会误报已安装；
- nvm `v25.2.0` 数字顺序优先于 `v9.8.0`；
- 在刻意移除 Node 目录的 PATH 下，`#!/usr/bin/env` 测试 CLI 仍能启动 app-server；
- 未安装状态能打开官方指南并显示反馈。

## 验收结果

- 全量自动化：**318 个测试、64 个测试组、0 失败**；
- 文档门禁：109 份 Markdown 无断链、版本或测试基线错误；
- Release：`AI_METER_INCLUDE_WIDGET=0` 构建通过，资源门禁与 `codesign --verify --deep --strict` 通过；
- 分发包为 macOS 14+、Apple Silicon arm64，ZIP 清单以 `AI Token Meter.app` 为根，无 `__MACOSX`；
- ZIP 解压至独立目录 `/private/tmp/ai-token-meter-v012.ZuxzoK` 后，在仅含系统目录的 Finder 风格 PATH 下启动并持续存活 5 秒，启动日志为空；
- 正常本机权限下，以同样最小 PATH 调用真实 OpenAI Codex 冒烟测试，官方额度与本机聚合快照在 0.986 秒内成功返回；
- M4 Max 真实 nvm 安装验证：待用户安装 `0.1.2` 后确认，不在交付前冒充完成。

## 0.1.2 交付物

- ZIP：`dist/AI-Token-Meter-0.1.2-macOS-arm64.zip`
- 校验文件：`dist/AI-Token-Meter-0.1.2-macOS-arm64.zip.sha256`
- ZIP 大小：2,347,172 bytes
- SHA-256：`a2c76017e65b088f42d509424eef72044784e6195af94a1b9d086b13eb4d143f`

分发包不含 Widget，采用 ad-hoc 签名，适合本人 Mac 间迁移；首次打开可能仍需通过 Finder 右键“打开”或系统隐私与安全确认。安装前必须退出旧版并完整替换 `.app`，不能只覆盖内部可执行文件。

## 安全与隐私

修复只枚举预定义本机路径并检查可执行权限，不读取 `.zshrc`、CLI 凭证、npm 配置或账户原始响应，不运行 Shell 初始化脚本。安装入口只打开 OpenAI 官方 HTTPS 页面；应用不会自动安装、升级或重签 OpenAI Codex。
