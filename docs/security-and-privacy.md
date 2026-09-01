# 隐私与安全

## 安全目标

AI Token Meter 是本地状态查看器，不是账户代理。它遵循最小权限原则：只读取展示所需的用量数据，不替用户登录、不执行充值或重置操作，也不把账户数据上传到自建服务。

## 凭证处理

### Claude 与 Codex

- 登录与凭证生命周期由官方 CLI 管理；
- AI Token Meter 调用已登录 CLI，不读取、复制或保存凭证文件；
- Settings 的登录按钮只生成权限为 `0700` 的本地命令文件，内容固定为官方 `claude auth login` 或 `codex login`，不拼接用户输入或秘密；
- CLI 返回的邮箱、套餐和认证方式只保留在当前 App 进程的内存展示状态，不进入统一快照、Widget、通知、脚本或日志；
- Codex 本机活动只查询本地线程表的 `tokens_used`、`created_at`、`updated_at`，不读取标题、预览、提示词、回复或凭证；
- Claude 本机活动只解码本地 JSONL 的 `timestamp`、`sessionId`、`message.model` 和 `message.usage` 白名单字段；提示词、回复、项目路径、标题、分支和文件内容不会进入领域模型、缓存或日志；
- Claude JSONL 扫描跳过符号链接、损坏记录、负计数和超出上限的行/文件；子代理 Token 可计入总量，但不会被重复算作主会话；
- CLI 标准输出会在解析后转换为统一字段，原始账户输出不写入业务缓存；
- 一次性 Claude 工作区批准由用户在终端确认。

### DeepSeek API Key

- 只通过设置界面接收；
- 使用 macOS Keychain 保存；
- Keychain 可访问级别为 `AfterFirstUnlockThisDeviceOnly`；
- 不写入 `UserDefaults`、普通文件、通知、截图或日志；
- 设置界面不回显已保存的 Key，只在内存中显示最后四位遮罩；
- 替换时先用候选 Key 调官方余额接口，验证成功后才原子更新 Keychain；任何验证或写入失败都会保留/尽力恢复旧 Key。
- Keychain 还会校验调用应用的代码身份。开发版使用 ad-hoc 签名时，每次重编译的 CDHash 都可能变化；应用会把无法读取诚实显示为不可用，不会绕过访问控制、导出旧 Key 或降低 Keychain 权限。稳定发布应使用固定代码签名身份。

## DeepSeek 网页会话

近 30 天用量使用 AI Token Meter 自己的 WebKit 会话：

- 不读取 Safari、Chrome 或其他浏览器 Cookie；
- 只允许官方 DeepSeek 平台来源参与用量捕获；
- 登录 Cookie 由 WebKit 自己的数据存储管理；
- 业务缓存不保存 Cookie、授权头、登录字段或网页原始响应；
- 单个被处理的响应受大小限制，防止无限负载进入应用；
- 仅标准化保存日期、成本、请求数、Token 数和更新时间。

App 内网页仍属于第三方官方站点，其隐私和账户安全受 DeepSeek 官方条款约束。共享设备上应使用独立的 macOS 用户账户，并在不再使用时退出登录或清理应用数据。

## 本地数据

| 数据 | 位置/机制 | 生命周期 |
| --- | --- | --- |
| DeepSeek API Key | macOS Keychain | 用户点击 Remove 或删除对应 Keychain 项目 |
| Settings 账户身份/Key 遮罩 | 仅 App 内存 | Settings 重查、应用退出或进程结束 |
| 界面与监控偏好 | `UserDefaults` | 直到用户清理偏好 |
| 最近成功用量快照 | `Application Support/AI Meter` | 新快照覆盖或用户删除缓存 |
| Codex 本机活动聚合 | 随统一快照写入 `Application Support/AI Meter` | 新快照覆盖或用户删除缓存 |
| Claude 本机活动聚合 | 随统一快照写入 `Application Support/AI Meter` | 新快照覆盖或用户删除缓存；只含日期、计数和模型 ID |
| DeepSeek 每日聚合 | `Application Support/AI Meter` | 新数据覆盖或用户删除缓存 |
| DeepSeek 登录会话 | App WebKit 数据存储 | 退出登录或清理应用网站数据 |
| Widget 展示快照 | 签名双方专用 App Group | 主应用刷新覆盖；只含脱敏展示字段 |

显示名称迁移不会改变安全身份：Bundle Identifier 仍为 `com.millerpan.AIMeter`，可执行文件仍为 `AIMeterApp`，既有 Keychain 项目与 `Application Support/AI Meter` 兼容目录继续使用，避免重新暴露或复制密钥与会话数据。

## 敏感文本清理

进入缓存、状态消息或通知前，文本会清理常见敏感形态，包括：

- `Authorization: Bearer ...`；
- 常见 API Key 前缀和长 Token；
- 可能随 HTTP 错误返回的授权内容。

清理是纵深防御，不应取代“不记录原始敏感响应”的设计原则。

## Widget 隐私边界

- Widget 进程不发网络请求、不运行 Claude/Codex CLI、不读取 DeepSeek Keychain，也不访问 WebKit 登录会话；
- 主应用在刷新成功或本地基准变化后构建最小展示快照，再请求 WidgetKit 更新；
- 共享 JSON 不包含 API Key、Bearer Token、Cookie、邮箱、手机号、登录表单、CLI 原始输出或 Codex 重置券兑换 ID；
- Widget 只显示重置券数量和最近到期时间，不提供“立即使用”或自动兑换；
- App Group 只有使用同一 Apple Team ID 和一致 entitlement 签名的主应用与扩展可以访问。普通 ad-hoc 构建不会嵌入 Widget，也不会写入 App Group 配置。

## 网络边界

- Claude：由 Claude Code CLI 连接其官方服务；
- Codex：由 Codex CLI 及其 `app-server` 提供本地结构化账户数据；
- DeepSeek 余额：直接访问官方 API；
- DeepSeek 历史：App 内 WebKit 访问官方平台；
- AI Token Meter 没有自建遥测、广告或分析服务。

## 日志

系统日志用于记录可行动的运行状态，不应包含：

- API Key、Bearer Token、Cookie；
- 原始网页响应或完整 CLI 输出；
- 登录表单内容；
- 可识别个人身份的账户字段。

提交问题时仍应人工检查日志与截图并删除个人信息。

## 威胁与限制

- 本机已被恶意软件或高权限账户控制时，任何本地应用都无法完全保护会话；
- 上游 CLI 或官网接口变化可能导致暂时无法识别数据；
- ad-hoc 签名只用于本机开发，不提供公开发行所需的身份保证；
- 当前没有自动更新机制；升级必须由用户重新构建或替换应用；
- 项目尚未进行第三方安全审计。

## 报告安全问题

不要在公开 Issue 中粘贴密钥、Cookie、账户响应或可复现的未修复漏洞。请遵循仓库根目录的 [SECURITY.md](../SECURITY.md)。
