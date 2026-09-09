# Gemini CLI 额度采集接入

关联需求：REQ-20260908-012。日期：2026-09-08。状态：本地实现、文档、可执行回归及独立审查完成；REQ-20260908-012为受环境限制，原生Windows与真实账号尚未验收，未合并或发布。

[实施协议](../design/implementation-plans/2026-09-08-gemini-provider.md) · [官方CLI能力及八场景证据](2026-09-08-gemini-cli-capability.md) · [四产品展示阶段](2026-09-08-gemini-presentation.md)。

## 数据与运行范围

首期面向固定官方Gemini CLI 0.58.0的普通OAuth模式。额度取自官方/model模型档位显示的已用百分比，保留全部可见档位和CLI重置描述；最高已用档位作为浮动环。百分比已经上游舍入，不还原原始请求数，不合计档位，也不将本机token统计替代官方额度。

使用私有工作目录、空.env和经过验证的官方限制参数；明确GEMINI_CLI_TRUST_WORKSPACE=false，不批准目录或改写信任文件。已有系统配置、企业/加密/API/Vertex、动态或有自定义启动行为的未支持配置准确降级，不能覆盖原有限制。需要认证或遇未知选择界面时停止输入，不自动登录。进程输入只含已识别就绪后的/model以及退出序列，既有缓存失败时保留最后成功数据和原时间。

Windows首期实现native CLI路径；WSL的对应用户目录/系统配置保留尚未验证，明确显示不支持该采集模式，不误报未安装。真实账号、真实服务及Windows原生运行仍需各自证据。

## 官方CLI显式测试

普通测试不依赖临时安装的上游CLI。`scripts/test.sh`主轮排除真实PTY套件，第二轮执行PTYCommandRunnerTests和GeminiPTYTests；GeminiOfficialPTYTests作为独立的显式测试，防止普通CI误访问用户CLI或依赖作者的临时目录。

先按[探测依赖锁说明](../../scripts/fixtures/gemini-cli-probe/README.md)在专用/private/tmp目录恢复固定依赖，再按能力报告生成以下四个合成场景的options和网络/文件护栏，缺任何一个都不能运行完整官方测试：

```sh
node scripts/test-gemini-cli-startup.mjs authenticated --untrusted-workspace
node scripts/test-gemini-cli-startup.mjs missing-auth
node scripts/test-gemini-cli-startup.mjs invalid-auth
node scripts/test-gemini-cli-startup.mjs quota-failure
```

然后在仓库根运行：

```sh
AI_METER_GEMINI_OFFICIAL_PTY=1 bash scripts/test.sh --filter GeminiOfficialPTYTests
```

仅在上述明确开关下，该测试读取固定测试目录的options，使用合成HOME、Node权限和进程内网络夹具，经应用实际PTY runner等候界面就绪、逐键打开/model、解析额度及退出。它不发现或启动用户CLI，不使用真实Google凭据。

根代理在修复提交 `f96b08c` 后执行上述标准入口，通过2项测试、4个官方CLI合成场景，11.691秒；包括成功额度与缺认证、无效认证、无额度。日志 `/private/tmp/req012-final-official.log`。该结果证明生产runner在这些官方CLI隔离场景中的行为，不是真实账号或原生Windows验收。

## 实现与集成证据

- 四产品展示与迁移：`f3a3428`、`7b7ec5e`；元数据与旧测试修复 `2124b78`、`8c632db`。
- 官方启动探测 `bbf701b`、不信任场景 `d11b572`、可复现依赖锁 `d9b68ef`，探测作者之外的代理独立审查通过。
- macOS 采集 `edd9cdd`、连续帧和退出校验 `f96b08c`；审查发现的 P2 与认证结构 Minor 已关闭。合法逐字节、重复帧正常退出、冲突/认证尾部均有正反证据。
- Windows 采集 `5a0cced`、修复 `8675152`、`057e79f`；独立审查发现合法 ANSI 分块误拒绝、清屏掩盖异常两项 P2 与父配置形状 Minor，测试先行修复。新增合法整块/两块/逐字节正例，纠正旧冲突负例可能在孤立ESC提前失败的假阳性。Schema同时拒绝null档位与缺失limit，由实际Draft202012Validator验证RED→GREEN。

根代理在 `f96b08c` 上执行：

| 验证 | 结果 | 本地原始日志 |
| --- | --- | --- |
| `bash scripts/test.sh` | 459 主测试/91套件 + 19 PTY/2套件，合计478；合同6份、负例、文档204份和公开安全门禁通过 | `/private/tmp/req012-final-swift.log` |
| 官方CLI显式入口 | 2测试/4合成场景，11.691秒 | `/private/tmp/req012-final-official.log` |
| Windows `npm test` / `npm run build` | 109测试/15文件、类型检查与生产构建通过 | `/private/tmp/req012-final-frontend.log`、`req012-final-frontend-build.log` |
| `cargo test --offline` / 严格 Clippy all-targets | macOS宿主243测试通过，41个target结果中包含0测的Windows限定目标；不是原生Windows通过 | `/private/tmp/req012-final-rust.log`、`req012-final-clippy.log` |
| Swift Release全target | 主应用与Widget均构建通过，15.22秒 | `/private/tmp/req012-final-release-isolated.log` |

Release首次直接调用因默认Clang缓存位于不可写用户目录而失败；将构建和缓存显式隔离至临时目录后通过。未使用“测试偶发”理由忽略失败，也未修改产品逻辑。

Windows实施者的浏览器检查通过8个Gemini状态、16种四按钮布局及632个文字角色；10份原始合成转录SHA-256/字节数一致。浏览器结果不代表原生WebView2/DPI。各任务由未编写相应产品代码的代理审查；最终集成审查明确排除探测作者对本人探测的自审，复用独立探测审查证据。

修复 `8675152` 后根代理重新执行全部宿主Rust：**247测试**、严格Clippy与6份合同及schema mutation门禁通过，日志 `/private/tmp/req012-final-rust-fixed.log`、`/private/tmp/req012-final-clippy-fixed.log`。本次修复只修改Windows解析/预检与合同，不改变Swift或前端产品代码，沿用上述对应源码已通过的Swift/前端/Release结果。

`057e79f`补齐成功额度后出现完整无额度框的状态变化。根最终完整宿主Rust **249测试**（41个target结果）、严格Clippy通过，日志 `/private/tmp/req012-final-rust-057e79f.log`、`/private/tmp/req012-final-clippy-057e79f.log`。同一独立审查者定向复审已关闭Windows全部P2与Minor；macOS和集成审查也无剩余P1/P2。公共复现说明及schema宽松两项Minor已关闭。

## 剩余验收与本地交付

本地可执行验证与独立审查已收尾。Windows-only ConPTY/Job代码及4场景测试未在原生环境编译/运行；此前交叉编译在第三方ring的Windows SDK头文件缺失处停止，尚未到项目原生代码类型检查，原始日志 `/private/tmp/req012-windows-cross-check.log`。真实Google账号、服务、GUI/DPI也未验收。未推送、未发布、未安装覆盖应用、未改版本/feed/签名配置。

分支保留为 `codex/gemini-research-design`。因原生Windows代码尚未编译和运行，暂不合入main；2026-09-08收尾复核main仍为 `3923393` 且干净，未覆盖主工作区。恢复验证时由开发入口先在原生Windows环境完成完整Rust/严格Clippy、四场景ConPTY及既有构建门禁，再在获得账号操作授权后补真实账号验收；当前不具备环境及账号授权，不能擅自执行。维持不推送、不发布的限制，因此不通过远端CI绕过本轮边界。

唯一台账将012标记受环境限制；015有“Gemini功能完成后”的前置条件，继续排队。后续环境或授权改变时由开发入口接续验收、必要修复和安全整合，再回传协调入口；不让协调入口代跑技术验证。
