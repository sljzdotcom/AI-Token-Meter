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

- [x] 固定官方稳定版v0.58.0，源码提交ac9431c9e2290d68af31a77614ff2fddb2391ca3，下载目录`/private/tmp/req012-gemini-source`。核对该源码而非滚动main。
- [x] 追踪`statsCommand.ts`、`modelCommand.ts`、实际StatsDisplay/ModelStatsDisplay与model对话框、nonInteractiveCliCommands/loader，明确空会话的最终可见字段和非交互参数含义。
- [x] 先建立能否决错误接入假设的隔离探测，禁止凭据读取和真实网络。运行原始内置命令/渲染代码时仅用测试适配器提供合成quota和账户，无真实HOME；若无法覆盖整个启动链，不把命令级验证宣称为端到端认证通过。

```js
// 探测必须涵盖这些性质；按上游真实接口接入夹具。
assert.equal(modelGenerationCalls, 0);
assert.equal(secretReads, 0);
assert.equal(actualVisibleQuota, expectedVisibleQuota);
// 错误假设（例如新会话model视图一定有额度）应被真实上游行为否决。
```

- [x] 记录可复现运行入口、上游固定SHA、原始输出、命令级/渲染级/启动链各自覆盖边界。对于headless未支持命令，证明是拒绝还是转模型，不能猜测。
- [x] 结论必须区分：已验证无模型调用并可见额度；命令内部刷新但不显示；来源存在而自动采集未证实。若不可行，立即回传根代理；不通过制造模型调用补齐、不直接调私有后端。
- [x] 用文档索引链接报告，运行文档检查；提交该任务，独立审查证据和结论。后续采集器仅依赖被验证的路径。

## 共同接口与并行边界

Task2/3独立于Task1的额度结论，先交付可测试的按钮/配置扩展，不能宣称完整Gemini采集。两端统一偏好`schemaVersion: 2`、provider ID `gemini`、全新四项显示、旧记录追加隐藏；没有版本但已经包含gemini的记录保留其显示选择。共享合同新增Gemini unavailable fixture，`usedRatio`和primaryMetric均缺失/null，不混入Demo额度。macOS拥有Swift合同测试，Windows拥有contracts和Ruby合同门禁；二者集成前可以注明共同合同尚未落地的已知依赖，最终必须全部验证。UI阶段Gemini未知账号不可提供实际安装/登录执行按钮，只提供官方文档与重试/状态说明，Task4验证路径后再接真实认证动作。原生widget未作为本轮新功能推广，但枚举编译与旧数据兼容必须通过。

## Task 2: macOS四产品按钮与偏好迁移

**所有权：** `Sources/`、`Tests/`中Swift实现与测试，Gemini品牌资源；不修改Windows、共享合同和台账。默认数据类型用Gemini枚举，注册阶段缺额度时必须unknown/unavailable且primaryMetric为nil。

- [x] 先在`FloatingStripPreferencesTests.swift`添加高度4项/旧JSON迁移/全新默认/重复未知值/至少一项可见/重启往返测试；先运行失败。

```swift
#expect(FloatingStripDensity.compact.height(providerCount: 4) == 344)
#expect(FloatingStripDensity.comfortable.height(providerCount: 4) == 428)
// 旧三项JSON：保留已有顺序和隐藏集合，追加gemini且隐藏。
// 新初始化：四项均显示；再次保存/载入遵从用户选择。
```

- [x] 扩展`Domain/UsageModels.swift`和所有必要switch、资源/调色板、详情分派、Demo与widget/菜单必要兼容点；Gemini强调色固定#3ED6B2，Logo使用可核对来源资产并记来源。尚未验证采集能力时明确状态，不能Demo数值流入真实运行。
- [x] 高度使用`firstHeight + (boundedCount - 1) * rowHeight`，保证1..4旧尺寸不变；迁移使用版本标记，区分无保存值与旧JSON。未知新provider不能破坏旧缓存整体读取。
- [x] 保留现有设置上下移动、拖放、恢复默认及隐藏仍监测语义。覆盖24个排列与15个非空集合、两密度及左右镜像；新增原生视图渲染测试第四项完整可点击、Logo不镜像、展开态无横线。
- [x] 定向Swift测试绿灯并提交，报告失败/通过日志与未验证的原生交互边界；独立任务审查。

## Task 3: Windows四产品按钮与偏好迁移

**所有权：** `windows/`、`contracts/`、相关跨平台合同脚本；不改Swift和台账。与Task2共享公开provider ID `gemini`、显示名Gemini和#3ED6B2。Task2/3只在共同合同基线明确后可并行。

- [x] 先为`strip_preferences.rs`增加4项尺寸、旧serde数据迁移、新默认、重复未知项、保存重载与至少一项可见测试；前端Settings测试第四项名称/末项移动/取消第三项而仍有一项可见。

```ts
expect(screen.getByLabelText('Move Gemini down')).toBeDisabled()
// 第3项不是末项；隐藏2项时仍能继续隐藏第3项，最后可见项才锁定。
```

- [x] 扩展Rust/TypeScript的provider类型、快照/可用状态、详情路由/注册、Logo资产、共享presentation合同。修正Settings名称三分支和index===2等固定上限。
- [x] 原生窗口和前端共同使用1至4动态尺寸，Rust不得以usize执行3-count。新旧偏好迁移与Task2一致；同步窗口尺寸、裁切和点击区域。
- [x] 定向前端/Rust测试先失败再通过，运行前端构建与四按钮左右/两密度浏览器检查；跨平台合同保持旧fixture兼容并增加Gemini缺失态样本。提交并独立任务审查。

## Task 4: 可验证采集接入与完整交付

本任务以Task1输出为前置。没有通过验证的无模型调用路径时，不编写假实现，先回传真正的范围分歧；Task2/3已授权内容仍可完成，但不能称012全部完成。

- [x] Task4a先做独立启动链探测：仅在`/private/tmp/req012-gemini-startup`准备固定官方npm CLI v0.58.0与测试依赖，禁用安装生命周期脚本，不修改用户CLI。真实子进程/PTY使用隔离用户目录、合成认证文件和系统设置，所有外部网络/模型端点默认拒绝并记录，账户/加载/额度响应仅通过进程内测试夹具返回。放入可观察但无害的hooks/MCP/extensions哨兵，验证官方设置确实禁止它们启动；不能只断言配置文本。验证全新会话`/model`可见额度、退出/超时、缺认证/失败状态。请求/文件/子进程边界必须显式列出，禁止读取原用户目录或系统凭据。脚本与记录为`test-gemini-cli-startup.mjs`和开发报告附节；该合成环境不能冒充真实账号验收。发现需无法可靠限制的启动副作用时给出证据并停止依赖实现。
- [x] 可行时按Task1固定协议先写真实解析器正反例：已用/剩余、缺上限、缺重置时间、多个池、空输出、版本不匹配、认证/网络/权限错误；校验归属和额度范围。
- [x] 在Swift/Rust既有进程生命周期框架中接入受支持命令，隔离工作目录、关闭工具/自动信任、固定超时、可取消、退出清理；保留最后成功样本及新鲜度。未知能力不触发重装/重登录。
- [x] 本机活动必须独立标注设备覆盖与字段范围；如无必要不新增历史分析UI。不能用本机token除套餐请求数，不能把缓存统计绑定到无法证明的当前账户。
- [x] 更新详细协议/测试计划及开发日志后运行文档门禁，再运行相关平台完整测试、Release/前端构建和跨平台合同门禁；真实Windows/账号未执行部分独立记录。
- [ ] 分支级独立审查、必要修复与定向复审；更新台账，安全整合主工作区的新需求，Git本地合并收尾。未发布、未推送。
- [ ] 回传实现/验证/审查/合并/边界与后续所有者；只有Gemini功能完成才转入015。若额度受限，准确标记并等待重大范围决定，不擅自完成012或提前执行015。

## 实施检查点记录

2026-09-08：Task1已提交`8bf76ec`并通过独立审查（无阻断项）；Task2为`f3a3428`，Task3为`7b7ec5e`。Task2独立审查通过；Task3审查及集成回归修复仍在进行。前端105项/13文件与生产构建通过，文档199份通过。首轮宿主Rust在共享元数据仅允许三产品处失败；首轮Swift436项中的刷新协调器测试仍期望三个模拟采集器，出现两条断言失败。另发现共享snapshot schema名称漏加Gemini；均纳入012当前修复，不宣称完整回归通过。Task4a仍在验证真实CLI合成环境启动、退出、认证失败与用户扩展隔离，尚未接入产品采集器。

修复后复验：`2124b78`关闭Windows元数据/schema两项P2，宿主Rust225项与严格Clippy通过；`8c632db`修复Swift四并发测试期望，436项主测试+13项PTY及全部仓库门禁通过，macOS Release编译通过。证据见[展示阶段开发日志](../../development/2026-09-08-gemini-presentation.md)。

## Task4b/4c：受支持环境的额度采集接口

Task4a固定证据为`bbf701b`及能力报告附节。此协议须在其独立审查通过后执行；真实账号/原生Windows未验证仍独立列为现场边界，不泛化为所有配置支持。

- 仅官方Gemini CLI 0.58.0，通过固定命令/交互协议；版本未知或不符不启动额度交互，准确返回不支持版本。不得调用headless -p /stats，不提取OAuth、不自行请求私有后端。
- CLI保持其原有用户目录自行读取账号；首期只支持已验证的普通OAuth模式，API/Vertex/加密或注入配置等未证实模式停止并准确标记不支持，不读取token内容。先检测官方默认系统settings/defaults与自定义环境路径；存在原配置但没有经过验证的保留方法时明确unavailable，不覆盖企业限制。只在没有此类配置的受支持环境提供隔离cwd、空.env、私有system settings/defaults覆盖，NO_BROWSER=true；使用已验证的hooksConfig/扩展/随机MCP允许名称/隐私与更新关闭组合。不能依赖测试Node26权限或nock才能保证生产路径安全，不复制synthetic HOME/credential逻辑进入产品。
- 分步PTY状态机：隔离环境version预检→等待真实输入就绪或认证/信任/主题等阻塞态→仅逐键/model→完整稳定Model usage视图→Esc /quit或有界取消。任何自然语言、授权码、菜单选择均禁止；超时/取消/异常必须清理子进程与PTY。
- 原始可见文本的Pro/Flash/Flash Lite百分比为已用，官方CLI已按tier归组并取组内最低remaining。保留可见档位，不声称知道原始模型或共享池。忽略启动footer聚合百分比。
- 两端新增可选geminiQuotaMetrics数组，元素沿用UsageMetric（label为可见档位，current为0..100已用整数百分比，limit100，unit percent，kind officialLimit，resetAt nil，resetDescription可选原样）。保留所有有效档位，primary选最高已用；同值固定显示顺序，secondary可选次高，仅详情数组为完整列表。
- CLI原文百分比舍入，展示不补造小数精度。缺重置不推算绝对日期。无quota/未知标题/越界百分比/不完整输出/前后冲突帧必须降级，不能假0或无限额。缓存失败保留最后成功数据及原fetchedAt。
- 账户状态以可观察CLI结果为依据，无凭据或401授权提示立刻停止、authRequired；API/Vertex模式或403/无quota应unavailable，不能误要求重装/登录。身份未输出就不编造email。安装/登录实际操作本轮禁止，UI官方指南和重试可用；生产user-triggered接入动作另按验证范围。
- 首期此检查点只实现官方额度，不添加本机历史token聚合。该数据缺失不影响官方额度；不得用活动统计替代额度。
- Windows owner拥有windows/、contracts/和合同脚本；Swift owner拥有Sources/、Tests/及Swift合同适配。共同schema仍1，可选新字段向后兼容；Gemini unavailable fixture保留，增加真实转录派生的quota fixture。root拥有设计/计划/台账/开发日志与集成。
- TDD优先正反解析、发现缺失/不可启动、环境覆盖与固定输入、拒绝headless、版本拒绝、noauth/异常/timeout/cancel清理、fresh/cache字段、详情多档位/无数据与settings状态一致。使用合成CLI/账号夹具，不启动用户CLI或真实服务。

补充生产门槛：启动前拒绝非空`tools.discoveryCommand`（并将自定义callCommand模式视为未支持）、启用的`tools.sandbox`/`GEMINI_SANDBOX`或`security.toolSandboxing`、启用的`advanced.ignoreLocalEnv`、外部认证`security.auth.useExternal`或不一致的enforcedType；承重字段包含未解析动态模板、异常类型或无法解析JSONC时停止。明确配置云端ADC/外部凭据或sandbox代理注入环境时不能悄悄改成另一认证/运行模式。对剩余普通字段不因名称中含shell等字样就添加无证据限制。拒绝上述环境，不传`--ignore-env`；后者会跳过私有cwd空.env并加载用户HOME下环境。默认目录信任开启，不能关掉或写信任规则。第八个合成场景已验证显式`GEMINI_CLI_TRUST_WORKSPACE=false`在不信任模式仍可/model并正常退出、没有信任文件写入；`d11b572`已通过定向独立复审，两端按此更受限模式运行，并覆盖untrusted的真实ready/额度帧。


## 2026-09-08 本地收尾检查点

Task1/2/3/4a/4b/4c的实现、当地可执行验证及独立复审完成。Swift `f96b08c`、Windows `5a0cced`/`8675152`/`057e79f`无剩余审查发现；478 Swift、109前端、249宿主Rust、严格Clippy、Release、官方CLI合成2测试/4场景、6合同及文档门禁通过。[完整日志与边界](../../development/2026-09-08-gemini-collector.md)。

上方“合并收尾”和“功能完成后转015”两项仍未完成：Windows原生SDK/ConPTY与真实账号验证受环境限制，未推送、未合并、未发布。保持已提交分支，由开发入口在验证条件可用后继续；不得将本地实现完成替换成需求012整体验收完成。
