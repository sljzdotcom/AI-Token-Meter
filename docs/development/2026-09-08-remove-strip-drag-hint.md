# 双平台展开态顶部横线移除

关联 REQ-20260908-011，日期2026-09-08。用户已批准两端去掉顶部小横线，保留原拖动行为及折叠态竖线，暂不发布。

## 基线与设计

从REQ-010最终交付 `6917af8` 接续，在独立分支 `codex/remove-strip-drag-hint` 以 `e843750` 引入批准 `4cf994a`；冲突只添加011批准行，完整保留010已完成记录与009协调证据。规格/计划检查点 `393a7eb`。

- [规格](../design/specifications/2026-09-08-remove-strip-drag-hint-design.md)
- [计划](../design/implementation-plans/2026-09-08-remove-strip-drag-hint.md)

macOS顶部18×3 Capsule关闭命中；Windows装饰为aria-hidden，拖动事件在父容器。删除范围仅为这两处装饰及其专用样式，不改背景、轮廓、窗口尺寸、Logo布局、Provider点击、贴边/多屏/位置持久化或折叠态3×40竖线。

## 验证与交付

实现、红绿回归、左右/密度渲染核验、完整测试与独立审查结果在完成后记录。本机浏览器/宿主Rust验证与原生Windows WebView2/DPI现场验收分别记录，不以宿主测试替代原生Windows结果。

本次不发布、安装替换应用或操作真实账号；0.5.1、版本、签名和更新源保持原状态。由协调入口整合010/011，本开发分支不直接合并main。
