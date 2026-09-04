# 任务 3：Windows DeepSeek 官方历史同步状态

## RED

- 新增 Vitest 用例后，Windows 详情页缺少 `Opening official page…`、`Sync in progress`、失败提示与禁用按钮；同步状态也不会暂停自动隐藏。用例在最小测试导出后以这些缺失的可见行为失败。
- 覆盖的破坏点是：遗漏即时 `opening` 状态、重复触发同步、命令 reject 后无恢复入口、以及 `active`/终止状态未正确影响详情倒计时。

## GREEN

- `DetailSurface` 只订阅 `deepseek-history-status` 的固定小写枚举，并在点击时先置为 `opening`；`open_deepseek_history` 拒绝时转为 `failed`。
- `DeepSeekHistory` 在无历史时显示安全的同步进度或通用可重试错误；有历史数据的图表分支保持原样，不展示网页正文或任何敏感信息。
- `opening` 与 `active` 期间不创建详情自动隐藏计时器；`completed`、`cancelled` 或 `failed` 后恢复正常倒计时。

## 生产状态与计时器覆盖

- “Opening”用例断言点击后的状态文本和禁用按钮，并把假时钟推进超过默认 8 秒，确认详情仍存在。
- “Reject”用例让真实详情事件链收到命令拒绝，断言通用错误与可用的 `Try again` 按钮。
- `completed` 与 `cancelled` 分别从 `active` 事件推进，先验证 9 秒内详情仍在，再验证终止事件后的 8 秒倒计时关闭详情。
- 测试只在 Tauri 边界替换外部事件/命令；断言的是渲染出的生产组件状态和计时器结果，而非 mock 调用次数。

## 自审

- 仅修改 `windows/` 前端和需求台账；macOS 源码未变。
- 前端仅接受六个固定状态字面量，未知 payload 被忽略；错误信息不包含命令异常、网页内容、认证信息或历史正文。
- 同步状态只影响 DeepSeek 详情的自动隐藏；已有官方历史继续走既有汇总和图表分支。

## 文件与验证

- `windows/src/Shell.tsx`
- `windows/src/details/DeepSeekDetail.tsx`
- `windows/src/details/ProviderDetail.tsx`
- `windows/src/App.test.tsx`
- `docs/requirements-backlog.md`

已验证：`npm --prefix windows test`（2 个测试文件、19 项通过）、`npm --prefix windows run build`、`scripts/check-docs.sh`（145 份 Markdown）。

Git 提交：`fix(windows): surface DeepSeek history sync status (REQ-20260904-006)`。

## 审查修复第 1 轮

### RED

- `windows/src/App.test.tsx` 新增“第一轮迟到 reject 不覆盖第二轮 active”用例：旧实现把迟到 reject 无条件写成 `failed`，同步进度消失且自动隐藏重新开始，按预期失败。
- 同文件新增“一个监听注册失败后仍清理其余监听”用例：旧 `Promise.all(...).catch(...)` 丢弃已成功的 unlisten，卸载后两项成功监听均未被取消，按预期失败。
- 终态表扩展为 `completed`、`cancelled`、`failed`；`failed` 用例在旧实现已通过，记录为缺失回归保护而非新的生产缺陷。

### GREEN

- `DetailSurface` 为每次同步点击递增 `useRef` 尝试号；只有当前尝试的命令 reject 能将状态切为 `failed`，第一轮迟到 reject 不会覆盖第二轮的 `opening` 或 `active`。
- 三个详情监听独立注册；每个成功获得的 unlisten 都会在组件已卸载时即时调用，或在正常卸载时统一调用，任一兄弟监听失败不影响清理。
- 后端 `failed` 保持终态语义：不再暂停自动隐藏，8 秒后关闭详情。

### 覆盖与验证

- 覆盖测试文件：`windows/src/App.test.tsx`。
- 生产状态/计时器：迟到 reject 后断言仍显示 `Sync in progress` 并在假时钟推进 9 秒后详情仍可见；`failed` 和其余终态均断言恢复 8 秒关闭。
- 生命周期：部分注册失败测试在卸载后断言两个已成功注册的监听均被取消；Tauri 边界是唯一替身，详情与状态机仍使用真实 React 组件。
- 运行 `npm --prefix windows test`：2 个测试文件、22 项通过；运行 `npm --prefix windows run build`：TypeScript 检查和 Vite 生产构建通过；运行 `scripts/check-docs.sh`：145 份 Markdown 通过。

Git 提交：`fix(windows): harden DeepSeek history sync state (REQ-20260904-006)`。
