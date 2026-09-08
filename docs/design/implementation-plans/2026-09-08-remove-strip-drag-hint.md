# 移除双平台顶部横线实现计划

> 面向AI工作者：使用subagent-driven-development逐任务实现，任务审查后进行最终分支审查。用户已批准设计与普通技术决定，无需再次确认。

**目标：** 两端展开态无顶部小横线，拖动和Provider点击不回归。
**架构：** 直接删除无命中装饰及专用样式，保留原父容器交互、命中形状和几何。
**技术栈：** SwiftUI/AppKit、React/CSS、Swift Testing、Vitest及真实浏览器验证。

## 全局约束

仅去掉展开态顶部横线；左右贴边与Compact/Comfortable均覆盖。背景拖动、Provider点击隔离、左右吸附/多屏/位置保存、背景外轮廓、Logo/间距/窗口尺寸及折叠态竖线保持。禁止发布、安装、版本/feeds/签名/账号操作，不修改主工作区或他人未提交内容。

### Task 1: 两端横线删除与交互回归

**所有权：** `Sources/AIMeterApp/Views/FloatingStripView.swift`；必要对应`Tests/AIMeterAppTests/FloatingStrip*Tests.swift`；`windows/src/components/FloatingStrip.tsx`及`.test.tsx`；`windows/src/styles.css`；必要浏览器夹具/测试。控制者负责docs/台账。

- [x] 阅读规格与现有拖动/布局测试，记录有横线的原生/浏览器渲染证据，运行相关基线。
- [x] RED：实际组件渲染验证不应存在展开态横线；以真实macOS渲染顶部区域检查与浏览器DOM/图像检查，不能用源码字符串断言替代。Windows核心断言例：
  ```ts
  expect(container.querySelector('.floating-strip__drag-handle')).toBeNull()
  fireEvent.pointerDown(screen.getByRole('navigation'), {button: 0})
  expect(drag).toHaveBeenCalledTimes(1)
  ```
  左右/两密度矩阵同时验证尺寸、背景拖动、Provider点击不触发drag。折叠态span/竖线及展开按钮存在。macOS用实际渲染证据确认移除，运行左右/两密度背景命中排除Provider的回归；必要增加渲染测试。
- [x] 最小删除macOS展开态Capsule专用VStack和Windows装饰元素/三处专用样式，不动其余逻辑。
- [x] GREEN：运行新测试和现有FloatingStrip命中、拖动、布局、位置/多屏相关测试；验证实际渲染左右/两密度无横线且折叠竖线保留，保存证据到`/private/tmp/req011-*`。
- [x] 先` scripts/check-docs.sh`，然后完整Swift测试（独立任务缓存）、Windows前端全套/生产构建、宿主Rust回归与macOS Release编译。记录准确命令、退出码、测试数与原生Windows边界；只提交自有文件，提供报告交独立审查。

## 控制者收尾

- [x] 补Unreleased、设置指南/当前状态、开发日志与索引，保留010交付和011批准记录。
- [x] 审查包与任务审查、最终分支审查全部通过，完成文档门禁及Git证据，标记本需求完成，主动回传协调对话；不自行合并main或发布。
