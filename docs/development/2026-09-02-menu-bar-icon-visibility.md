# 菜单栏 Quantum Dial 可见性修复记录

**日期：** 2026-09-02  
**需求：** `REQ-20260902-012`  
**分支：** `codex/menu-bar-icon-visibility`  
**状态：** 已完成并合入 `main`

## 问题

用户反馈已安装应用的菜单栏图标看不见。应用进程和浮动条均正常，菜单栏动态图形也具有 18×18pt 的透明像素输出，因此问题不是应用未启动、数据未刷新或状态项尺寸为零。

## 根因

Quantum Dial 原先由 SwiftUI `Shape` 使用 `Color.primary` 直接绘制到 `MenuBarExtra` 标签。测试画布会根据显式浅色/深色环境切换颜色，但输出不是 macOS 模板图像；当真实菜单栏背景与 SwiftUI 推断的颜色不一致时，深色线条会与深色或动态菜单栏背景融合。

同一代码库中能够由系统可靠着色的图像明确设置了 `NSImage.isTemplate = true`，这也是正常实现与故障实现的关键差异。

## 修复

- 保留既有 270° 断环、三枚刻度、指针、中心轴和动态进度；
- 将矢量字形渲染到透明的 18×18pt、2× 像素图像；
- 输出 `NSImage` 时设置 `isTemplate = true`，SwiftUI 侧同时使用 template rendering mode；
- 图形颜色现在由 macOS 根据当前菜单栏前景色统一着色，不再依赖桌面背景与 SwiftUI 颜色推断。

## TDD 与自动化

红灯：新增测试先引用尚不存在的 `MenuBarMeterTemplateImage`，编译明确失败。  
绿灯：新增契约验证模板属性、18×18pt 逻辑尺寸、36×36 像素表示和非空可见像素；Quantum Dial 定向套件 4/4 通过。

完整回归：功能分支及合并后的 `main` 均为 **305 个测试、60 个测试组、0 失败**。

## Release 与安装

- 无 Apple Development 身份，Widget 按既有策略跳过；主应用 Release 构建成功；
- 候选与安装版均通过 `codesign --verify --deep --strict`；
- 候选与安装版主可执行文件 SHA-256 均为 `19196c72bda9d0dd0c1da78d95e1387031c60fa1aafc1bf7f98b2363ba6578e7`；
- 旧安装版可恢复备份位于 `/private/tmp/AI Token Meter.pre-template-menu-icon-20260902-1450.app`；
- 新版已安装到 `/Applications/AI Token Meter.app` 并成功启动，真实数据刷新到 Claude Code 0%、OpenAI Codex 38% 与 DeepSeek 余额状态。

## 验收边界

当前自动化界面工具能读取浮动条和应用菜单，但不会捕获 macOS 状态栏本身；模板属性、像素、Release、签名、安装和运行时刷新由自动化证据覆盖。用户随后明确回复“看到了”，确认真实菜单栏肉眼可见，视觉验收通过。

## Git 证据

- `ed4e5e4`：登记菜单栏图标不可见缺陷；
- `70a6bab`：以 macOS 模板图像输出 Quantum Dial，并增加回归测试；
- `1392b61`：记录根因、Release、安装哈希和待视觉确认状态；
- `1953182`：记录用户视觉验收并快进合入 `main`。
