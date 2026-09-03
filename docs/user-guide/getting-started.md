# 安装与首次使用

## 1. 准备环境

AI Token Meter 稳定版支持 Apple Silicon Mac 与 macOS 14 或更新版本；Windows 11 x64 版正在 Preview 验收。从源码构建的共同依赖是 Git，平台依赖分别为：

- Xcode Command Line Tools；
- Swift 6 工具链；
- Git；
- Claude Code CLI（可选）；
- OpenAI Codex CLI（可选）；
- DeepSeek API Key（可选）。

Windows 还需要 Node.js 24、Rust 1.88、Microsoft C++ Build Tools 与 WebView2 Runtime；macOS 需要 Xcode Command Line Tools 与 Swift 6。三项 Provider 在两平台均为可选服务。

三项服务彼此独立。没有安装或配置某项服务时，其他服务仍可正常使用。

## 2. 下载公开版本

从 [GitHub Releases](https://github.com/sljzdotcom/AI-Token-Meter/releases/latest) 下载 `AI-Token-Meter-0.2.2-macOS-arm64.zip` 和同名 `.sha256`。在下载目录验证：

```bash
shasum -a 256 -c AI-Token-Meter-0.2.2-macOS-arm64.zip.sha256
```

解压后把 `AI Token Meter.app` 移到 `/Applications`。当前公开包为 ad-hoc signed、not notarized；首次运行如果 macOS 阻止，请在 Finder 中右键 App 并选择“打开”。不要从不可信镜像下载，也不要绕过更新签名失败。

`0.1.2` 不含更新器，因此要手动安装一次当前版本。安装 `0.2.0` 或更新版本后，后续稳定版本可在 Settings → About 手动检查和安装。

首个 Windows Preview 发布后，同一 GitHub Release 会包含 `AI-Token-Meter-X.Y.Z-windows-x64-setup.exe` 与同名 `.sha256`。当前 `v0.2.2` 没有 Windows 资产；不要从第三方网盘取得所谓 Windows 版本。Preview 安装器是 current-user NSIS，不要求管理员权限；取得 Authenticode 证书前 Windows 可能显示 SmartScreen，请先确认发布页域名和 SHA-256。

## 3. 从源码构建

### macOS

```bash
git clone https://github.com/sljzdotcom/AI-Token-Meter.git
cd AI-Token-Meter
bash scripts/build-app.sh
```

成功后应用位于：

```text
dist/AI Token Meter.app
```

启动测试：

```bash
open "dist/AI Token Meter.app"
```

构建脚本会执行 release 构建、确定性生成完整尺寸的仪表指针 App Icon、组装 `.app`、复制资源、嵌入 Sparkle framework 与 helper、校验 `Info.plist`、执行 ad-hoc 签名并验证签名和运行路径。ad-hoc 签名适合本机使用，不等同于面向其他用户分发所需的 Developer ID 签名与 Apple 公证。

默认构建使用 `AI_METER_INCLUDE_WIDGET=auto`：检测到真实 Apple Development 证书和 Team ID 时包含 Widget；否则显示 `Widget skipped` 并继续生成普通主应用。Widget 不能使用 ad-hoc 签名共享 App Group。首次准备方式：

1. 打开 Xcode > Settings > Accounts；
2. 登录 Apple Account，并让 Xcode 创建 Apple Development 证书；
3. 返回仓库执行 `AI_METER_INCLUDE_WIDGET=1 bash scripts/build-app.sh`；
4. 脚本会自动计算双方相同的 App Group、先签 Widget、再签主应用并验证嵌套包。

### Windows 11 x64

```powershell
git clone https://github.com/sljzdotcom/AI-Token-Meter.git
cd AI-Token-Meter\windows
npm ci
npm test
npm run tauri build
```

NSIS 安装器输出到 `windows\src-tauri\target\release\bundle\nsis\`。普通构建不会生成可发布的 updater signature，也不需要任何私钥。安装后从系统托盘打开 Settings；Windows Widget 尚未实现。

## 4. 安装到应用程序

1. 从菜单栏退出正在运行的 AI Token Meter。
2. 把 `dist/AI Token Meter.app` 移到 `/Applications`。
3. 从“应用程序”重新启动。
4. 如果之前开启过“登录时启动”，在设置中关闭再重新开启，以更新应用路径。

本机 ad-hoc 签名会随重新构建而变化。覆盖安装后，macOS 可能要求重新确认 AI Token Meter 对已保存 DeepSeek Keychain 项目的访问；确认应用路径为 `/Applications/AI Token Meter.app` 后，输入登录钥匙串密码并选择 **Always Allow**。这是系统的本机签名更新保护，不代表 API Key 被修改。

### 添加桌面 Widget

只有使用 Apple Development 签名且包内包含 `.appex` 的版本会出现在 Widget Gallery：

1. 先启动一次 `/Applications/AI Token Meter.app` 并手动刷新；
2. 在桌面空白处右键，选择“编辑小组件”；
3. 搜索 **AI Token Meter**；
4. 选择尺寸并拖到桌面。

| 尺寸 | 显示内容 |
| --- | --- |
| Small | 三个同尺寸 Provider Logo 状态环；无可见文字 |
| Medium | Claude Code、OpenAI Codex、DeepSeek 三张额度/余额卡 |
| Large | 三项 Provider 行、最近重置、OpenAI Codex 重置券数量和最近到期 |

点击 Widget 只会唤醒 AI Token Meter 并触发刷新，不会自动登录、消费额度或兑换重置券。WidgetKit 按系统预算调度，主应用刷新后通常不是逐秒更新；超过快照有效期时 Widget 会显示陈旧状态。

## 5. 配置 Claude Code

1. 在终端确认 `claude` 可以正常启动。
2. 使用 Claude Code 官方流程完成登录。
3. 回到 AI Token Meter 手动刷新。
4. 如果显示需要工作区设置，点击 **Open one-time setup**。
5. AI Token Meter 会打开终端，在专用空目录中启动 Claude Code；批准该工作区后退出终端并再次刷新。

AI Token Meter 不在用户项目中运行 `/usage`，而是在兼容目录 `Application Support/AI Meter` 下的私有空工作区执行，以减少项目指令、MCP 服务或当前会话对额度查询的干扰。显示名称改版后继续保留该目录，以沿用既有批准和缓存。

Windows 会先查原生 CLI，再查可用 WSL 发行版；Services 会明确显示 `Native Windows` 或 `WSL · <发行版>` 与 CLI 版本。登录按钮只打开固定官方登录命令，不会把密码、Token 或自定义 Shell 文本交给应用。

## 6. 配置 OpenAI Codex

1. 在终端启动 OpenAI Codex CLI。
2. 使用官方流程完成登录。
3. 回到 AI Token Meter 手动刷新。

AI Token Meter 通过 OpenAI Codex CLI 的 `app-server` 结构化接口读取账户速率限制，不解析终端全屏界面。详情页下方还会读取本机 OpenAI Codex 状态库中的三个数值/时间列，计算近 30 天 Token、连续活动天数和最长会话；这些值不包含其他设备活动。

从 Finder 启动时，AI Token Meter 不依赖 `.zshrc`：它会自动检查 `~/.local/bin`、Homebrew、nvm 与常见 Node 管理器目录，也能使用已安装 ChatGPT/Codex App 内置的原生 `codex`。通过 nvm/npm 安装的脚本会自动配对同目录 Node，无需手工修改 `launchctl PATH`。确实没有可执行文件时，可在 Settings > Services 点击 **Open Install Guide**。

Windows 同样不依赖交互式 PowerShell 配置：会检查当前进程环境、注册表 PATH、常见 Node 安装和 WSL。Native/WSL 发现、账号读取和实际采集始终使用同一候选，避免状态页与用量页来自不同账号。

## 7. 配置 DeepSeek

### 余额

1. 打开 AI Token Meter 设置，并进入 **Services** Tab。
2. 在 **DeepSeek API Key** 中粘贴密钥并点击 **Save**。
3. 确认页面显示安全保存状态；macOS 使用 Keychain，Windows 使用 Credential Manager。
4. 设置 **Balance baseline**；默认值为 ¥100。

余额基准只是圆环参考值，不会修改 DeepSeek 账户或设置消费上限。例如基准 ¥100、余额 ¥77.99 时，显示约 22.01% 已消耗。

### 最近 30 天用量

1. 点击 DeepSeek 圆环打开详情。
2. 首次使用时在嵌入的官方 `platform.deepseek.com` 页面登录。
3. 登录并打开用量页面后，AI Token Meter 会在详情页展示最近 30 天的成本、请求数、Token 数和每日成本图。

该登录会话只属于 AI Token Meter，不读取其他浏览器的 Cookie。网页结构或网络暂时不可用时，详情会退回最近一次标准化缓存。

## 8. 检查与安装更新

1. 打开 Settings → About。
2. 点击 **Check for Updates**。只有此时应用才访问 GitHub 更新清单。
3. 如果显示新版本，点击 **Update Now**。
4. macOS 在 Sparkle 标准窗口确认安装；Windows 在 Tauri/NSIS 安装流程中确认。两平台都只在更新签名验证通过后替换并重新启动。

显示 `You're up to date` 时，**Update Now** 会保持禁用。离线、清单不可用、签名不匹配或目标不可写时，当前应用保持不变。Windows 使用固定 GitHub `latest.json` 与内置 minisign 公钥；macOS 使用固定 appcast 与 Sparkle EdDSA 公钥。参见[故障排查](troubleshooting.md#检查更新失败或-update-now-不可用)。

## 9. 常用操作

- 点击圆环：展开对应服务详情。
- 从三个 Logo 以外的玻璃空白处拖动：调整浮岛上下位置；Automatic 模式还可拖到另一侧。
- 设置中的 **Screen edge**：选择 Automatic、Left 或 Right。
- 点击空白处：立即关闭详情。
- 悬停在详情上：暂停自动隐藏倒计时。
- 菜单栏 Quantum Dial：断环进度和指针表示三项服务中的最高有效已用比例，旁边显示精确百分比；无有效数据时显示中性仪表与 `—`。
- 菜单栏刷新按钮：立即刷新三项服务，并同步更新 Quantum Dial 与百分比。
- `⌘,`：打开设置。
- 菜单栏退出项：完全退出应用。
- Settings → About 的 **Check for Updates**：手动检查 GitHub 稳定版；不会开启后台检查。

## 10. 卸载

1. 在 **Monitoring** 设置中关闭“Open AI Token Meter at login”。
2. 如需删除 DeepSeek API Key，点击 **Remove**。
3. 从菜单栏退出 AI Token Meter。
4. 删除应用。

如需彻底清除非敏感缓存与偏好，可另外删除用户 `Application Support` 中的 `AI Meter` 目录及相关 `UserDefaults`。执行前请先备份需要保留的数据。

Windows 可从 Settings > Apps > Installed apps 卸载；如需彻底清理非敏感状态，再删除当前用户 `%APPDATA%\AI Token Meter` 与 `%LOCALAPPDATA%\AI Token Meter`。Credential Manager 中的 DeepSeek 项和官方 CLI 登录不会被静默删除，必须由用户明确移除。
