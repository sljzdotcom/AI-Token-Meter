# 贴边浮岛 S 曲线开发日志

日期：2026-08-31  
开发分支：`codex/floating-strip-s-curve`（已合入 `main`）

## 背景与目标

上一版连续轮廓仍使用平台和近似半圆组合，和用户手绘的“贴边尖点—S 形肩部—中央主体”不一致。本阶段只替换 `FloatingStripShape` 几何：保留 `108 × 356` 窗口、三枚 Logo、详情页、窗口布局、拖动控制器和三家服务采集逻辑。

确认规格见 [贴边浮岛 S 曲线轮廓规格](../design/specifications/2026-08-31-floating-strip-s-curve-design.md)，执行步骤见 [实现计划](../design/implementation-plans/2026-08-31-floating-strip-s-curve.md)。

## 几何实现

右侧基准路径从 `(108, 0)` 出发，使用两段三次贝塞尔到达主体顶部 `(0, 88)`；底部严格使用垂直镜像坐标，从 `(0, 268)` 收束到 `(108, 356)`。左侧继续复用同一路径的水平镜像逻辑。

关键连接点为 `(78, 52)` 与 `(78, 304)`。尖点和主体连接处的控制点保持近似竖直切线，因此没有水平平台、阶梯、额外端帽或独立边框。

## 测试驱动证据

### 红灯

先把视觉测试改为 S 曲线契约，再对旧实现运行。测试在右侧顶部外部点 `(90, 1)`、`(90, 20)`、`(70, 40)` 及其上下/左右镜像处产生 9 个预期断言失败，证明旧平台区域会被新测试准确识别；失败为几何断言而非编译问题。

### 绿灯与计划校正

最小 Shape 修改后，新的视觉测试通过。随后拖动专项测试指出旧夹具 `(86, 40)` 已位于新曲线透明区；这属于测试坐标过期，不是生产逻辑回归。计划据此补充夹具迁移：右侧 `(75, 58)`、左侧 `(33, 58)`、两倍缩放并平移后的 `(250, 316)`，以及 AppKit 底部原点窗口坐标 `(75, 298)`。

最终专项结果：

- `VisualSystemTests`：7 项通过；
- `FloatingStripDragShapeTests`：4 项通过；
- `FloatingStripPointerDragStateTests`：3 项通过；
- 完整 `bash scripts/test.sh`：148 项测试、33 个测试组、0 失败；环境相关 smoke 检查按设计跳过；
- `git diff --check`：无输出。

## Release、签名与安装

- `bash scripts/build-app.sh`：Release 构建、App Bundle 组装与 ad-hoc 签名通过；
- `plutil -lint`：`Info.plist` 合法；
- `codesign --verify --deep --strict --verbose=2`：构建版和安装版均有效；
- 可执行文件：arm64 Mach-O；App Icon 存在且非空；
- 构建版和安装版可执行文件 SHA-256 均为 `2556a949625a92a26f9550c8f6f59dd04837a13b0caf16da3463914b4373972d`；
- 替换前应用保存在可恢复备份 `/private/tmp/AI Meter.app.pre-s-curve-20260831-final`；
- 安装目标为 `/Applications/AI Meter.app`，安装后已重新启动。

## 真实界面验收

使用已安装的 `/Applications/AI Meter.app` 完成真实 AppKit/SwiftUI 界面验收：

- 右侧轮廓从上下贴边尖点以连续 S 曲线接入主体，没有平台、阶梯、边框或接缝；
- 切换到 Left 后得到严格水平镜像，随后恢复用户默认 Right；
- 三枚 Logo 的位置、尺寸和视觉重量保持不变；
- 上部曲面真实拖动使垂直位置从 99% 变为 95%；
- 圆环间玻璃真实拖动使位置从 95% 变为 99%；
- 下部曲面真实拖动使位置从 99% 变为 95%；
- 点击 Codex Logo 只把详情状态从关闭变为打开，位置仍为 95%；
- macOS 应用菜单中的 `Settings…` 成功打开唯一的 `AI Meter Settings` 窗口，既有 Automatic / Left / Right 与自动隐藏设置均正常显示。

最终公开截图见 [浮岛截图](../assets/screenshots/floating-strip.png)，只包含浮岛本身，不包含账户凭证或私密详情。

## Git 节点

| 提交 | 内容 |
| --- | --- |
| `a840bce` | 确认 S 曲线轮廓规格 |
| `6f15d9e` | 编写测试驱动实施计划 |
| `9f351e7` | 用双贝塞尔实现 S 曲线，并完成专项与完整测试 |
| `78bc256` | 记录 Release 安装、真实交互与文档验收 |
| `de3d5d6` | 非快进合并到 `main` |

## 主分支集成结果

`main` 在合并提交 `de3d5d6` 上重新验证：完整测试为 148 项、33 个测试组、0 失败；Release 构建、`Info.plist`、arm64 可执行文件、App Icon 和严格代码签名检查均通过。最终文档节点写入后还会再次执行同一验证，不以功能分支结果代替主分支证据。

## 安全与非目标复核

- 未修改 Claude、Codex、DeepSeek 采集器、凭证、缓存或额度算法；
- 未读取、打印、迁移或删除任何 API Key；
- 未修改窗口尺寸、Logo 资源、进度环颜色或详情页布局；
- 安装替换保留明确备份，可在需要时恢复。
