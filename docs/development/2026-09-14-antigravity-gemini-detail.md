# Google Antigravity Gemini 优先详情开发记录

## 范围

REQ-20260914-001 将 Google Antigravity 从四窗口混合展示改为 Gemini 优先紧凑详情。Antigravity 的主次指标、悬浮条圆环、菜单/托盘摘要、提醒和缓存统一只使用 Gemini Five hour 与 Weekly；独立 Claude Code、OpenAI Codex Provider 不受影响。

详情保留青绿色强调和深色底板，显示两个 Gemini 额度窗口与重置时间，并在可用时补充：

- 当前 Gemini 模型；
- 动态可用 Gemini 模型数量与去重系列；
- Antigravity CLI 版本；
- 本次检查时间。

首版不显示 AI Credits，不推测套餐、30 天统计或当前第三方模型。

## 数据与容错

Swift 与 Rust 采集器继续验证官方 `/usage` 的完整允许形态，但只发布两个 Gemini 窗口。旧四窗口缓存加载时重新计算 Gemini-only 主次指标与比例；不完整旧缓存不保留可见额度。

额度成功后才分别运行有界只读 `/model` 与 `models`。两条补充命令彼此独立，失败、超时、异常行或第三方模型名称只省略对应信息，不能把成功额度降级。共享 JSON 使用显式 `antigravityCLIInfo` 字段；Rust 端显式固定该大小写，避免自动驼峰把 `CLI` 改成 `Cli`。缓存加载会再次移除越界、重复、第三方或会被隐私清洗器改写的模型信息。Windows 生产采集使用同一可注入编排，直接验证额度失败短路、补充命令独立失败及取消传播。

## 失败先行证据

- Swift `/usage`、缓存归一化测试先在旧实现中报告 14 个预期问题；Rust 缓存测试先报告 2 个预期失败。
- 新增 CLI 信息解析与采集测试先分别因缺少 Swift 字段/解析器、Rust 解析函数与参数构造而失败编译。
- 详情合同测试先因缺少 CLI 信息展示与新高度参数失败编译。
- 缓存第三方模型回归先在 Swift 与 Rust 各失败一次，证明旧加载路径会保留异常信息；增加二次归一化后转绿。
- 独立审查的合成探针先证明邮箱、路径、密钥、手机号及异常词形可能被接受，并指出旧迁移测试没有真实读取旧格式、Windows 生命周期测试没有覆盖生产编排；补充失败先行回归后统一了双平台解析、缓存和 JSON Schema 边界。

## 验证

- Swift 定向解析、缓存、共享合同、详情布局与三语言本地化通过；仓库正式门禁按既定隔离策略完成 562 + 3 + 18，共 583 项测试。
- Windows 18 个前端测试文件、133 项测试、TypeScript 类型检查和 production Vite build 通过。
- Rust 268 项完整测试、格式检查和 `clippy --all-targets -D warnings` 通过。
- JSON Schema 对新 fixture 校验通过；跨平台合同与篡改拒绝脚本通过。
- 真实 Chrome 密度门禁通过 25 项进程生命周期测试与 8 组 Antigravity fresh/cached/authenticationRequired/unavailable 场景，确认仅两项 Gemini 额度、CLI 信息、青绿色强调和窄宽度无裁切。
- macOS 详情真实 SwiftUI 位图渲染通过：两个额度窗口与 CLI 信息卡完整可见，青绿色强调和深色不透明底板保持。
- `scripts/check-docs.sh` 通过 313 份 Markdown 文档，公开安全门禁通过。

独立审查分三轮：首轮 `0/3/2`，第二轮 `0/2/1`，修正全部发现后最终为 **Critical 0 / Important 0 / Minor 0**。候选证据提交为 `4aa7b09`，已本地合入最新 `main`，合并提交为 `e1f43b6`。协调入口原有未提交台账先保存到本地 stash，并确认其中的新事项与历史状态已由合并后的唯一台账完整覆盖。未推送、未建 Tag、未公开发布。
