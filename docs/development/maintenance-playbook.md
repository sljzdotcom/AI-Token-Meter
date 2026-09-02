# 维护手册

这份手册面向数月后重新接手项目的维护者。先读[当前项目状态](../project-status.md)，再按任务使用下面的最短路径。

## 开始任何改动

1. 阅读并更新[需求台账](../requirements-backlog.md)，新事项使用下一个 `REQ-YYYYMMDD-NNN`；
2. 检查 `git status --short --branch`，不要覆盖用户已有修改；
3. 在 `.worktrees/<topic>` 创建隔离分支，确认 `.worktrees/` 仍被忽略；
4. 运行 `scripts/test.sh` 建立基线；若偶发并发/PTY 超时，先单独复跑对应 suite，再完整复跑并记录两次结果；
5. 行为变更先写失败测试，设计变更先写规格和实施计划；
6. 每个独立可回滚阶段提交一次，完成后再合入 `main`。

## 变更影响矩阵

| 改动 | 必查代码/测试 | 必改文档 |
| --- | --- | --- |
| Provider 数据源或解析 | `Collectors/`、`Domain/`、fixture、隐私回归 | providers、architecture、security、troubleshooting、CHANGELOG |
| 圆环或百分比 | `ProviderPresentation`、Widget builder、通知阈值 | providers、README；确认文字/圆环/通知同源 |
| 账户或凭证 | `Accounts/`、`Security/`、Services、Keychain 测试 | security、settings、troubleshooting |
| 窗口/浮岛/详情 | `FloatingPanelController`、presentation policy、布局/命中测试 | settings、architecture、开发日志 |
| 字体、颜色、Logo | typography、palette、visual tests、Widget 镜像资源 | settings、README、CHANGELOG |
| Settings | 对应 Tab、消息路由、Settings structure tests | settings、README |
| Widget | Core snapshot contract、publisher、extension、build script | status、architecture、security、release、Widget 日志 |
| 版本/发布 | 两个 Info.plist、README 徽章、CHANGELOG | release、commit history、项目状态 |
| 应用更新 | `SoftwareUpdate/`、About、Info.plist、Sparkle Bundle/归档验证 | settings、architecture、security、release、更新日志 |

## 三服务诊断顺序

### Claude Code

1. 在 Settings > Services 查看 CLI 安装、账户和工作区状态；
2. 终端确认 `claude auth status --json`；
3. 若账号正常但额度超时，使用 **Authorize Usage Workspace** 完成私有工作区批准；
4. 检查 `/usage` 格式是否变化，只保存去身份化最小 fixture；
5. 官方额度失败应立即返回，本机活动最多等待 2 秒且不能遮蔽官方结果。

### OpenAI Codex

1. 在 Services 检查账户，需要时运行官方 `codex login`；
2. 确认 `codex app-server` 可启动并返回 `account/rateLimits/read`；
3. 检查顶层通用窗口是否仍与模型专属窗口分开；
4. 本机统计失败时检查 `~/.codex/state_5.sqlite` 是否存在/可读，但不要打开或记录对话文本列；
5. 充值券只读展示，永不自动兑换。

### DeepSeek

1. Services 只应显示 Key 后四位；完整 Key 不得出现在日志、截图或 fixture；
2. 新 Key 必须先通过官方余额接口验证，失败保留旧 Key；
3. 余额正常但历史图表异常时，分别检查隔离 WebKit 登录、官方 host、用量/费用两组分片和标准化缓存；
4. 官网结构变化时允许历史不可用，不允许解析任意来源或保存原始响应；
5. 圆环计算始终是 `(基准 - 余额) / 基准` 的 0...1 夹紧值。

## 本地数据与安全处置

- 普通排障优先手动刷新和查看“更新时间/状态”，不要先删缓存；
- 需要重置缓存时，先退出 App并备份 `~/Library/Application Support/AI Meter`，再只处理明确文件；
- 不自动删除 Keychain 项、WebKit Cookie 或官方 CLI 登录；这三类操作都可能造成不可恢复的重新认证；
- 错误消息、通知、缓存和 Widget 共享字符串必须经过 `SensitiveTextRedactor`；
- fixture 只保留解析所需字段，替换邮箱、账户 ID、Token、Cookie、项目名和真实时间线。

## 测试、构建与安装

```bash
scripts/test.sh
scripts/check-docs.sh
scripts/build-app.sh
codesign --verify --deep --strict --verbose=2 "dist/AI Token Meter.app"
scripts/verify-update-bundle.sh "dist/AI Token Meter.app"
```

真实 CLI/Keychain 门控和 Widget 强制构建命令见[测试指南](testing.md)。安装验收：

1. 完全退出当前 App；
2. 把 `/Applications/AI Token Meter.app` 移到带时间戳的 `/private/tmp` 备份；
3. 复制 `dist/AI Token Meter.app`；
4. 对比候选与安装版 `Contents/MacOS/AIMeterApp` 的 SHA-256；
5. 验证严格签名、arm64、菜单图标、浮岛、三详情、Settings 与自动隐藏；
6. 失败时退出新版本，恢复完整旧 Bundle，而不是只替换可执行文件。

## 发布前不可省略

- 更新主应用和 Widget `Info.plist` 的版本/build；
- README 版本徽章、项目状态和 CHANGELOG 同步；
- 完整测试、文档检查、Release、严格签名和实机验收；
- 确认许可证、远程、tag、Developer ID、公证、校验和与支持范围是真实存在的；
- 没有证书时明确发布“无 Widget 主应用”，不要声称 Widget 可安装。
- 更新发布必须使用 Keychain 中的 Sparkle EdDSA 私钥，经单一脚本生成 ZIP、SHA-256 和 appcast；不得手改 enclosure 签名或在生成 appcast 后重建 ZIP。
- 发布后必须验证 raw appcast、Release 下载和匿名 SHA-256；更新失败或签名异常时保留当前 App，禁止指导用户绕过验证。

## 更新故障与密钥轮换

1. About 无法检查时，先核对固定 feed 和 GitHub Release 资产是否可匿名访问，再检查代理/离线状态；不要开启隐藏后台轮询。
2. `Update Now` 禁用时先确认当前状态是否真的发现更高版本；最新版和失败状态按设计禁用。
3. 下载后拒绝安装时，依次核对 enclosure 长度、版本/build、ZIP 是否在签名后被修改，以及 App 内公开键是否匹配。
4. 私钥只允许留在维护者 Keychain；任何疑似泄露都按[发布流程](release-process.md#回滚更新发布)停止发布并轮换信任根。

## 日志与证据

- 系统崩溃：`~/Library/Logs/DiagnosticReports/AIMeterApp-*.ips`；
- 实时日志：Console.app 按进程 `AIMeterApp` 筛选；
- 功能阶段：新增 `docs/development/YYYY-MM-DD-topic.md` 并加入[日志索引](README.md)；
- Git 关键节点：更新[提交历史](commit-history.md)；
- 用户可见变化：更新根 [CHANGELOG](../../CHANGELOG.md)；
- 阻塞和延期：只更新需求台账的合法状态，绝不伪装完成。

## 定期健康检查

每次阶段结束运行：

```bash
git status --short --branch
git diff --check
scripts/check-docs.sh
```

同时确认没有重新出现第二需求列表、`docs/superpowers`、被跟踪的 `.app`/`.build`、真实凭证、无入口文档或已合并但长期占用空间的临时 worktree。
