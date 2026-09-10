# Google Antigravity CLI 接入迁移：调研与待确认规格

关联需求：REQ-20260909-007。调研日期：2026-09-09。代码基线：`506092f`。

状态：用户于2026-09-10明确确认采用推荐方案，进入双平台实施。本文不构成发布授权。

## 已验证事实

Google 当前把 Antigravity CLI 作为原生 macOS、Linux 与 Windows 的终端产品，默认命令是 `agy`。官方安装页给出的默认位置是 macOS/Linux 的 `~/.local/bin/agy` 和 Windows 的 `%LOCALAPPDATA%\agy\bin`，并说明本地登录优先使用系统 Keychain 或 Credential Manager 中的既有会话。官方 `/usage`（别名 `/quota`）会主动刷新后端配额；官方 headless 文档明确要求把 `/usage` 作为独立的 `agy -p /usage` 命令执行，由 CLI 自身响应，而不是交给模型生成。

本机验证保持现有账号与配置不变，得到以下结果：

- `agy` 位于官方默认用户目录，版本为 `1.1.28`，是 arm64 Mach-O，签名 Team ID 为 `EQHXZ8M8AV`；
- `agy models` 成功返回当前可用模型清单，证明可执行文件和既有登录可用；
- `agy -p /usage` 退出码为 0，没有认证错误、模型轮次或输入/输出 token 元数据；
- 脱敏后的结构固定为四行：Gemini 模型和 Claude/GPT 模型各有 Weekly Limit 与 Five Hour Limit，字段为剩余百分比和 ISO 8601 重置时间；
- 探测输出在离开本机进程前已删除实际百分比、时间、邮箱与套餐信息，未保存到仓库、fixture 或截图。

核对来源：

- [Antigravity CLI 安装与认证](https://antigravity.google/docs/cli/install/)
- [Model Quotas `/usage`](https://antigravity.google/docs/cli/commands/usage)
- [Headless mode](https://antigravity.google/docs/cli/headless/)
- [CLI reference](https://antigravity.google/docs/cli/reference/)
- [Status line JSON 结构](https://antigravity.google/docs/cli/statusline/)
- [AI Credits 与额度](https://antigravity.google/docs/cli/credits/)

## 已确认的产品决定

### 推荐：第四项整体迁移为 Antigravity

用户可见名称改为 **Google Antigravity**，详情副标题使用 **Antigravity CLI quota**。展示 `/usage` 返回的全部四个额度窗口：

1. Gemini models · Five hour
2. Gemini models · Weekly
3. Claude and GPT models · Five hour
4. Claude and GPT models · Weekly

界面把 CLI 的“剩余百分比”转换成应用现有进度环所需的“已用百分比”，同时在额度卡片明确显示 Remaining，避免用户把环和文本理解为同一方向。重置时间使用 CLI 返回值，不推测请求总数或 token 上限。

选择这一方案的原因是 Antigravity 的实际额度跨越 Gemini、Claude 和 GPT 模型。继续只显示 Gemini 两行会隐藏同一账号的另一半官方额度，也会让产品名称与数据来源不一致。

### 不纳入首期的 AI Credits

AI Credits 是独立余额和可能产生费用的使用设置。当前 `/usage` 成功输出不包含 credits；官方把它放在 `/credits` 与独立设置中。首期不读取、不展示、不启用 credits，后续如需接入另立需求和口径。

## 兼容与迁移

现有持久化 provider ID `gemini` 暂时保留，只改变用户可见名称和采集实现。这样可保留用户的显示顺序、隐藏选择、缓存与 Widget 数据，避免升级后出现第五个重复按钮或偏好丢失。新写入仍使用同一 ID；文档明确该 ID 只是兼容键，不代表当前接入仍是旧 Gemini CLI。

旧 `gemini` 可执行文件不作为自动回退。发现逻辑改查 `agy` 和官方平台路径；旧 Gemini 缓存只可作为带时间戳的 stale 数据参与一次兼容迁移，下一次 Antigravity 成功采集后覆盖。安装与登录引导全部改为官方 Antigravity 原生脚本、`agy` 命令和系统安全凭据说明。

## 采集与安全边界

双平台采集器在专用空目录中执行固定参数 `agy -p /usage`，不发送自然语言，不执行工具，不自动登录、退出或修改配置。环境仅保留 CLI 恢复系统安全会话和联网所需的最小变量；拒绝 API Key、代理注入、调试和自定义执行覆盖。stdout 设置尺寸上限和超时，并在任何日志或错误展示前先做邮箱、路径、百分比、时间和令牌样式脱敏。

首个现场兼容版本是 1.1.28。实现按结构而非硬编码单一补丁版本验收：同一主版本只有在完整表头、唯一分组、唯一时间窗口、0 至 100 的 remaining 值和合法重置时间全部满足时才接受；未知列、重复行、缺行、混合方向或未来主版本均失败关闭并保留最后成功缓存。不会把错误、空表或只返回模型清单误报为 0% 用量。

`/usage` 会访问官方后端刷新额度，但不会发起模型生成。本机能力探测已经证明该路径在既有登录下无模型轮次元数据；产品测试仍使用脱敏合成输出，真实账号只用于用户主动授权的现场复验。

## 双平台实施范围

macOS 将替换 Swift 的 Gemini 发现、环境、PTY 对话框和解析路径，优先使用普通有界进程读取 headless 表格。Windows 将替换 Rust 的 Gemini 发现、ConPTY 会话和解析路径，同样使用有界隐藏进程；若官方 Windows headless 行为与 macOS 不一致，保持 unavailable 并等待原生 CI 或现场证据，不退回模拟成功。

两端同步修改可见名称、详情四张额度卡片、Settings 安装/登录说明、恢复动作、固定官方链接、可访问名称、Widget 文案、共享合同和文档。Logo 是否使用 Antigravity 官方资产需在实现阶段核对授权来源；没有可分发证据时先沿用现有 Google 四角星图形并只改文字，不下载来历不明的品牌图。

## 验收口径

1. 升级后只有一个第四服务按钮，原有显示顺序和隐藏选择不丢失，所有可见位置统一叫 Google Antigravity。
2. macOS 与 Windows 都能发现官方 `agy`；未安装、需登录、版本或输出不兼容、网络失败各有准确状态和恢复入口。
3. 合成测试覆盖四行成功、顺序变化、0/100 边界、缺行、重复行、未知列、非法时间、认证失败、超时、超量输出和取消清理。
4. 进度环继续表示已用比例，额度卡明确表示剩余比例与重置时间；Gemini 和 Claude/GPT 两组不会互相覆盖。
5. 采集过程无模型提示、工具执行、账号配置写入、OAuth/Keychain 内容读取或公开日志中的个人额度。
6. 运行文档检查、双平台完整测试、原生 CI 与独立审查。真实 Windows GUI/DPI 和不同账号套餐仍保留为现场边界，不由夹具冒充通过。
