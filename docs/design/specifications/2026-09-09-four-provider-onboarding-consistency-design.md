# 四服务商新用户接入一致性：审计与设计

关联需求：REQ-20260908-015。审计日期：2026-09-09。基线：`e3c4071`，需求接单：`57a8293`。

状态：按用户既有授权采用推荐方案实施。用户另以“确认发布”授权本项验收完成后发布下一稳定版本，登记为REQ-20260909-001；发布不会先于本项验证完成。原候选审查与本机门禁完成后，用户以REQ-20260909-003补充Gemini安装按钮的真实验收反馈，本规格纳入修正后再发布。

## 目标与原则

新用户从浮动条进入任一服务详情时，应能看懂当前状态并到达真正能解决问题的操作。Settings继续是安装、登录和凭据配置的主入口；详情只提供直接恢复动作或跳转，不复制凭据输入、安装和登录逻辑。

本次保持以下安全与真实性边界：

- 只有明确的可执行文件发现结果才显示“未安装”。权限、路径检查、进程、网络或输出失败显示“暂不可用”或具体错误，不能触发安装。
- 安装、登录、Claude工作区授权和DeepSeek密钥保存均由用户点击触发。测试不下载或执行安装器，也不操作真实账户。
- DeepSeek依赖API Key；官网登录仅用于30天历史同步，不能冒充API Key配置。
- Gemini依赖受支持的官方Gemini CLI 0.58.0。macOS支持原生CLI；Windows首期只支持原生CLI，WSL和Gemini桌面应用不等于受支持CLI。
- 应用不读取密码、验证码或OAuth令牌。DeepSeek密钥只进入macOS Keychain或Windows受保护凭据提示框与Credential Manager。

## 双平台依赖与恢复矩阵

| 平台 / 服务 | 实际依赖与发现 | 明确缺失 | 登录或凭据缺失 | 检查失败 / 不支持 | 推荐恢复动作 |
| --- | --- | --- | --- | --- | --- |
| macOS Claude Code | `PATH`及常见用户级CLI目录中的官方`claude` | 安装CLI；点击后才运行官方终端安装脚本 | 在终端运行官方登录；额度读取另有一次性私有空工作区授权 | 保持“账户状态暂不可用”，不误报缺失 | 详情中工作区状态直达一次性授权；其他问题打开Services |
| macOS OpenAI Codex | `PATH`、常见用户级CLI目录及官方应用内置`codex` | 安装CLI；点击后才运行官方终端安装脚本 | 在终端运行官方登录 | 保持“账户状态暂不可用”，不误报缺失 | 详情打开Services |
| macOS DeepSeek | Keychain中的API Key；不依赖DeepSeek桌面应用或CLI | 不适用 | 保存API Key，先验证后替换；失败保留旧Key | 网络验证失败与无Key分开表达 | 无Key时详情先打开Services；官网登录只用于已配置后的历史同步 |
| macOS Gemini | 常见原生CLI路径中的官方Gemini CLI 0.58.0 | Node.js 20+、固定版本npm命令及官方安装页；不自动执行安装器 | 运行`gemini`并选择Google登录，应用只重试状态 | 版本、认证模式、配置、权限和网络错误保留具体说明 | 详情和Settings提供安装/登录/回流步骤、重试及官方安装页 |
| Windows Claude Code | Auto、Native Windows、显式WSL发行版或自定义官方CLI | Auto/Native确认缺失后可点击官方安装；WSL/自定义走说明 | 在所选运行环境打开官方登录；额度读取另有私有空工作区初始化 | 坏路径、探测失败和取消保持“暂不可用” | 详情打开Services，再按运行环境操作 |
| Windows OpenAI Codex | Auto、Native Windows、显式WSL发行版或自定义官方CLI | Auto/Native确认缺失后可点击官方安装；WSL/自定义走说明 | 在所选运行环境打开官方登录 | 坏路径、探测失败和取消保持“暂不可用” | 详情打开Services，再按运行环境操作 |
| Windows DeepSeek | Windows Credential Manager中的API Key；不依赖DeepSeek桌面应用或CLI | 不适用 | 受保护Windows提示框保存或替换，先验证后写入 | 验证失败时按是否已有Key给出准确结果 | 无Key时详情打开Services；官网同步继续只负责历史 |
| Windows Gemini | Native Windows官方Gemini CLI 0.58.0 | Node.js 20+、PowerShell固定版本npm命令及官方安装页；不自动执行安装器 | 运行`gemini`并选择Google登录，应用只重试状态 | WSL明确不支持；版本、配置、权限和网络错误保留说明 | 详情和Settings提供安装/登录/回流步骤、重试及官方安装页 |

## 现状审计结论

发现、安装与登录控制器已经具备关键保护：常见路径发现、明确区分missing与unavailable、安装前重新发现、操作期间按服务去重、迟到结果拒绝、登录/安装后有限轮询，以及不自动登录。保留这些实现，不重写采集或安装架构。

需要修正的用户界面问题如下：

1. 双平台Claude/Codex在`unavailable`时，主按钮与独立按钮都显示“Check Status”，同一卡片出现两个同名动作。
2. Windows DeepSeek在首次配置、尚无Key时仍显示“Replace API Key”，并在失败时固定声称“现有Key仍有效”。macOS说明和失败消息也会在首次配置时提到不存在的旧Key。
3. macOS DeepSeek详情会在API Key缺失时优先展示官网登录；该登录只解决历史同步，不能恢复余额采集。
4. 双平台Claude/Codex详情在未安装、需登录、输出变化或暂不可用时能显示状态，却没有可达的恢复按钮；DeepSeek详情缺少API Key恢复入口。
5. Gemini原有“官方文档”按钮实际打开额度与价格页面，用户无法从中找到安装方法。它仍不应自动安装或自动登录，但必须把按钮改成明确的固定版本安装指南，直达官方安装页，并在应用内给出可执行步骤。

## 统一呈现与交互

浮动条继续用同一快照状态展示`Sign in`、`Set up`、`Not installed`、`Refreshing`、`Update needed`或`Unavailable`；操作提示只表示确实需要用户处理的状态，不把普通网络失败标成缺失。

详情的恢复策略为：

| 快照状态 | Claude | Codex | DeepSeek | Gemini |
| --- | --- | --- | --- | --- |
| `fresh` / 普通`cached` / `refreshing` | 无新增动作 | 无新增动作 | 正常展示余额及独立官网历史 | 保留重试/文档 |
| 带登录、凭据或设置失败原因的`cached` | 保留旧额度并提供对应工作区或Services动作 | 保留旧额度并打开Services | 保留旧余额并打开Services；不先展示无关的官网登录 | 保留重试/文档 |
| `setupRequired` | 一次性工作区授权 | 打开Services | 打开Services | 保留重试/文档 |
| `authenticationRequired` | 打开Services | 打开Services | 打开Services；不先展示无关的官网登录 | 保留重试/文档 |
| `notInstalled` | 打开Services | 打开Services | 打开Services | 保留重试/文档 |
| `unrecognizedOutput` / `unavailable` | 打开Services | 打开Services | 打开Services，并保留已有历史数据 | 保留重试/文档 |

macOS使用应用内设置路由通知，同时激活应用、打开Settings并选中Services。Windows扩展既有`open_settings`命令，使详情窗口可以请求Services页签。路由只携带固定页签名，不携带路径、命令或凭据。

Settings中的Claude/Codex继续保留主动作和手动检查。当主动作本身就是“Check Status”时不再渲染第二个同名按钮；其余状态保留独立检查，方便用户在终端完成操作后手动确认。

DeepSeek根据当前凭据状态显示`Save API Key`或`Replace API Key`。验证失败文案同样区分首次保存与替换：首次流程失败说明新Key没有保存；替换失败说明旧Key继续保留。候选Key验证成功前不覆盖旧Key的事务边界不变。

Gemini缺失、检查中或状态未知时显示Node.js 20+、`npm install -g @google/gemini-cli@0.58.0`、运行`gemini`选择Google登录以及返回应用重试的完整顺序。已安装但需要登录时跳过安装命令；带认证原因的旧额度缓存同样只提示登录，普通网络或限流缓存继续被动展示旧额度，不提示安装或登录；连接后不显示安装步骤。两端按钮使用“Gemini CLI 0.58.0 installation guide”并固定打开官方`/docs/get-started/installation/`，不再打开额度说明页，也不执行命令。

## 测试与验收

1. Swift与TypeScript各自用纯策略矩阵覆盖四服务商和所有快照状态，确保详情动作不会随视图改版漂移。
2. 视图测试覆盖Claude/Codex不可用时只有一个状态检查按钮、DeepSeek首次保存/替换文案及详情恢复按钮，以及Gemini缺失、待登录、连接、认证缓存和普通网络缓存的安装、登录及回流文案。
3. macOS路由测试验证先选择Services，再激活并打开Settings；Windows Rust/React测试验证固定页签路由及详情调用。
4. 固定链接测试验证两端只打开官方安装页；保留并运行既有发现、安装、登录、凭据事务、Gemini能力、浮动条和详情回归，证明没有新增自动安装、自动登录、真实网络或凭据暴露路径。
5. 完整门禁前运行`scripts/check-docs.sh`；完成独立审查与双平台CI。真实Google账号、Windows交互式终端/WebView2及DPI继续明确列为现场边界，不由模拟测试冒充通过。
