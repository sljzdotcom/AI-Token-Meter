# Security Policy

## 支持范围

当前维护主分支最新状态和公开双平台稳定 Release `0.4.0`。旧 Windows Preview 更新源已引导升级到稳定版；真实 CI/真机状态按需求台账和 Release notes 如实管理。`CHANGELOG.md` 的 `Unreleased` 与开发日志说明尚未进入公开安装包的改动，开发中的条目明确标注状态。

项目通过 GitHub Releases 和签名更新清单发行，但尚无长期支持分支；旧提交不会单独获得安全补丁。发现问题后应先在主分支修复，再决定是否制作新版本。

## 第三方 CLI 安装信任边界

正在开发的 Settings 安装引导仅在用户明确点击后，下载并运行 Claude Code 或 OpenAI Codex 的官方安装器；该功能不包含在 0.4.0 中。安装地址按服务固定，不能由网页或任意前端参数指定。完整下载失败时不执行脚本；不使用管理员权限，不修改系统全局执行策略，不记录安装器原始输出或登录凭据。

这些安装器属于上游服务，其内容可能更新。HTTPS 官方地址并不等于本项目对每个上游版本进行了代码审计，也不同于 AI Token Meter 自身的签名更新校验。用户可以选择官方指南自行安装；本应用不会在后台静默安装第三方 CLI。实现与验证边界见[开发记录](docs/development/2026-09-07-cli-onboarding.md)。

## 私下报告漏洞

请不要在公开 Issue、讨论区或 Pull Request 中披露尚未修复的漏洞，也不要上传真实：

- DeepSeek API Key；
- Claude/Codex Token 或配置文件；
- Cookie、Authorization 请求头；
- 完整账户响应；
- 包含邮箱、姓名、组织或余额明细的截图。

优先使用 GitHub 仓库的 [Security → Report a vulnerability](https://github.com/sljzdotcom/AI-Token-Meter/security/advisories/new) 私有报告功能。如果该入口暂时不可用，请等待维护者恢复私密报告渠道，不要改为公开披露；报告只发送最小复现信息。

报告应包含：

1. 受影响提交、版本和 macOS/Windows 版本；
2. 问题类型与潜在影响；
3. 不含真实凭证的最小复现步骤；
4. 是否需要已登录账户、网络或本机权限；
5. 建议修复（如有）。

## 处理原则

- 确认收到后先评估是否涉及凭证、代码执行、越权网络访问或敏感数据落盘；
- 修复开发和测试使用虚构或脱敏 fixture；
- 在补丁可用前限制披露范围；
- 修复后更新 `CHANGELOG.md`、隐私文档、测试和开发日志；
- 如已公开发布受影响版本，发布安全版本并清楚说明升级路径。

## 范围内问题

- Keychain 访问控制或密钥泄漏；
- Windows Credential Manager 访问控制或密钥泄漏；
- CLI 凭证、Token、Cookie 或账户响应被记录或缓存；
- DeepSeek WebKit 会话接受非官方来源数据；
- DeepSeek WebView2 会话接受非官方来源、泄露 Cookie/授权头或绕过 nonce/大小边界；
- 外部输入引发任意命令执行或路径注入；
- 未经用户同意兑换额度、发起付费调用或修改账户；
- 缓存、通知或日志暴露敏感账户信息。
- Sparkle/Tauri 更新签名绕过、Release 资产替换或未验证更新安装。

## 不属于安全漏洞

- 上游 Claude、Codex 或 DeepSeek 服务暂时不可用；
- 官方界面和 AI Token Meter 刷新时间不同造成的短暂数值差异；
- 上游未公开格式变化导致解析暂时失败，但没有泄露或越权；
- 在已完全控制当前 macOS 或 Windows 用户账户的前提下读取该用户可访问的数据。
