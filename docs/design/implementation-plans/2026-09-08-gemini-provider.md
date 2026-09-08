# Gemini 四产品接入实施计划

> 面向 AI 代理的工作者：使用 subagent-driven-development 分任务实现与审查；执行中证据更新本计划，唯一需求状态仍在 requirements-backlog.md。

**需求：** REQ-20260908-012；完成后自动接续015。
**目标：** 双平台提供青绿色 Gemini 按钮及可靠的显示/排序迁移，只有通过无模型调用可行性验证的官方CLI额度才进入用量环。
**架构：** 沿用Swift/Rust采集、共享快照、React/SwiftUI展示和持久化。CLI原生输出为候选额度来源，不复制OAuth私有后端调用；采集失败不产生虚假百分比。
**技术栈：** Swift Testing/SwiftUI、Rust/serde、TypeScript/React/Vitest；Node隔离探测夹具。
**授权：** 用户明确批准#3ED6B2；CLI优先与旧用户默认隐藏由既有非重大技术决策授权支持。真实账户、安装覆盖、发布和推送未授权。无需例行重问。
**规格：** [Gemini调研与设计](../specifications/2026-09-08-gemini-provider-design.md)。

## Task 1: 固定官方CLI基线并验证无推理查询边界

**所有权：** `scripts/test-gemini-cli-capability.mjs`、`docs/development/2026-09-08-gemini-cli-capability.md`；临时上游源码与探测产物在`/private/tmp/req012-gemini-*`。不改产品代码或需求台账。

- [ ] 固定官方稳定版v0.58.0，源码提交ac9431c9e2290d68af31a77614ff2fddb2391ca3，下载目录`/private/tmp/req012-gemini-source`。核对该源码而非滚动main。
- [ ] 追踪`statsCommand.ts`、`modelCommand.ts`、实际StatsDisplay/ModelStatsDisplay与model对话框、nonInteractiveCliCommands/loader，明确空会话的最终可见字段和非交互参数含义。
- [ ] 先建立能否决错误接入假设的隔离探测，禁止凭据读取和真实网络。运行原始内置命令/渲染代码时仅用测试适配器提供合成quota和账户，无真实HOME；若无法覆盖整个启动链，不把命令级验证宣称为端到端认证通过。

```js
// 探测必须涵盖这些性质；按上游真实接口接入夹具。
assert.equal(modelGenerationCalls, 0);
assert.equal(secretReads, 0);
assert.equal(actualVisibleQuota, expectedVisibleQuota);
// 错误假设（例如新会话model视图一定有额度）应被真实上游行为否决。
```

- [ ] 记录可复现运行入口、上游固定SHA、原始输出、命令级/渲染级/启动链各自覆盖边界。对于headless未支持命令，证明是拒绝还是转模型，不能猜测。
- [ ] 结论必须区分：已验证无模型调用并可见额度；命令内部刷新但不显示；来源存在而自动采集未证实。若不可行，立即回传根代理；不通过制造模型调用补齐、不直接调私有后端。
- [ ] 用文档索引链接报告，运行文档检查；提交该任务，独立审查证据和结论。后续采集器仅依赖被验证的路径。

## Task 2: macOS四产品按钮与偏好迁移

**所有权：** `Sources/`、`Tests/`中Swift实现与测试，Gemini品牌资源；不修改Windows、共享合同和台账。默认数据类型用Gemini枚举，注册阶段缺额度时必须unknown/unavailable且primaryMetric为nil。

- [ ] 先在`FloatingStripPreferencesTests.swift`添加高度4项/旧JSON迁移/全新默认/重复未知值/至少一项可见/重启往返测试；先运行失败。

```swift
#expect(FloatingStripDensity.compact.height(providerCount: 4) == 344)
#expect(FloatingStripDensity.comfortable.height(providerCount: 4) == 428)
// 旧三项JSON：保留已有顺序和隐藏集合，追加gemini且隐藏。
// 新初始化：四项均显示；再次保存/载入遵从用户选择。
```

- [ ] 扩展`Domain/UsageModels.swift`和所有必要switch、资源/调色板、详情分派、Demo与widget/菜单必要兼容点；Gemini强调色固定#3ED6B2，Logo使用可核对来源资产并记来源。尚未验证采集能力时明确状态，不能Demo数值流入真实运行。
- [ ] 高度使用`firstHeight + (boundedCount - 1) * rowHeight`，保证1..4旧尺寸不变；迁移使用版本标记，区分无保存值与旧JSON。未知新provider不能破坏旧缓存整体读取。
- [ ] 保留现有设置上下移动、拖放、恢复默认及隐藏仍监测语义。覆盖24个排列与15个非空集合、两密度及左右镜像；新增原生视图渲染测试第四项完整可点击、Logo不镜像、展开态无横线。
- [ ] 定向Swift测试绿灯并提交，报告失败/通过日志与未验证的原生交互边界；独立任务审查。

## Task 3: Windows四产品按钮与偏好迁移

**所有权：** `windows/`、`contracts/`、相关跨平台合同脚本；不改Swift和台账。与Task2共享公开provider ID `gemini`、显示名Gemini和#3ED6B2。Task2/3只在共同合同基线明确后可并行。

- [ ] 先为`strip_preferences.rs`增加4项尺寸、旧serde数据迁移、新默认、重复未知项、保存重载与至少一项可见测试；前端Settings测试第四项名称/末项移动/取消第三项而仍有一项可见。

```ts
expect(screen.getByLabelText('Move Gemini down')).toBeDisabled()
// 第3项不是末项；隐藏2项时仍能继续隐藏第3项，最后可见项才锁定。
```

- [ ] 扩展Rust/TypeScript的provider类型、快照/可用状态、详情路由/注册、Logo资产、共享presentation合同。修正Settings名称三分支和index===2等固定上限。
- [ ] 原生窗口和前端共同使用1至4动态尺寸，Rust不得以usize执行3-count。新旧偏好迁移与Task2一致；同步窗口尺寸、裁切和点击区域。
- [ ] 定向前端/Rust测试先失败再通过，运行前端构建与四按钮左右/两密度浏览器检查；跨平台合同保持旧fixture兼容并增加Gemini缺失态样本。提交并独立任务审查。

## Task 4: 可验证采集接入与完整交付

本任务以Task1输出为前置。没有通过验证的无模型调用路径时，不编写假实现，先回传真正的范围分歧；Task2/3已授权内容仍可完成，但不能称012全部完成。

- [ ] 可行时按Task1固定协议先写真实解析器正反例：已用/剩余、缺上限、缺重置时间、多个池、空输出、版本不匹配、认证/网络/权限错误；校验归属和额度范围。
- [ ] 在Swift/Rust既有进程生命周期框架中接入受支持命令，隔离工作目录、关闭工具/自动信任、固定超时、可取消、退出清理；保留最后成功样本及新鲜度。未知能力不触发重装/重登录。
- [ ] 本机活动必须独立标注设备覆盖与字段范围；如无必要不新增历史分析UI。不能用本机token除套餐请求数，不能把缓存统计绑定到无法证明的当前账户。
- [ ] 更新详细协议/测试计划及开发日志后运行文档门禁，再运行相关平台完整测试、Release/前端构建和跨平台合同门禁；真实Windows/账号未执行部分独立记录。
- [ ] 分支级独立审查、必要修复与定向复审；更新台账，安全整合主工作区的新需求，Git本地合并收尾。未发布、未推送。
- [ ] 回传实现/验证/审查/合并/边界与后续所有者；只有Gemini功能完成才转入015。若额度受限，准确标记并等待重大范围决定，不擅自完成012或提前执行015。
