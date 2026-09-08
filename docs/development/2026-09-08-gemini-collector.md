# Gemini CLI 额度采集接入

关联需求：REQ-20260908-012。日期：2026-09-08。状态：实现与定向验证中，尚未作分支最终交付或标记012完成。

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

## 待收尾证据

两端实现提交、定向验证汇总、独立审查、完整回归、Release/前端构建、合同门禁和本地整合结果由开发入口在本日志补齐。未推送、未发布、未安装覆盖应用、未改版本/feed/签名配置。
