# Codex 详情与 DeepSeek 自动同步开发日志

日期：2026-08-31  
分支：`codex/codex-deepseek-details`

## 背景与目标

本阶段继续完成审计清单中的 1、2、4：实现已确认的 Codex “额度优先 + 三项本机统计”详情；修复 DeepSeek 已登录官网数据不能自动进入原生 30 天图表的问题；同步过期文档。

## 问题证据

- Codex 详情只有官方窗口与充值券，批准的本机三项统计仍停留在视觉原型；
- DeepSeek 官方 `/usage` 页面能够登录并显示数据，但 `deepseek-usage-history.json` 不存在；
- 旧解析器只识别“日期、成本、请求、Token 位于同一对象”的结构；官网已把按 API Key 的每日 Token/请求与每日费用拆成两个响应；
- README、测试指南、开发索引和提交历史仍停留在焦点修复以前的状态。

## 实现

### Codex

1. 新增 `CodexLocalActivityReader`，以只读 SQLite 查询访问 `~/.codex/state_5.sqlite`。
2. SQL 只选择 `tokens_used`、`created_at`、`updated_at` 三列，并限定最近 30 天更新的线程。
3. 计算 Token 总量、允许截至昨天的当前连续活动天数，以及最长非负线程持续时间。
4. 本机读取失败时返回 `nil`，官方 `app-server` 额度仍照常成功。
5. 新建专用 `CodexDetailView`：官方额度和重置券在上，本机三项统计与来源说明在下。

### DeepSeek

1. 适配官网当前 `/usage/by_api_key/amount` bucket：累加缓存命中、缓存未命中、输出 Token 和请求数，忽略 API Key 身份对象。
2. 适配 `/usage/by_api_key/cost` bucket：按日累加 CNY 实际费用。
3. 新增双分片累加器；每种响应按最新一份替换，只有 amount 与 cost 都存在时才合并、补齐 30 天并覆盖缓存。
4. 缓存仍只含日期、成本、请求数、Token 数和更新时间，不含网页原始响应、Cookie、Token 或 API Key 名称。

## 测试驱动证据

- 新测试先因 Codex 本机模型/汇总器和 DeepSeek 分片累加器不存在而编译失败；实现后定向测试通过。
- 新增当前官网 amount/cost 最小去敏 fixture、跨模型/Key 聚合、完整分片门控、本机窗口/连续天数/持续时间、数值行解析和界面文案测试。
- 完整测试：110 个测试、25 个套件、0 失败；普通运行按设计跳过 1 个 Keychain 与 3 个真实 CLI 检查。
- 显式开启真实 Codex 冒烟测试后，官方额度与本机活动摘要同时返回成功。

## 真实官网与安装验收

- 已登录的 AI Meter WebKit 会话自动生成固定 30 天历史缓存；缓存中的成本、请求数和 Token 总量与同一时刻官方页面三项汇总一致。
- 验收只比较聚合结果，不保存登录 Token、Cookie、手机号、验证码、API Key 身份或原始响应。
- release 构建、Info.plist、ad-hoc 签名和安装版/构建版可执行文件校验全部通过。
- 安装前旧 App 已备份到 `/private/tmp/AI Meter.app.pre-codex-deepseek-20260831-003100`。

## 安全与隐私复核

- Codex 只读查询不接触 `title`、`preview`、`first_user_message` 或任何对话正文；
- DeepSeek 分片解析递归深度、记录数和负载大小均有限制；
- 只有官方 HTTPS 主机和 WebKit 官方 frame 可以提交响应；
- 不完整同步不会覆盖最近一次完整历史；
- 临时验收代码和无敏感诊断记录均不进入最终提交或安装包。

## Git 节点

- `827ff87`：设计规格与实施计划；
- `be5a778`：Codex 本机详情与 DeepSeek 分片同步功能；
- 文档、最终安装与合并节点见 [提交历史](commit-history.md)。

## 已知边界

- Codex 本机统计是当前 Mac 的线程级估算，不等同于 OpenAI 跨设备账户统计；旧线程在窗口内更新时，`tokens_used` 无法按日拆分。
- DeepSeek `/api/v0/usage/*` 属于官网未公开内部接口；未来字段变化时会保留完整缓存并显示降级状态，不影响余额 API。
