# 2026-09-01 服务账户、重新登录与需求台账

## 背景与目标

Services 原本只说明 Claude/Codex 由 CLI 管理，无法看到当前账号，也没有重新登录入口；DeepSeek 会直接覆盖保存输入的 Key。与此同时，长时间开发过程中缺少统一的碎片化需求记录位置。

本阶段目标：

1. Services 始终显示 Claude、Codex、DeepSeek 的准确连接状态与可安全展示的身份；
2. Claude/Codex 一键打开官方登录或重新登录流程，并在有限时间内自动回查；
3. DeepSeek 候选 Key 验证成功后才替换旧 Key；
4. 建立项目级待完成需求台账，所有未来需求先登记再处理。

## 影响范围

- `AIMeterCore/Accounts`：账户状态、三项 Reader、协调器、登录脚本与 DeepSeek 凭据管理器；
- `CodexAppServerClient`：复用现有初始化、超时与进程树清理生命周期读取 `account/read`；
- `AppModel`：内存账户状态、登录任务取消、最长 2 分钟回查、DeepSeek 替换反馈；
- `Settings > Services`：账户行、登录/重新登录、检查状态、遮罩 Key 与验证进度；
- `AGENTS.md` 与 `docs/requirements-backlog.md`：需求登记、分类、状态和完成证据。

## 测试驱动证据

每个功能先加入失败测试，再做最小实现：

- Claude 状态模型与 Reader：缺失类型导致编译失败后实现，16 项定向测试通过；
- Codex `account/read`：Reader 缺失和响应路由错误先被测试捕获，15 项账户/额度/超时回归通过；
- CLI 登录启动器：Builder/Launcher 缺失后实现，7 项固定命令、quoting、权限和失败测试通过；
- DeepSeek 两阶段替换：Manager 缺失后实现，20 项验证、回滚、遮罩与余额测试通过；
- AppModel/Settings：状态和操作缺失后实现，并额外捕获“已连接账号重新登录时旧状态误报成功”问题，23 项账户、启动、路由和隐私测试通过。

完整命令：

```text
bash scripts/test.sh
```

结果：`267 tests in 55 suites passed`，0 failures；4 项既有真实环境测试按门控跳过。新增的并发回归用例确认：前一次登录检查即使在读取过程中被取消，也不能覆盖后一次登录产生的新账户状态；启动刷新与 Settings 刷新重叠时只执行一次服务读取，避免 DeepSeek 被短暂误报为不可用。

## 安全与隐私决定

- Claude/Codex 登录脚本只含固定官方命令，权限 `0700`，不接收用户参数或秘密；
- 邮箱、套餐、认证方式和 DeepSeek Key 后四位只存在 App 内存；
- DeepSeek 验证前不调用 Keychain `save`/`delete`；验证、网络、超时和写入失败都保留或恢复旧 Key；
- AppModel 初始化仍保持 Keychain 读取次数为 0；读取在受限后台任务进行；
- Widget、统一快照、通知、缓存和日志没有账户身份字段。

## Git 节点

| 提交 | 内容 |
| --- | --- |
| `360667a` | 设计规格 |
| `641f74c` | 项目需求台账与根级规则 |
| `ba5ee02` | 实施计划 |
| `f95c6cf` | Claude 账户状态 |
| `4996e01` | Codex 账户身份 |
| `0d9a86d` | 官方 CLI 登录启动器 |
| `4993361` | DeepSeek 安全换 Key |
| `7175c70` | Settings 账户管理 |
| `698b4d4` | 防止旧登录回查覆盖新状态 |
| `bfc7412` | 合并重叠的服务账户刷新 |
| `cd77e25` | 合入 `main` |

## Release 与真实验收

- `bash scripts/build-app.sh` 完成 arm64 Release 构建；由于没有 Apple Development identity，Widget 按既定计划跳过，主应用不受影响；
- `codesign --verify --deep --strict` 与 `plutil -lint` 均通过；安装前后可执行文件 SHA-256 均为 `0d8b5df8b3949bfd53aa77cedc303b61ee95efda24ee3fcf89ef21ecc803c322`；
- 最终 Release 已安装到 `/Applications/AI Token Meter.app`，被替换版本保存在独立临时备份目录，可恢复；
- 真实 Settings 使用系统字体，四个顶部 Tab 正常；Services 始终展示 Claude、Codex、DeepSeek 三个区块；
- Claude 与 Codex 均读到当前本机账户与套餐，显示 **Sign in again** 和 **Check Status**；点击 Claude 的 **Check Status** 后身份保持一致；
- DeepSeek 显示安全输入框、余额基准、验证后保存说明与禁用状态。验收机上的既有 Keychain 项仍存在，但它由旧的临时签名版本创建；本阶段的 ad-hoc 签名 CDHash 已变化，macOS 不允许新哈希静默读取，因此真实界面诚实显示 `Account status unavailable`，未读取、输出、删除或覆盖旧 Key；
- 官方登录启动器的固定命令、`0700` 权限、缺失 CLI 与 Terminal 打开失败路径由 7 项测试覆盖。真实验收没有启动新的登录会话，避免干扰当前已登录账户。

## 已知限制与后续事项

- 官方 CLI 登录仍可能需要浏览器、密码或 MFA；AI Token Meter 只打开流程，不代填、不截获；
- 已连接账号选择重新登录时，旧身份未变化不会被误判为完成；用户可在官方流程结束后点击 **Check Status**；
- 当前主应用使用 ad-hoc 临时签名，每次代码变化都会产生新的 CDHash。旧版本创建的 Keychain 项可能因此要求重新授权或在稳定签名版本中重新录入一次；这是 macOS 钥匙串访问控制，不会通过降低安全性绕过；
- Widget 的 Apple Development 证书与 Gallery 真实验收继续按 `REQ-20260901-003` 延期；
- Mission Control、第二普通 Space、真实指针拖动和多显示器补验继续按 `REQ-20260901-004` 受环境限制。
