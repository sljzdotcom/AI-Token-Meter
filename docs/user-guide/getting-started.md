# 安装与首次使用

## 1. 准备环境

AI Token Meter 当前支持 Apple Silicon Mac 与 macOS 14 或更新版本。从源码构建需要：

- Xcode Command Line Tools；
- Swift 6 工具链；
- Git；
- Claude Code CLI（可选）；
- OpenAI Codex CLI（可选）；
- DeepSeek API Key（可选）。

三项服务彼此独立。没有安装或配置某项服务时，其他服务仍可正常使用。

## 2. 构建应用

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

构建脚本会执行 release 构建、确定性生成完整尺寸的仪表指针 App Icon、组装 `.app`、复制资源、校验 `Info.plist`、执行 ad-hoc 签名并验证签名。ad-hoc 签名适合本机使用，不等同于面向其他用户分发所需的 Developer ID 签名与 Apple 公证。

默认构建使用 `AI_METER_INCLUDE_WIDGET=auto`：检测到真实 Apple Development 证书和 Team ID 时包含 Widget；否则显示 `Widget skipped` 并继续生成普通主应用。Widget 不能使用 ad-hoc 签名共享 App Group。首次准备方式：

1. 打开 Xcode > Settings > Accounts；
2. 登录 Apple Account，并让 Xcode 创建 Apple Development 证书；
3. 返回仓库执行 `AI_METER_INCLUDE_WIDGET=1 bash scripts/build-app.sh`；
4. 脚本会自动计算双方相同的 App Group、先签 Widget、再签主应用并验证嵌套包。

## 3. 安装到应用程序

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

## 4. 配置 Claude Code

1. 在终端确认 `claude` 可以正常启动。
2. 使用 Claude Code 官方流程完成登录。
3. 回到 AI Token Meter 手动刷新。
4. 如果显示需要工作区设置，点击 **Open one-time setup**。
5. AI Token Meter 会打开终端，在专用空目录中启动 Claude Code；批准该工作区后退出终端并再次刷新。

AI Token Meter 不在用户项目中运行 `/usage`，而是在兼容目录 `Application Support/AI Meter` 下的私有空工作区执行，以减少项目指令、MCP 服务或当前会话对额度查询的干扰。显示名称改版后继续保留该目录，以沿用既有批准和缓存。

## 5. 配置 OpenAI Codex

1. 在终端启动 OpenAI Codex CLI。
2. 使用官方流程完成登录。
3. 回到 AI Token Meter 手动刷新。

AI Token Meter 通过 OpenAI Codex CLI 的 `app-server` 结构化接口读取账户速率限制，不解析终端全屏界面。详情页下方还会读取本机 OpenAI Codex 状态库中的三个数值/时间列，计算近 30 天 Token、连续活动天数和最长会话；这些值不包含其他设备活动。

从 Finder 启动时，AI Token Meter 不依赖 `.zshrc`：它会自动检查 `~/.local/bin`、Homebrew、nvm 与常见 Node 管理器目录，也能使用已安装 ChatGPT/Codex App 内置的原生 `codex`。通过 nvm/npm 安装的脚本会自动配对同目录 Node，无需手工修改 `launchctl PATH`。确实没有可执行文件时，可在 Settings > Services 点击 **Open Install Guide**。

## 6. 配置 DeepSeek

### 余额

1. 打开 AI Token Meter 设置，并进入 **Services** Tab。
2. 在 **DeepSeek API Key** 中粘贴密钥并点击 **Save**。
3. 确认页面显示 **Stored securely in Keychain**。
4. 设置 **Balance baseline**；默认值为 ¥100。

余额基准只是圆环参考值，不会修改 DeepSeek 账户或设置消费上限。例如基准 ¥100、余额 ¥77.99 时，显示约 22.01% 已消耗。

### 最近 30 天用量

1. 点击 DeepSeek 圆环打开详情。
2. 首次使用时在嵌入的官方 `platform.deepseek.com` 页面登录。
3. 登录并打开用量页面后，AI Token Meter 会在详情页展示最近 30 天的成本、请求数、Token 数和每日成本图。

该登录会话只属于 AI Token Meter，不读取其他浏览器的 Cookie。网页结构或网络暂时不可用时，详情会退回最近一次标准化缓存。

## 7. 常用操作

- 点击圆环：展开对应服务详情。
- 从三个 Logo 以外的玻璃空白处拖动：调整浮岛上下位置；Automatic 模式还可拖到另一侧。
- 设置中的 **Screen edge**：选择 Automatic、Left 或 Right。
- 点击空白处：立即关闭详情。
- 悬停在详情上：暂停自动隐藏倒计时。
- 菜单栏 Quantum Dial：断环进度和指针表示三项服务中的最高有效已用比例，旁边显示精确百分比；无有效数据时显示中性仪表与 `—`。
- 菜单栏刷新按钮：立即刷新三项服务，并同步更新 Quantum Dial 与百分比。
- `⌘,`：打开设置。
- 菜单栏退出项：完全退出应用。

## 8. 卸载

1. 在 **Monitoring** 设置中关闭“Open AI Token Meter at login”。
2. 如需删除 DeepSeek API Key，点击 **Remove**。
3. 从菜单栏退出 AI Token Meter。
4. 删除应用。

如需彻底清除非敏感缓存与偏好，可另外删除用户 `Application Support` 中的 `AI Meter` 目录及相关 `UserDefaults`。执行前请先备份需要保留的数据。
