# AI Token Meter 公开 GitHub 发布实现计划

> **面向 AI 代理的工作者：** 必需子技能：使用 executing-plans 逐任务实现此计划。步骤使用复选框（`- [ ]`）语法跟踪进度。

**目标：** 将完整项目安全发布为公开 GitHub 仓库，提供标准社区文档、脱敏截图和可匿名下载的 `v0.1.2` Release，并在 About 标注作者 Miller。

**架构：** 应用品牌信息继续集中在 `AppBrand`，公开发布门禁由独立 Shell 脚本执行并由 Swift 测试验证。项目文档、GitHub 模板与 CI 都保持无凭证运行；只有工作树、完整历史、ZIP 和全量测试全部通过后，才创建远程仓库、推送 `main` 并发布已有的 arm64 安装包。

**技术栈：** Swift 6、SwiftUI、Swift Testing、Bash、Ruby、Git、GitHub Actions、GitHub CLI、Gitleaks、macOS `screencapture`。

---

## 文件结构

- 修改 `Sources/AIMeterCore/Presentation/AppBrand.swift`：集中提供公开作者名。
- 修改 `Sources/AIMeterApp/Views/AboutSettingsView.swift`：显示作者。
- 修改 `Tests/AIMeterCoreTests/AppBrandTests.swift`：验证作者品牌常量。
- 创建 `Tests/AIMeterAppTests/AboutSettingsViewTests.swift`：验证 About 使用集中作者信息。
- 创建 `scripts/check-public-release.sh`：扫描当前内容、完整 Git 历史及可选 Release 解压目录，只输出类别与路径，不输出秘密原文。
- 创建 `Tests/AIMeterAppTests/PublicReleaseSafetyScriptTests.swift`：验证门禁接受安全仓库、拒绝当前文件和历史中的模拟秘密。
- 修改 `scripts/test.sh`：把公开发布静态门禁纳入常规测试尾部。
- 创建 `.gitleaks.toml`：使用默认规则，仅允许明确列出的测试占位值。
- 创建 `LICENSE`、`CODE_OF_CONDUCT.md`、`SUPPORT.md`：MIT 和标准社区政策。
- 创建 `.github/ISSUE_TEMPLATE/bug_report.yml`、`.github/ISSUE_TEMPLATE/feature_request.yml`、`.github/ISSUE_TEMPLATE/config.yml`：公开 Issue 入口。
- 创建 `.github/pull_request_template.md`：贡献检查清单。
- 创建 `.github/workflows/ci.yml`：无凭证 macOS CI。
- 修改 `scripts/check-docs.sh` 与 `Tests/AIMeterAppTests/DocumentationCheckScriptTests.swift`：把公开仓库文件、截图和作者纳入文档门禁。
- 创建 `docs/assets/screenshots/floating-strip.png`、`provider-detail.png`、`settings.png`：由 Demo Mode 生成并裁切的脱敏截图。
- 修改 `README.md`：英文摘要、截图、GitHub Release 安装、作者、MIT 与公开限制。
- 修改 `SECURITY.md`、`CONTRIBUTING.md`、`docs/project-status.md`、`docs/development/release-process.md`：同步公开发布状态和政策。
- 创建 `docs/development/2026-09-02-public-github-release.md`：记录扫描、构建、远程与 Release 验收证据。
- 修改 `docs/design/README.md`、`docs/development/README.md`、`docs/development/commit-history.md`、`docs/requirements-backlog.md`：索引、检查点和状态。

### 任务 1：建立公开发布敏感信息门禁

**文件：**
- 创建：`scripts/check-public-release.sh`
- 创建：`.gitleaks.toml`
- 创建：`Tests/AIMeterAppTests/PublicReleaseSafetyScriptTests.swift`
- 修改：`scripts/test.sh`

- [ ] **步骤 1：编写失败的脚本合同测试**

测试创建临时 Git 仓库，分别验证安全内容通过、当前文件含 `1234567890:AA...` 时失败、秘密只存在于旧提交时仍失败，并断言输出不包含模拟秘密原文。

```swift
@Test("Rejects a credential that exists only in Git history")
func rejectsHistoricalSecret() throws {
    let fixture = try PublicReleaseFixture()
    try fixture.commit(path: "secret.txt", contents: fixture.telegramToken)
    try fixture.commit(path: "secret.txt", contents: "removed")
    let result = try fixture.runSafetyCheck()
    #expect(result.status != 0)
    #expect(!result.output.contains(fixture.telegramToken))
}
```

- [ ] **步骤 2：运行测试并确认因脚本不存在而失败**

运行：`swift test --filter PublicReleaseSafetyScriptTests`

预期：FAIL，报告 `scripts/check-public-release.sh` 不存在或无法执行。

- [ ] **步骤 3：实现最小门禁脚本**

脚本必须：检查危险跟踪文件名；对工作树与 `git log -p --all --full-history` 使用高置信度规则静默匹配；专门阻断已知 Telegram Bot 数字前缀；如果存在 Gitleaks，则以 `--redact` 分别扫描当前目录和 Git 历史；如果传入 ZIP，则解压到 `mktemp -d` 后扫描；任何失败只报告检查阶段，不打印匹配行。

```bash
if git log -p --all --full-history --no-ext-diff | grep -Eaq "$HIGH_CONFIDENCE_PATTERN"; then
  fail "Git history contains a high-confidence credential pattern"
fi
```

- [ ] **步骤 4：运行目标测试和真实只读扫描**

运行：

```bash
swift test --filter PublicReleaseSafetyScriptTests
bash scripts/check-public-release.sh
```

预期：测试通过；真实仓库若有命中则停止后续发布并先处理命中。

- [ ] **步骤 5：提交安全门禁**

```bash
git add scripts/check-public-release.sh scripts/test.sh .gitleaks.toml Tests/AIMeterAppTests/PublicReleaseSafetyScriptTests.swift
git commit -m "security: add public release secret gate"
```

### 任务 2：补充作者与 MIT License

**文件：**
- 修改：`Sources/AIMeterCore/Presentation/AppBrand.swift`
- 修改：`Sources/AIMeterApp/Views/AboutSettingsView.swift`
- 修改：`Tests/AIMeterCoreTests/AppBrandTests.swift`
- 创建：`Tests/AIMeterAppTests/AboutSettingsViewTests.swift`
- 创建：`LICENSE`

- [ ] **步骤 1：先写作者合同测试**

```swift
@Test("Exposes the public author")
func publicAuthor() {
    #expect(AppBrand.author == "Miller")
}
```

About 源码合同测试同时要求 `Text("Author: \\(AppBrand.author)")`，防止 UI 绕开品牌常量。

- [ ] **步骤 2：运行测试并确认失败**

运行：`swift test --filter AppBrandTests --filter AboutSettingsViewTests`

预期：FAIL，`AppBrand.author` 尚不存在。

- [ ] **步骤 3：实现作者展示和 MIT License**

```swift
public static let author = "Miller"
```

About 在版本文字后增加 `Author: Miller`，使用 `.caption` 和 `.secondary`；`LICENSE` 使用标准 MIT 文本与 `Copyright (c) 2026 Miller`。

- [ ] **步骤 4：运行作者测试**

运行：`swift test --filter AppBrandTests --filter AboutSettingsViewTests`

预期：PASS。

- [ ] **步骤 5：提交品牌与许可**

```bash
git add Sources/AIMeterCore/Presentation/AppBrand.swift Sources/AIMeterApp/Views/AboutSettingsView.swift Tests/AIMeterCoreTests/AppBrandTests.swift Tests/AIMeterAppTests/AboutSettingsViewTests.swift LICENSE
git commit -m "feat: credit Miller in About"
```

### 任务 3：建立标准 GitHub 社区与 CI 文件

**文件：**
- 创建：`CODE_OF_CONDUCT.md`
- 创建：`SUPPORT.md`
- 创建：`.github/ISSUE_TEMPLATE/bug_report.yml`
- 创建：`.github/ISSUE_TEMPLATE/feature_request.yml`
- 创建：`.github/ISSUE_TEMPLATE/config.yml`
- 创建：`.github/pull_request_template.md`
- 创建：`.github/workflows/ci.yml`
- 修改：`scripts/check-docs.sh`
- 修改：`Tests/AIMeterAppTests/DocumentationCheckScriptTests.swift`

- [ ] **步骤 1：扩充失败的文档门禁测试**

测试 fixture 缺少 `LICENSE`、`SECURITY.md`、`CODE_OF_CONDUCT.md`、`SUPPORT.md` 或 `.github/workflows/ci.yml` 时必须失败；完整 fixture 必须通过。

- [ ] **步骤 2：运行测试并确认失败**

运行：`swift test --filter DocumentationCheckScriptTests`

预期：FAIL，当前文档检查器没有执行公开仓库文件合同。

- [ ] **步骤 3：实现社区文件与 CI**

CI 使用 `macos-15`、`actions/checkout` 固定主版本并运行：

```yaml
- name: Test and validate documentation
  run: bash scripts/test.sh
```

Issue 与 PR 模板明确禁止上传 API Key、Token、Cookie、账户响应或未脱敏截图。CODE_OF_CONDUCT 使用 Contributor Covenant 2.1，执行渠道指向 GitHub 私密安全报告，不公开私人邮箱。

- [ ] **步骤 4：运行文档门禁测试**

运行：`swift test --filter DocumentationCheckScriptTests && bash scripts/check-docs.sh`

预期：PASS。

- [ ] **步骤 5：提交社区基础设施**

```bash
git add .github CODE_OF_CONDUCT.md SUPPORT.md scripts/check-docs.sh Tests/AIMeterAppTests/DocumentationCheckScriptTests.swift
git commit -m "chore: add GitHub community files"
```

### 任务 4：制作脱敏产品截图并完善 README

**文件：**
- 创建：`docs/assets/screenshots/floating-strip.png`
- 创建：`docs/assets/screenshots/provider-detail.png`
- 创建：`docs/assets/screenshots/settings.png`
- 修改：`README.md`
- 修改：`SECURITY.md`
- 修改：`CONTRIBUTING.md`
- 修改：`docs/project-status.md`
- 修改：`docs/development/release-process.md`
- 测试：`Tests/AIMeterAppTests/DocumentationCheckScriptTests.swift`

- [ ] **步骤 1：为 README 公开信息增加失败测试**

文档 fixture 要求 README 包含英文摘要、三个截图链接、`Miller`、`MIT`、`v0.1.2` 下载说明以及 ad-hoc/未公证提示。

- [ ] **步骤 2：运行文档测试并确认失败**

运行：`swift test --filter DocumentationCheckScriptTests`

预期：FAIL，README 公开发布合同未满足。

- [ ] **步骤 3：使用 Demo Mode 生成并检查截图**

以 `AI_METER_DEMO_MODE=1` 启动 Release App，捕获贴边浮岛、OpenAI Codex 详情和 Appearance Settings。裁切只保留 App 界面，转为 PNG，移除扩展属性和 EXIF，再用 OCR/人工检查确认没有邮箱、Key 后四位、用户名、文件路径或真实账户数据。

- [ ] **步骤 4：更新 README 与现行政策**

README 增加简短英文摘要、Screenshots、从 GitHub Releases 下载、首次打开、作者和 MIT；把现有的示例 clone 地址替换为 GitHub 登录后解析出的实际 HTTPS 地址。SECURITY 修正支持版本为 `0.1.2`，发布流程明确公开 GitHub Release 仍为 ad-hoc、未公证，项目状态从“无许可证/无 CI”更新为实施中的真实状态。

- [ ] **步骤 5：运行截图与文档验证**

运行：

```bash
swift test --filter DocumentationCheckScriptTests
bash scripts/check-docs.sh
file docs/assets/screenshots/*.png
mdls -name kMDItemAuthors -name kMDItemWhereFroms docs/assets/screenshots/*.png
```

预期：三张有效 PNG；文档无断链；元数据不含个人来源。

- [ ] **步骤 6：提交 README 与截图**

```bash
git add README.md SECURITY.md CONTRIBUTING.md docs/project-status.md docs/development/release-process.md docs/assets/screenshots Tests/AIMeterAppTests/DocumentationCheckScriptTests.swift
git commit -m "docs: prepare public project presentation"
```

### 任务 5：完成本地发布审计与开发记录

**文件：**
- 创建：`docs/development/2026-09-02-public-github-release.md`
- 修改：`docs/design/README.md`
- 修改：`docs/development/README.md`
- 修改：`docs/development/commit-history.md`
- 修改：`docs/project-status.md`
- 修改：`docs/requirements-backlog.md`

- [ ] **步骤 1：安装并固定本机扫描工具**

通过 Homebrew 安装 GitHub CLI 与 Gitleaks；记录版本，不把认证 Token 或 Homebrew 日志提交仓库。

- [ ] **步骤 2：重新构建包含作者信息的 0.1.2 分发包**

About 已发生用户可见变化，因此不得复用作者变更前的 ZIP。运行无 Widget Release 构建，重新生成 `AI-Token-Meter-0.1.2-macOS-arm64.zip` 与 `.sha256`，并删除被新版替代的旧校验值。版本仍为 `0.1.2`（build `3`），因为此前候选尚未公开发布。

```bash
AI_METER_INCLUDE_WIDGET=0 bash scripts/build-app.sh
```

- [ ] **步骤 3：运行全量测试与三层敏感信息扫描**

```bash
bash scripts/test.sh
bash scripts/check-public-release.sh dist/AI-Token-Meter-0.1.2-macOS-arm64.zip
git diff --check
```

预期：全部通过，输出不含秘密原文。

- [ ] **步骤 4：复验 Release 资产**

```bash
cd dist
shasum -a 256 -c AI-Token-Meter-0.1.2-macOS-arm64.zip.sha256
unzip -t AI-Token-Meter-0.1.2-macOS-arm64.zip
```

解压后继续验证主二进制为 arm64、`CFBundleShortVersionString=0.1.2`、About 包含作者常量、资源门禁和严格签名通过；从独立临时目录以 Finder 风格最小 PATH 启动并确认进程持续存活。

- [ ] **步骤 5：写入本地验收记录并提交**

记录测试数量、扫描器版本、扫描范围、ZIP SHA-256、已知限制与 Git 检查点；需求在远程发布完成前保持 `进行中`。

```bash
git add docs
git commit -m "docs: record public release audit"
```

### 任务 6：创建公开仓库并发布 v0.1.2

**外部状态：** GitHub 用户账户、公开仓库、Tag、Release。

- [ ] **步骤 1：通过官方流程登录 GitHub**

运行 `gh auth status`；如未登录，运行 `gh auth login --web --git-protocol https`。只接受 GitHub 官方浏览器/设备授权，不在终端历史或聊天中粘贴 Token。

- [ ] **步骤 2：解析账户并检查同名仓库**

运行 `GITHUB_OWNER="$(gh api user --jq .login)"` 得到所有者；再以 `gh repo view "$GITHUB_OWNER/AI-Token-Meter"` 检查同名仓库。若存在且不是当前本地仓库对应的空仓库，按规格停止，不覆盖。

- [ ] **步骤 3：创建 Public 仓库并推送 main**

```bash
gh repo create AI-Token-Meter --public --source=. --remote=origin --push --description "Native macOS usage meter for Claude Code, OpenAI Codex, and DeepSeek"
gh repo edit --add-topic macos --add-topic swift --add-topic swiftui --add-topic claude-code --add-topic openai-codex --add-topic deepseek --add-topic menu-bar-app
```

- [ ] **步骤 4：等待 GitHub Actions 完成**

运行 `RUN_ID="$(gh run list --workflow ci.yml --limit 1 --json databaseId --jq '.[0].databaseId')"` 和 `gh run watch "$RUN_ID" --exit-status`。预期：公开 `main` CI 成功；失败则先修复并重新验证，不发布 Release。

- [ ] **步骤 5：创建标签与 GitHub Release**

在最终发布提交创建注释标签 `v0.1.2`，推送标签，使用已验证 ZIP 和 `.sha256` 创建 Release。Release Notes 必须包含安装、nvm 修复、无 Widget、ad-hoc 与未公证限制。

- [ ] **步骤 6：匿名验证仓库与下载资产**

通过无需凭证的 GitHub URL 检查仓库为 Public、默认分支 `main`、README 三张截图可加载、License 被识别、CI 成功、Release 两个附件名称和远程 SHA-256 与本地一致。

- [ ] **步骤 7：回填需求与最终提交**

将仓库 URL、Release URL、CI run、标签和资产哈希写入开发日志、项目状态、提交历史与需求台账；只有公开下载复验通过后把 `REQ-20260902-017` 标记为 `已完成`。

```bash
git add docs README.md
git commit -m "docs: record public GitHub release"
git push origin main
```

### 任务 7：最终独立复核

- [ ] **步骤 1：从远程重新检查公开表面**

确认没有未推送提交，`git status --short --branch` 干净，`git ls-remote origin` 的 `main` 与本地 HEAD 相同。

- [ ] **步骤 2：重新运行完整验证**

运行：

```bash
bash scripts/test.sh
bash scripts/check-public-release.sh dist/AI-Token-Meter-0.1.2-macOS-arm64.zip
git diff --check
```

预期：全绿；需求列表除既有 `待用户确认`、`已延期`、`受环境限制` 项外，无遗漏的可继续发布事项。

- [ ] **步骤 3：报告发布结果**

向用户提供公开仓库、Release、直接下载链接、SHA-256、CI 状态、测试数量、已知签名限制和作者展示位置。
