# Antigravity 显式恢复自动刷新实施计划

> 面向执行Agent：使用 subagent-driven-development 按本计划分工，所有执行/审查显式 gpt-6.1-sol/high。root承担最终决策、验证、Git和发布。

目标：实现[已明确的恢复契约](../specifications/2026-10-26-antigravity-explicit-resume-design.md)，保持旧暂停默认关闭。用户已授权003优先、正常门禁后直接发布，再做004；不重复请求普通开发/发布授权。

## 文件与职责

- Core：`Sources/AIMeterCore/Coordination/RefreshCoordinator.swift`，凭证、代际、单调期限、同步事务和自动quota-only路由。`Collectors/GeminiCollector.swift`仅将既有严格版本判断供Core内部复用，不扩版本。
- Core测试：`Tests/AIMeterCoreTests/RefreshCoordinatorTests.swift`、`GeminiCollectorTests.swift`，真实actor持久化/并发/重启及合成CLI边界。
- App：`Sources/AIMeterApp/AppModel.swift`，凭证任务/会话及UI状态；`Views/GeminiDetailView.swift`、`ServicesSettingsView.swift`、`FloatingStripView.swift`传递动作；三语言strings。
- App测试：现有recovery、Services及detail测试，验证明确动作、stop/新会话拒绝迟到结果和原登录/清理防线不变。
- 文档：本规格/计划、唯一台账、开发记录及索引；发布版本与说明在功能GREEN后处理。

## Core API契约

`GeminiAutomaticRefreshResumeAuthorization`为Sendable/Equatable的单次能力，内部UUID初始化，仅Core生成。`GeminiAutomaticRefreshResumeError`固定case：validationRequired、alreadyRunning、persistenceFailed、notPaused。

`geminiAutomaticRefreshResumeAuthorization(for authorization: GeminiQuotaRecoveryAuthorization) -> GeminiAutomaticRefreshResumeAuthorization?`只返回与参数中的已消费quota授权ID精确匹配、本次新鲜有效检查后的未过期能力；迟到旧回调不能拿到新检查的能力。`resumeGeminiAutomaticRefresh(authorization:) throws -> UsageSnapshot`是无内部await的actor提交；busy不消费能力，成功或保存失败消费。`invalidateGeminiAutomaticRefreshResumeAuthorization()`撤销能力并推进代际。Core可注入只用于恢复TTL的Sendable单调Instant时间源，默认ContinuousClock.now的Instant；无真实等待五分钟的测试。

### Task 1: 失败先行与Core

- [x] 先写actor回归：制造Gemini持久暂停，按原登录token授权一次新鲜支持版本quota，确认仍暂停；取得新能力后明确调用恢复，要求动作集不再包含Gemini、重启后可自动取额且其他provider退避不变。先搭最小拒绝型API契约使测试可执行，保留未实现行为为真实RED；不先编写放行逻辑。
- [x] 静态检查/窄审后提交测试与契约检查点，用既有DraftPR正常macOS CI观察具名行为RED，完整保留编译/测试输出；编译错误不能冒充行为RED。
- [x] RED成立后只实现内存能力、有效性撤销与同步事务。保存失败、过期、错误ID、重放、取消、重启、缓存/不支持版本都不能放行。
- [x] 自动刷新只调用quota-only。用具备collect与quotaOnly独立计数的受控collector证明路由；缺少协议必须失败暂停，collect计数零。
- [x] 用other-provider gate持有完整refresh；busy时原暂停不变，等FULLrefresh返回后同一有效能力明确重试成功，lastPresented不回写旧暂停。任何新Gemini失败或新恢复均使旧能力失效。

### Task 2: App与界面

- [x] 按上述API接入可注入获取能力/恢复/撤销操作，保持现有一次检查语义、exec确认与清理不确定拒绝。
- [x] 回归：未检查时无恢复、检查成功不自动恢复、显式恢复后去角标、重复点击不多提交；busy提示可手动重试，保存/验证失败保持暂停。stop取消待提交task、撤销能力，晚到结果不得覆盖新会话；已提交的明确选择不回滚。
- [x] 详情和Services复用同一能力/动作，不增加拥挤按钮；英语/简体/繁体均给出未检查、可恢复、忙、保存失败、已恢复的真实文案。
- [x] 真实非交互假CLI认证负例：无TTY时返回authenticationRequired，不执行interactive/sign-in或model/catalog路径，不调用真实浏览器。保留64KiB、8s/30s、原生产清理及3秒期限。

### Task 3: 验证与交付

- [x] root全文读取差异/真实RED与GREEN证据，独立Sol源码及实际结果审查，无阻塞后正常合并。当前基线main940已有完整双GREEN，不在宿主重跑旧717或旧GUI。
- [x] 正常PR/发布门禁依新增差异执行，所有组/跳过/失败/未运行如实记录；不盲重试、延长原期限或删除断言。
- [x] 按仓库原脚本版本递增、签名打包、正式双平台发布工作流和匿名固定下载/三更新源验收；仅标准授权公开文件，不带私有环境/现场凭据或详细本机日志。
- [x] 更新唯一台账、开发记录与A回传。产品发布与真实设备恢复分开；无新阻塞不重复索取已回答信息或普通授权。完成003实现发布后接续004，001/002仅保留检查点。
