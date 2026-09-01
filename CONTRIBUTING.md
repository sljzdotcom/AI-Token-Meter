# 贡献指南

感谢关注 AI Token Meter。这个项目直接接触本机 CLI、Keychain 和账户用量，因此正确性、隐私与可恢复性优先于快速堆叠功能。

## 开始之前

1. 阅读 [README](README.md) 和 [文档索引](docs/README.md)。
2. 架构改动先阅读 [架构概览](docs/architecture/overview.md)。
3. 涉及凭证、网页、缓存或日志时先阅读 [隐私与安全](docs/security-and-privacy.md)。
4. 安全漏洞不要公开提交，按 [SECURITY.md](SECURITY.md) 报告。

## 开发流程

1. 从最新主分支创建短生命周期分支，推荐 `codex/<topic>`。
2. 为新行为或修复先增加能够失败的测试。
3. 做满足需求的最小实现，避免顺手重构无关模块。
4. 更新相关 README、用户指南、架构文档和 `CHANGELOG.md`。
5. 运行完整测试和差异检查。
6. 在 Pull Request 中说明数据来源、隐私影响、测试证据和已知限制。

## 提交信息

使用清晰的类型前缀：

```text
feat: add provider usage collector
fix: preserve timeout cause
docs: document provider setup
test: cover malformed usage response
refactor: isolate presentation mapping
chore: update build tooling
release: package AI Token Meter v0.2.0
```

一个提交应表达一个可理解、可回滚的节点。不要把密钥、账户数据、构建产物或临时调试文件纳入提交。

## 测试要求

最少执行：

```bash
bash scripts/test.sh
git diff --check
```

涉及 App Bundle 或系统集成时还应执行：

```bash
bash scripts/build-app.sh
codesign --verify --deep --strict "dist/AI Token Meter.app"
```

真实 CLI 与 Keychain 测试需要显式许可，具体见 [测试指南](docs/development/testing.md)。Pull Request 不应要求 CI 使用个人账户凭证。

## 新数据源要求

新增服务或指标必须说明：

- 是否为官方、稳定且允许使用的数据接口；
- 认证方式与凭证保存位置；
- 采集超时、速率限制和失败降级；
- 原始响应是否落盘；
- 统一快照中的指标口径；
- 数值、圆环和通知是否使用同一计算；
- fixture 如何去除个人和凭证信息。

禁止通过未经说明的浏览器 Cookie 抓取、绕过登录、自动兑换额度或静默执行产生费用的操作。

## 文档要求

- 用户可见变化：更新 README、对应用户指南和 `CHANGELOG.md`；
- 新设置：更新 `docs/user-guide/settings.md`；
- 新故障模式：更新排障指南；
- 模块或目录变化：更新代码库结构；
- 凭证、网络、缓存变化：更新隐私与安全文档；
- 发布：更新版本号、发布流程记录和提交历史。

## 代码评审重点

- 数据是否来自正确的账户与额度窗口；
- 失败或缓存是否被清楚标识；
- 外部进程是否能在超时和取消后收尾；
- 是否意外记录密钥、Token、Cookie 或原始账户响应；
- UI、菜单栏和无障碍描述是否一致；
- 退出、窗口关闭和屏幕变化后是否遗留任务或监听器。

## 许可

仓库目前尚未声明开源许可证。在许可证确定前，贡献的接受和再分发条件并不明确；请先与维护者确认，再提交计划用于公开再分发的大型改动。
