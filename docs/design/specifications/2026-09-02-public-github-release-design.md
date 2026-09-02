# AI Token Meter 公开 GitHub 发布设计

- **需求：** `REQ-20260902-017`
- **日期：** 2026-09-02
- **状态：** 已确认方案 A，待书面规格复核

## 目标

将 AI Token Meter 以公开 GitHub 仓库形式发布，保留完整且可追溯的开发历史，提供标准开源项目文档、脱敏产品截图和可下载的 `v0.1.2` Apple Silicon Release，并在应用 About 与项目文档中把作者标注为 `Miller`。

发布的硬性前提是：当前源码、完整 Git 历史、文档、图片与分发包均不得包含个人 API Key、Telegram Bot Token、Cookie、私钥、CLI 凭证、真实账户响应或其他可用于访问账户的秘密。

## 选定方案

采用**方案 A：保留完整 Git 历史并公开发布**。

- 保留现有 `main` 历史、需求台账、设计规格和开发日志；
- 不通过压缩历史掩盖未经验证的内容；
- 公开前对工作树、所有可达 Git 对象及 Release 包分别执行敏感信息扫描；
- 任何高置信度秘密命中都会阻断仓库创建或推送，先完成清理、撤销/轮换和复验；
- 现有开发记录中的提交哈希保持有效，不主动重写历史。

## GitHub 仓库

- 仓库名：`AI-Token-Meter`；
- 可见性：Public；
- 默认分支：`main`；
- 描述：原生 macOS AI 用量监控工具，支持 Claude Code、OpenAI Codex 与 DeepSeek；
- Topics：`macos`、`swift`、`swiftui`、`claude-code`、`openai-codex`、`deepseek`、`menu-bar-app`；
- About Website 暂不填写，不虚构产品主页；
- 保留现有本地仓库为权威来源，安全门禁通过后再增加 `origin` 并首次推送。

如果 GitHub 账户中已存在同名仓库，先检查所有者、可见性和现有内容；不会覆盖、删除或强推未知仓库。此时停止发布并要求用户选择新名称或明确处理方式。

## 开源与社区文件

仓库根目录保持或新增：

- `README.md`：中英文项目摘要、产品截图、功能、下载、安装、权限、服务配置、从源码构建、隐私、已知限制、文档导航和作者；
- `LICENSE`：MIT License，版权方为 `Miller`；
- `CHANGELOG.md`：现有版本历史；
- `CONTRIBUTING.md`：开发、测试、隐私和提交规范；
- `SECURITY.md`：受支持版本、私密报告方式与敏感数据边界；
- `CODE_OF_CONDUCT.md`：Contributor Covenant；
- `SUPPORT.md`：用户问题、缺陷、安全问题的正确渠道；
- `.github/ISSUE_TEMPLATE/`：缺陷与功能需求模板，明确禁止粘贴真实 Key、Cookie 和账户响应；
- `.github/pull_request_template.md`：测试、隐私、截图与文档检查清单；
- `.github/workflows/ci.yml`：在 GitHub macOS runner 上运行测试和文档门禁，不注入个人凭证。

README 以中文为主要说明语言，并在开头提供简洁英文摘要，方便 GitHub 搜索与非中文读者判断项目用途。深层设计与开发记录继续保持现有语言，不做无价值的批量翻译。

## 产品截图

公开素材放在 `docs/assets/screenshots/`，README 至少展示：

1. 贴边浮动条；
2. Provider 详情页；
3. Settings 的分栏结构。

截图规则：

- 优先使用测试数据、不可识别的示例数据或不含账户内容的界面；
- 不显示 API Key 后四位、邮箱、账户名、组织名、真实余额、真实充值券编号或浏览器登录状态；
- 裁掉桌面菜单栏、其他应用、用户名、磁盘路径和个人壁纸等无关环境信息；
- 保留统一尺寸、圆角和暗色背景，采用 PNG；
- 图片提交前进行 OCR/元数据检查，并删除可能包含定位或设备信息的元数据。

不为了截图加入会进入正式产品的演示后门或硬编码假数据。需要稳定画面时使用测试/预览构造器或局部组件渲染工具，并与生产凭证路径隔离。

## About 与作者信息

About 页面在版本信息附近新增 `Author: Miller`。作者文字使用 Settings 固定的系统字体，与现有 About 视觉层级保持一致，并提供可测试的集中品牌常量，避免在多个文件重复写死。README 与 MIT License 同步使用 `Miller`；不公开私人邮箱、电话号码或其他联系信息。

## 敏感信息发布门禁

发布前按以下层级验证：

1. **当前工作树：** 检查已跟踪和未跟踪文本、配置、脚本、fixture、图片元数据；
2. **完整 Git 历史：** 扫描所有分支、标签和可达对象，不只检查 `HEAD`；
3. **生成产物：** 解压 ZIP，检查文件清单、文本资源、Info.plist、签名信息和嵌入资源；
4. **人工高风险复核：** Telegram Bot Token、OpenAI/DeepSeek/Anthropic Key、Authorization、Cookie、私钥、`.env`、凭证数据库和真实账户响应；
5. **防回归：** 扩充现有隐私测试，并在 CI 中运行无需真实账户的检查。

检测工具输出不得在日志中打印秘密原文；记录规则、文件路径、命中类别和处理结论即可。已知用户曾在对话中提供的 Telegram Token 不属于仓库内容，仍以其数字前缀和 Token 结构专门检查 Git 历史。

Keychain、CLI 凭证和 WebKit Cookie 保持运行时本机存储，不进入 Git、截图、CI 或 Release。`dist/`、`.build/`、Xcode 用户数据和本机缓存继续由 `.gitignore` 排除。

## GitHub Release

- 标签与标题：`v0.1.2`；
- 附件：`AI-Token-Meter-0.1.2-macOS-arm64.zip` 和对应 `.sha256`；
- Release Notes 摘要功能、系统要求、安装步骤、三项 Provider 准备、0.1.2 的 nvm Codex 修复、已知限制；
- 明确标注当前包为 Apple Silicon、macOS 14+、ad-hoc 签名、未经过 Developer ID 公证；
- 明确标注公开 Release 暂不包含 Widget；
- 发布前再次验证 ZIP 哈希、解压完整性、arm64 架构、资源包和严格签名。

如果 GitHub 已存在 `v0.1.2` 标签或 Release，不覆盖未知资产；先对比标签目标、资产名称和哈希，再决定复用或停止。

## 自动化与权限

GitHub Actions 只执行确定性的源码测试和文档检查：使用受支持的 macOS runner，checkout 后运行 `bash scripts/test.sh`，不访问 Keychain、真实 CLI 账户、DeepSeek 官网会话或个人 API，不保存任何 Actions Secret，也不自动发布 Release。首次发布由本地验证完成后显式创建。

仓库创建、推送和 Release 上传是用户已明确授权的外部写操作。GitHub 登录如果缺失，只通过 GitHub 官方设备/网页授权完成，不要求用户在聊天中粘贴 Token。

## 测试与验收

- About 显示 `Author: Miller`，对应单元/源码合同测试通过；
- README 所有相对链接和图片可解析；
- 标准社区文件存在，模板不诱导用户提交秘密；
- 全量 Swift 测试和文档门禁通过；
- 敏感信息扫描覆盖工作树、完整 Git 历史和 ZIP，结果为无未处理高置信度秘密；
- Release ZIP 的 SHA-256 与 GitHub 附件一致；
- 远程仓库为 Public、默认分支为 `main`；
- GitHub 仓库页面能显示 README 与截图；
- `v0.1.2` Release 页面可以匿名下载两个附件。

## 错误与停止条件

- 未安装或未登录 GitHub 工具：安装官方 GitHub CLI，并引导一次官方网页登录；
- 同名仓库已存在且内容未知：不覆盖、不强推，停止并报告；
- 扫描命中秘密：不创建公开仓库、不推送，记录位置并先清理；
- CI 在 GitHub 环境失败：仓库可以保持已创建但不宣告发布完成，修复后再发 Release；
- Release 上传失败：保留本地已验证附件，重试同一标签，不创建重复版本；
- GitHub API 或网络暂时不可用：不改变本地可发布状态，稍后安全重试。

## 非目标

- 本次不购买 Apple Developer 会员、不做 Developer ID 签名或公证；
- 不恢复 Widget 证书事项；
- 不建立产品网站、自动更新器、Homebrew Cask 或 App Store 发布；
- 不公开联系方式、真实服务账号、个人统计或凭证；
- 不重写现有 Git 历史，除非扫描发现必须从历史中清除的秘密。
