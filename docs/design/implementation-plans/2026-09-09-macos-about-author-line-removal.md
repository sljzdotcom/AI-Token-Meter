# macOS About 作者行移除实现计划

> **面向 AI 代理的工作者：** 在长期开发对话内使用 `executing-plans` 顺序执行；每个检查点都以新鲜验证证据放行。

**目标：** 从macOS Settings → About删除独立的`Author: Miller`可见行及其自然消失的间距，同时保留全部品牌、链接、隐私和更新功能。

**架构：** 直接移除`AboutSettingsView`中唯一的作者文字节点，不引入条件分支或新状态。作者元数据继续用于开源归属，`BrandLinksView`及其固定目标保持原样。

**技术栈：** Swift 6、SwiftUI、AppKit、Swift Testing、SwiftPM。

---

### 任务1：建立可审查基线

**文件：**
- 已修改：`docs/requirements-backlog.md`
- 创建：`docs/design/specifications/2026-09-09-macos-about-author-line-removal-design.md`
- 创建：`docs/design/implementation-plans/2026-09-09-macos-about-author-line-removal.md`
- 修改：`docs/design/README.md`

- [x] **步骤1：读取唯一需求台账、对话绑定、协作规则和当前checkout状态。**
- [x] **步骤2：检查`AboutSettingsView`、`AppBrand`、`BrandLinksView`及现有测试，确认作者文字是独立节点且没有专用留白。**
- [x] **步骤3：比较直接删除、条件隐藏和扩大删除三种方案，按用户明确范围选择直接删除。**
- [x] **步骤4：运行文档与差异检查，提交规格和计划检查点。**

### 任务2：删除macOS可见作者行

**文件：**
- 修改：`Sources/AIMeterApp/Views/AboutSettingsView.swift`
- 验证：`Tests/AIMeterCoreTests/AppBrandTests.swift`
- 验证：`Tests/AIMeterAppTests/BrandLinksViewTests.swift`
- 验证：`Tests/AIMeterAppTests/SettingsStructureTests.swift`

- [x] **步骤1：从`AboutSettingsView`删除`Text(AppBrand.authorLine)`及其两个专用样式修饰符，不改变相邻版本文字或`BrandLinksView`。**
- [x] **步骤2：运行`swift test --filter 'AppBrandTests|BrandLinksViewTests|SettingsStructureTests'`，确认品牌元数据、三条链接、窄宽度布局、点击、失败反馈与Settings结构全部通过。**
- [x] **步骤3：检查生产差异，确认Windows源码、链接目标、版本和更新源均未改变。**

### 任务3：完整验证、文档与本地整合

**文件：**
- 创建：`docs/development/2026-09-09-macos-about-author-line-removal.md`
- 修改：`docs/development/README.md`
- 修改：`docs/project-status.md`
- 修改：`docs/requirements-backlog.md`
- 修改：`CHANGELOG.md`

- [x] **步骤1：运行项目规定的完整Swift测试与macOS无Widget Release App构建，读取退出码和测试总数。**
- [x] **步骤2：运行`scripts/check-docs.sh`、跨平台合同、公开安全与`git diff --check`。**
- [x] **步骤3：逐项审查范围、布局、链接、版权、Windows隔离和未发布边界，修复发现并重跑受影响检查。**
- [x] **步骤4：把REQ-006标为已完成，记录测试、审查和Git证据；提交候选。**
- [x] **步骤5：保留协调入口未提交记录及既有stash，把验证后的提交安全快进整合到本地main；不推送、不打标签、不改feed。**
- [ ] **步骤6：重新读取需求台账，向协调入口回传REQ-006结果并继续REQ-007官方调研。**
