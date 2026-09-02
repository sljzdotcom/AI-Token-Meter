# OpenAI Codex 跨 Mac 发现修复设计

## 背景

`0.1.1` 在 MacBook Pro M4 Max 上能够正常读取 Claude Code 与 DeepSeek，但 OpenAI Codex 在 Services 和详情页均显示未安装。当前定位器虽然检查进程 `PATH`、`~/.local/bin`、Homebrew 与系统目录，却没有检查 OpenAI 桌面应用内置的 `codex`。Finder 启动的菜单栏应用也不能依赖交互式 Shell 的路径配置。

## 目标

1. 在 Finder 启动的最小 `PATH` 下仍能发现用户级 Codex CLI；
2. 直接发现已安装 ChatGPT/Codex 桌面应用内可执行的 `codex`，无需用户手工创建软链接；
3. 搜索顺序稳定，优先尊重用户显式安装的 CLI；
4. 确实没有任何 Codex 可执行文件时，在 Services 提供官方安装指南入口；
5. 同一个定位器继续供账户状态、登录和官方额度采集共用，避免三个界面状态分叉。

## 搜索顺序

1. 当前进程 `PATH`；
2. `~/.local/bin`；
3. `/opt/homebrew/bin`、`/usr/local/bin`、`/usr/bin`；
4. 用户与系统 Applications 中 OpenAI 桌面应用的内置 Resources 目录。

只有通过系统可执行权限检查的候选才会被采用。应用不复制、不修改、不重新签名 OpenAI 的二进制文件。

## 缺失恢复

当 OpenAI Codex 状态为 `notInstalled` 时，原本不可用的登录按钮替换为 **Open Install Guide**。入口只打开 OpenAI 官方 Codex CLI 指南；安装完成后用户点击 **Check Status** 即可。其他状态继续显示 Sign in / Sign in again。

## 验收

- 空 `PATH` + 可执行的桌面应用内置 Codex：能够找到；
- 多个候选同时存在：优先使用用户显式 CLI；
- 候选不可执行：不得误报已安装；
- 未安装状态：官方指南入口可触发并显示反馈；
- 完整测试、Release 构建、资源/签名验证、ZIP 解压后启动验证通过。
