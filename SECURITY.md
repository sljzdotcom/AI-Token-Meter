# Security Policy

## 支持范围

当前仍在维护的代码为主分支最新状态、公开稳定 Release `0.2.2` 与双平台 Preview `0.3.0-preview.0`。Preview 的真实 CI/真机状态按需求台账和 Release notes 如实管理。`CHANGELOG.md` 的 `Unreleased` 只记录已合入、尚未进入下一个公开版本的安全与功能改动。

由于项目尚未建立正式发行渠道和长期支持分支，旧提交不会单独获得安全补丁。发现问题后应先在主分支修复，再决定是否制作新版本。

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
