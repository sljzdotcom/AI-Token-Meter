# 贴边浮岛与统一视觉开发日志

## 背景与目标

旧悬浮条使用 `84 × 300` 的圆角矩形，并在屏幕右侧保留 12 点空白；三个品牌资源的透明留白不同，因此使用同一几何尺寸时视觉大小并不一致。详情页也分别硬编码黑色背景、卡片明度和强调色，缺少统一层级。原 App Icon 内含 `AI` 文字，与产品含义容易混淆。

本阶段按确认规格完成：

- 纯深色玻璃与青绿至蓝紫重点色；
- 无图片背景；
- 左右可镜像、无边框和无接缝的贴边浮岛；
- Automatic / Left / Right 三种侧边模式；
- 拖动柄、多显示器选择、侧边和垂直位置记忆；
- 统一三种详情页表面与卡片层级；
- Claude、Codex、DeepSeek Logo 集中光学校正；
- 无文字仪表指针 App Icon。

设计规格见 [视觉系统与贴边浮岛设计](../design/specifications/2026-08-31-visual-system-edge-docking-design.md)，实施步骤见 [测试驱动实施计划](../design/implementation-plans/2026-08-31-visual-system-edge-docking.md)。

## 影响范围

- `AIMeterCore/Preferences`：跨平台的侧边偏好、最后侧边、显示器标识和归一化垂直位置存储；
- `AIMeterApp/System`：纯几何布局、共享显示状态、窗口拖动、多显示器和详情内向展开；
- `AIMeterApp/Views`：视觉主题、自定义浮岛轮廓、Logo 光学校正、统一圆环和三种详情页；
- `scripts` 与 `Info.plist`：确定性 App Icon 绘制、ICNS 容器和 Bundle 声明；
- 用户指南、架构、测试、变更和提交历史文档。

没有修改三家服务的数据采集、额度选择、余额计算、刷新周期、凭证管理或缓存格式。

## TDD 证据

### 位置模型

首次运行 `FloatingStripPositionTests` 时类型不存在；实现后覆盖默认 `automatic + right + 0.5`、完整保存再读取、未知枚举回退和垂直位置夹紧。最终审查补充了 NaN 与正负无穷回归测试，非有限坐标统一回到 `0.5`。

### 布局和拖动

首次运行 `FloatingStripLayoutTests` 时布局器不存在；后续新增固定侧拖动测试时，`resolvedPlacement` 再次按预期编译失败。实现后覆盖：

- 左右零边距贴边；
- Automatic 选择最近侧边，Left/Right 覆盖横向拖动；
- 垂直位置往返和顶底夹紧；
- 详情始终向桌面内部展开；
- 小屏幕时详情缩小且不覆盖浮岛。

最终审查进一步增加“保存的显示器已断开”场景：首次回退到主屏幕、Automatic、右侧和垂直中点，并立即收敛持久化状态，之后 Settings 的 Left/Right 仍能即时生效。

测试还暴露了“小屏幕超宽详情会覆盖浮岛”的边界问题，布局器改为先计算浮岛与屏幕内侧之间的真实可用宽度，再夹紧详情尺寸。

### 视觉规则

`VisualSystemTests` 先后验证缺失的集中 Logo 缩放、浮岛 Shape、层级常量和 App Icon 声明。实现后的固定规则为：

- Claude `1.28`、Codex `1.0`、DeepSeek `0.92`；Claude 倍率在真实 60 点圆环截图检查后由初值 `1.16` 小幅上调；
- 浮岛左右路径的 `boundingRect` 都完整覆盖 `108 × 356` 窗口，贴边侧不保留透明画布；
- 面板、卡片、小胶囊圆角构成严格层级，详情内边距为 20 点；
- `CFBundleIconFile` 固定为 `AppIcon`。

渲染级测试还使用 `ImageRenderer` 检查浮岛上下左右窗口边缘的 alpha，避免只验证 Path 几何却遗漏 SwiftUI modifier 顺序造成的透明区域。

## 实现决定

### 贴边几何

窗口控制器不重复保存几何规则，而是消费 `FloatingStripLayout` 的纯值结果。浮岛使用 `108 × 356` 窗口；右侧 `maxX` 与 `visibleFrame.maxX` 相同，左侧 `minX` 与 `visibleFrame.minX` 相同。详情可见表面与浮岛间隔 9 点，并按当前屏幕可见区域夹紧。

拖动只从顶部短横开始。Automatic 允许横向跨侧，固定模式忽略横向位移但继续响应纵向移动。松手保存屏幕编号、最终侧边和归一化垂直位置；显示器消失时回退到可用屏幕。

### 视觉与可访问性

`AIMeterVisualTheme` 集中管理深色玻璃、卡片、文字、重点色、间距和圆角。减少透明度时使用实色表面；“不只依赖颜色区分”或增强对比度开启时增加轮廓。进度环正常状态使用 mint→violet，警告和危险保留语义色；在不只依赖颜色时，缓存、警告、危险和不可用状态还显示独立状态图形，并写入 VoiceOver 文案。

顶部拖动柄同时支持鼠标、VoiceOver adjustable/custom action 和普通键盘方向键；动态 value 朗读实际侧边与垂直位置。详情在鼠标悬停、键盘控件聚焦、DeepSeek 登录交互或 VoiceOver 运行时暂停自动隐藏。每次 provider 切换都会重置交互状态，并以 selection ID 拒绝退休视图的迟到回调；详情关闭后 VoiceOver 焦点回到原服务按钮。

浮岛由单条连续 Path 绘制顶部内收、中段与底部内收曲线；左侧通过同一坐标规则镜像，没有叠加端帽或额外边框。三个品牌资源不被重绘，只在 `ProviderLogoStyle` 应用集中缩放。

### App Icon

`generate-app-icon.swift` 使用 AppKit 确定性绘制 16 到 1024 像素的 PNG：深靛圆角底板、分段 mint→violet 用量环和浅色指针，不包含文字或服务商标志。脚本同时组装现代 PNG ICNS 元素；构建脚本在签名前验证图标非空并复制到 Bundle。

## Git 节点

| 提交 | 内容 |
| --- | --- |
| `515d91e` | 视觉与贴边设计规格 |
| `f120b1c`、`92bc892` | 实施计划与并发模型校正 |
| `1e2305c` | 位置偏好持久化 |
| `a5fbbc4` | 纯几何贴边布局 |
| `b210b17` | 拖动、吸附、多屏和设置联动 |
| `d49bf15` | 视觉主题、连续浮岛、Logo 光学校正 |
| `a0b07e2` | 三种详情页统一视觉 |
| `1a165d3` | App Icon 生成和打包 |

## 自动化、构建与安装验收

分支收尾前的完整证据：

- `bash scripts/test.sh`：137 个测试、29 个测试组、0 个失败；4 个环境相关检查按设计跳过；
- `bash scripts/build-app.sh`：Release 构建、App Icon 生成、Bundle 组装和 ad-hoc 签名通过；
- `codesign --verify --deep --strict`：通过；
- 可执行文件：arm64 Mach-O；App Icon：1,039,493 字节 ICNS；
- 右侧和左侧真实屏幕截图复核：`108 × 356` 浮岛贴边无透明接缝，连续轮廓、阴影和 Logo 光学尺寸正确镜像；
- Markdown 相对链接和 `git diff --check`：通过。

安装后的可执行文件/图标哈希、启动窗口复核和合并后完整回归结果在安装与主分支集成后追加。

## 安全与隐私

- 浮岛偏好只保存非敏感几何状态；
- App Icon 生成不依赖网络或随机输入；
- 详情改版不改变 CLI 凭证、Keychain、WebKit Cookie 或业务缓存边界；
- 文档和截图不得包含真实 API Key、Cookie、授权头或未去敏账户响应。

## 已知边界

- 拖动跨越未连接的显示器空隙时，松手会选择最后可解析的屏幕；
- 图标采用本机 ad-hoc 签名打包，公开分发仍需要 Developer ID 和 Apple 公证；
- 浮岛 Shape 和 Logo 倍率以当前三份品牌资源为基准，替换源资源后应重新做视觉验收。
