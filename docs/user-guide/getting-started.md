# 安装与首次使用

## 1. 准备环境

AI Meter 当前支持 Apple Silicon Mac 与 macOS 14 或更新版本。从源码构建需要：

- Xcode Command Line Tools；
- Swift 6 工具链；
- Git；
- Claude Code CLI（可选）；
- Codex CLI（可选）；
- DeepSeek API Key（可选）。

三项服务彼此独立。没有安装或配置某项服务时，其他服务仍可正常使用。

## 2. 构建应用

```bash
git clone <repository-url>
cd AI-Meter
bash scripts/build-app.sh
```

成功后应用位于：

```text
dist/AI Meter.app
```

启动测试：

```bash
open "dist/AI Meter.app"
```

构建脚本会执行 release 构建、确定性生成完整尺寸的仪表指针 App Icon、组装 `.app`、复制资源、校验 `Info.plist`、执行 ad-hoc 签名并验证签名。ad-hoc 签名适合本机使用，不等同于面向其他用户分发所需的 Developer ID 签名与 Apple 公证。

## 3. 安装到应用程序

1. 从菜单栏退出正在运行的 AI Meter。
2. 把 `dist/AI Meter.app` 移到 `/Applications`。
3. 从“应用程序”重新启动。
4. 如果之前开启过“登录时启动”，在设置中关闭再重新开启，以更新应用路径。

本机 ad-hoc 签名会随重新构建而变化。覆盖安装后，macOS 可能要求重新确认 AI Meter 对已保存 DeepSeek Keychain 项目的访问；确认应用路径为 `/Applications/AI Meter.app` 后，输入登录钥匙串密码并选择 **Always Allow**。这是系统的本机签名更新保护，不代表 API Key 被修改。

## 4. 配置 Claude

1. 在终端确认 `claude` 可以正常启动。
2. 使用 Claude Code 官方流程完成登录。
3. 回到 AI Meter 手动刷新。
4. 如果显示需要工作区设置，点击 **Open one-time setup**。
5. AI Meter 会打开终端，在专用空目录中启动 Claude；批准该工作区后退出终端并再次刷新。

AI Meter 不在用户项目中运行 `/usage`，而是在 `Application Support/AI Meter` 下的私有空工作区执行，以减少项目指令、MCP 服务或当前会话对额度查询的干扰。

## 5. 配置 Codex

1. 在终端启动 Codex CLI。
2. 使用官方流程完成登录。
3. 回到 AI Meter 手动刷新。

AI Meter 通过 Codex CLI 的 `app-server` 结构化接口读取账户速率限制，不解析终端全屏界面。详情页下方还会读取本机 Codex 状态库中的三个数值/时间列，计算近 30 天 Token、连续活动天数和最长会话；这些值不包含其他设备活动。

## 6. 配置 DeepSeek

### 余额

1. 打开 AI Meter 设置。
2. 在 **DeepSeek API Key** 中粘贴密钥并点击 **Save**。
3. 确认页面显示 **Stored securely in Keychain**。
4. 设置 **Balance baseline**；默认值为 ¥100。

余额基准只是圆环参考值，不会修改 DeepSeek 账户或设置消费上限。例如基准 ¥100、余额 ¥77.99 时，显示约 22.01% 已消耗。

### 最近 30 天用量

1. 点击 DeepSeek 圆环打开详情。
2. 首次使用时在嵌入的官方 `platform.deepseek.com` 页面登录。
3. 登录并打开用量页面后，AI Meter 会在详情页展示最近 30 天的成本、请求数、Token 数和每日成本图。

该登录会话只属于 AI Meter，不读取其他浏览器的 Cookie。网页结构或网络暂时不可用时，详情会退回最近一次标准化缓存。

## 7. 常用操作

- 点击圆环：展开对应服务详情。
- 从三个 Logo 以外的玻璃空白处拖动：调整浮岛上下位置；Automatic 模式还可拖到另一侧。
- 设置中的 **Screen edge**：选择 Automatic、Left 或 Right。
- 点击空白处：立即关闭详情。
- 悬停在详情上：暂停自动隐藏倒计时。
- 菜单栏刷新按钮：立即刷新三项服务。
- `⌘,`：打开设置。
- 菜单栏退出项：完全退出应用。

## 8. 卸载

1. 在设置中关闭“Open AI Meter at login”。
2. 如需删除 DeepSeek API Key，点击 **Remove**。
3. 从菜单栏退出 AI Meter。
4. 删除应用。

如需彻底清除非敏感缓存与偏好，可另外删除用户 `Application Support` 中的 `AI Meter` 目录及相关 `UserDefaults`。执行前请先备份需要保留的数据。
