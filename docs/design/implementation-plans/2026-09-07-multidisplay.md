# 多显示器实现计划

> **面向 AI 代理的工作者：** 使用 executing-plans 逐任务实现；独立 Windows 子系统按 dispatching-parallel-agents 分工。复选框追踪实施，不替代需求台账。

**目标：** 修复跨屏拖动并支持主屏、指定屏、所有屏与持久位置。

**架构：** Core 保存模式与每屏位置；App 协调实例、唯一详情和屏幕变化；每个控制器只负责一个展示实例。Windows 共享原有采集运行时，按同一语义管理展示窗口。

**技术栈：** Swift/AppKit/SwiftUI、Rust/Tauri、React/TypeScript。

## 任务 1：位置与拓扑策略

文件：新增 Sources/AIMeterCore/Preferences/FloatingStripDisplays.swift、Tests/AIMeterCoreTests/FloatingStripDisplaysTests.swift；新增 Sources/AIMeterApp/System/FloatingStripDragPolicy.swift、Tests/AIMeterAppTests/FloatingStripDragPolicyTests.swift。

- [x] 先写迁移/持久化和坐标测试；分别运行 scripts/test.sh --filter FloatingStripDisplaysTests 与 --filter FloatingStripDragPolicyTests 确认缺失行为导致失败。

~~~swift
let preferences = FloatingStripDisplays(mode: .selected, selectedIdentifier: "external")
#expect(preferences.targetIdentifiers(online: ["primary"], primary: "primary") == ["primary"])
#expect(preferences.selectedIdentifier == "external")
#expect(preferences.targetIdentifiers(online: ["primary", "external"], primary: "primary") == ["external"])
~~~

- [x] 实现 Codable 模式 primary/selected/all、每屏 placement，使用独立 UserDefaults Data 键；未知数据从旧位置迁移，不修改账户设置。
- [x] 实现不受 edge preference 限制的 translation；全屏矩形指针命中优先，间隙选最近屏；All 模式返回原屏。
- [x] 运行定向测试转绿并提交，检查点 3b6dcee。

## 任务 2：macOS 实例协调及设置接线

文件：新增 Sources/AIMeterApp/System/FloatingStripCoordinator.swift、FloatingStripWindowRegistry.swift、Views/FloatingStripDisplaySettings.swift；修改同目标 System/FloatingPanelController.swift、AppModel.swift、AppDelegate.swift、Views/AppearanceSettingsView.swift；新增 Tests/AIMeterAppTests/FloatingStripCoordinatorTests.swift。

- [x] 写唯一详情所有权/拓扑增删测试：呈现 B 前关闭 A、断开 B 后无悬挂详情；真实协调策略而非源码字符串检查。运行定向测试确认失败。
- [x] 控制器接受固定屏标识及 present 回调，移除覆盖 model 全局 handler 的行为；协调器统一广播外观、显示与拓扑变化。

~~~swift
// 所有实例共用同一个 model；呈现新详情前释放旧详情宿主。
for controller in controllers.values { controller.dismissDetail() }
controllers[target]?.showDetail(for: provider)
~~~

- [x] model 增加显示模式、在线屏列表和持久化方法；Settings 加模式/目标选择与 Move to primary。离线目标仍显示说明；用户拖动更新 selected，自动重排不保存。
- [x] controller 按指针目标处理拖动，拖动时不自动重排；所有屏模式限制目标；释放详情 contentView，清理被移除实例资源。
- [x] 定向测试、完整 bash scripts/test.sh、Release 编译通过，复审修复检查点 d4bd42d、255e8b3。

## 任务 3：Windows 对等显示控制

文件：windows/src-tauri/src/platform/windows/monitor.rs、lib.rs、persistence/settings.rs；windows/src/settings/SettingsWindow.tsx、Shell.tsx 及对应测试。

- [ ] 先测试 legacy→selected、primary/all 解析、断屏回退与重连恢复，每屏位置无串写；运行 cargo test 见红灯。
- [ ] 增加实例管理、窗口标签到稳定屏 ID 映射和唯一详情归属；单屏拖动换目标，all 模式每屏独立。保持一个采集 runtime。
- [ ] 用 Rust 与前端行为测试验证设置保存、事件同步、显示列表更新；运行 npm test、npm run build、cargo test 与 cargo clippy --all-targets -- -D warnings。
- [ ] 提交 Windows 检查点，报告 Windows-only 编译/真机边界。

## 任务 4：集成与证据

- [ ] 更新用户指南、架构说明、开发日志、CHANGELOG 和需求状态；运行 bash scripts/check-docs.sh。
- [ ] 独立代码审查，处理全部阻断；完整双平台测试及 Windows CI 编译验证后再集成。未取得真机双屏证据不得写成真机验收通过。
