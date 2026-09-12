# macOS Services 产品 Logo 实施计划

关联规格：`docs/design/specifications/2026-09-12-macos-services-provider-logos-design.md`
关联需求：REQ-20260912-003。

### 任务1：可复用的 Settings Logo 样式

**文件：**
- 修改：`Sources/AIMeterApp/Views/ProviderLogo.swift`
- 创建或修改：`Tests/AIMeterAppTests/ProviderLogoTests.swift`

- [x] 先写失败测试，证明现有 ProviderLogo 只能输出白色样式，无法在浅色 Settings 中使用系统前景色。
- [x] 为 ProviderLogo 增加保持默认白色的可选 tint/style，并证明四个 Provider 的资源、18pt框和现有光学校正不变。
- [x] 运行 ProviderLogo 与现有浮动条渲染专项，提交实现。

### 任务2：四个 Services 分组标题

**文件：**
- 修改：`Sources/AIMeterApp/Views/ServicesSettingsView.swift`
- 修改：`Tests/AIMeterAppTests/SettingsLocalizationCoverageTests.swift`
- 修改：`Tests/AIMeterAppTests/SettingsStructureTests.swift`

- [x] 先写失败的真实宿主测试，断言四个分组目前缺少 Logo。
- [x] 用统一组件渲染18pt Logo、6pt间距与产品名称，Logo使用`.primary`并隐藏于辅助功能树。
- [x] 在浅色/深色真实窗口中核对可见、未裁切，辅助功能只朗读一次名称；运行 Settings 与本地化回归并提交。

### 任务3：完整验证、文档与本地整合

**文件：**
- 修改：`CHANGELOG.md`
- 创建：`docs/development/2026-09-12-macos-services-provider-logos.md`
- 修改：`docs/development/README.md`
- 修改：`docs/project-status.md`
- 修改：`docs/requirements-backlog.md`
- 修改：本计划复选框。

- [x] 运行 Services/Logo/真实窗口专项、`scripts/test.sh`、`scripts/build-app.sh`、`scripts/check-docs.sh`和`git diff --check`。
- [ ] 独立审查四产品映射、光学尺寸、浅深色、辅助功能与非目标行为；发现先补失败回归再修复。
- [ ] 记录测试、Release资源和审查证据，标记需求完成并合入本地`main`；没有单独发布授权时不创建版本或更新源。
