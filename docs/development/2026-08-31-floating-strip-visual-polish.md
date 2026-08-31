# 浮岛接边、阴影与服务品牌色开发记录

日期：2026-08-31  
分支：`codex/visual-polish`  
规格：`docs/design/specifications/2026-08-31-floating-strip-corner-shadow-design.md`

## 问题与根因

上一版浮岛的 S 曲线在进入中央竖直主体时只有切线方向连续，随后立即变为直线，曲率突然归零。浅色壁纸会把这个位置显示成轻微硬转角。表面还叠加了 34% 黑色、18 点模糊、带水平和垂直偏移的外投影，进一步放大了接边处的黑色轮廓。

三个服务的正常状态此前都直接使用全局薄荷绿到紫色渐变，圆环、菜单卡片和详情页缺少稳定的服务身份。部分 Codex 子视图还直接引用薄荷色，无法随着服务主题同步。

## 实现决策

### 浮岛轮廓

- 保留 `108 × 356` 窗口和屏幕边缘尖点；
- 顶部依次经过 `(78, 52)`、`(16, 82)`、`(0, 104)`，在最后一段把水平控制量降为零后进入直边；
- 底部严格关于 `y = 178` 镜像；
- 左贴边继续由同一基准路径水平镜像；
- 完全移除 `FloatingStripSurface` 外投影，不增加替代描边、内阴影或装饰层。

### 服务品牌色

新增 `ProviderAccentPalette.swift`，由 `UsageProvider` 唯一解析服务主题：

| 服务 | 正常状态渐变 |
| --- | --- |
| Claude | `#E8B96D → #D97757` |
| Codex | `#FF6FAE → #A96DFF` |
| DeepSeek | `#54EDC6 → #7769FF` |

统一解析器只在 `UsageSemantic.normal` 时返回品牌渐变。warning、critical、stale 和 unavailable 继续使用警告黄、严重红、缓存橙和不可用灰，并保留非颜色状态符号。

品牌色已经同步到：

- 浮岛与菜单卡片圆环；
- 菜单卡片服务标题和主值；
- Claude 紧凑详情的标题、额度、次级百分比和进度条；
- Codex 标题、额度、进度条、本机统计和重置券卡片；
- DeepSeek 标题、余额、统计卡、每日成本图和登录框强调线。

Logo 资源、Logo 光学校正、圆环尺寸、详情布局和所有数据采集逻辑保持不变。

## TDD 证据

### 红灯

1. 新肩部内外点和透明像素测试先在旧路径运行：`VisualSystemTests` 出现 6 个预期失败，证明旧路径在 `y = 94/100` 已过早进入竖直主体，且外部像素存在 alpha。
2. 新 palette 测试先产生编译失败：`UsageProvider` 尚无 `accentPalette`，`UsageSemantic` 尚无 `accentRole`。
3. 进度条渲染测试先产生编译失败：`AIMeterProgressBar` 尚无 `provider` 参数。

### 测试校正

最初把 `(0, 100)` 同时用于几何与 alpha 测试。新曲线在该点附近经过，正常抗锯齿会产生少量 alpha，因此该像素不适合判定阴影。最终保留 `(0, 100)` 几何断言，并使用距边界更安全的 `(0, 94)` 检查 Shape 外透明度；这个点在旧实现失败、在新实现通过。

### 绿灯

- `VisualSystemTests`：11 项通过；
- `FloatingStripDragShapeTests`：4 项通过；
- `FloatingStripPointerDragStateTests`：3 项通过；
- 完整 `bash scripts/test.sh`：152 项测试、33 个测试组、0 个失败；4 项真实环境或钥匙串门控测试按设计跳过。

完整测试中 WebKit 测试辅助进程报告用户缓存目录不可写，但对应 App 启动、视觉和焦点测试均通过，测试进程退出码为 0。项目测试脚本继续使用隔离的 SwiftPM/Clang 缓存。

## Git 检查点

- `1984ecc`：实施计划；
- `200c23e`：圆滑肩部与无阴影表面；
- `60ec0d2`：集中式服务 palette 和语义覆盖；
- `6f2315f`：浮岛、菜单与详情页配色同步。

## Release、安装与真实界面验收

`bash scripts/build-app.sh` 完成 release 构建、资源组装、图标生成和 ad-hoc 签名。`codesign --verify --deep --strict` 与 `plutil -lint` 均通过。

- 功能工作树 Release 候选 SHA-256：`d94655f5fedaa93d346da48406e6b54adf1a055fb5b39069d895c027524c6f68`；
- 合并后 `main` Release SHA-256：`dde926493ecc438d061d2dcbf9606da85aa50f8e59a3af1d615a56e5ce89ca96`；
- 最终安装版 SHA-256：`dde926493ecc438d061d2dcbf9606da85aa50f8e59a3af1d615a56e5ce89ca96`；
- 原安装版可恢复备份：`/private/tmp/AI Meter.app.pre-visual-polish-20260831-1527`；
- 功能工作树安装候选备份：`/private/tmp/AI Meter.app.pre-main-build-20260831-1536`。

功能工作树和主目录的绝对构建路径不同，因此 Swift Release 二进制哈希不同。合并后重新从 `main` 构建、签名和安装，最终安装版与 `main/dist` 严格一致。

真实安装版验收结果：

- 右贴边和左贴边均与屏幕零间距，上下长切线平滑进入主体，浅色背景上没有旧黑影；
- 左右轮廓正确镜像，验收后恢复默认 Right；
- Settings 菜单可正常打开设置窗口，Automatic/Left/Right 与 8 秒自动隐藏选项仍可用；
- 只读演示模式显示 Claude 黄、Codex 玫红紫、DeepSeek 薄荷紫三个可辨圆环；
- 演示数据中的 Claude 73% 正确触发 warning 黄，真实数据中的 DeepSeek 缓存超时正确使用 stale 橙，证明语义状态会覆盖品牌色；
- Codex 详情可由 Logo 打开；自动隐藏保持 8 秒设置并由既有会话测试覆盖，三个详情组件均通过编译和共享颜色解析器的真实 SwiftUI 渲染测试；
- 安装版已恢复真实采集模式，未在日志中保存凭证、OAuth URL、Cookie 或账户原始响应。
