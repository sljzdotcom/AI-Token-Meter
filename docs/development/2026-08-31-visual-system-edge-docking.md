# 贴边浮岛与统一视觉开发日志

## 背景与目标

旧悬浮条使用 `84 × 300` 的圆角矩形，并在屏幕右侧保留 12 点空白；三个品牌资源的透明留白不同，因此使用同一几何尺寸时视觉大小并不一致。详情页也分别硬编码黑色背景、卡片明度和强调色，缺少统一层级。原 App Icon 内含 `AI` 文字，与产品含义容易混淆。

本阶段按确认规格完成：

- 纯深色玻璃与青绿至蓝紫重点色；
- 无图片背景；
- 左右可镜像、无边框和无接缝的贴边浮岛；
- Automatic / Left / Right 三种侧边模式；
- 玻璃背景拖动、多显示器选择、侧边和垂直位置记忆；
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

原视觉阶段没有修改三家服务的数据采集、额度选择、余额计算、刷新周期、凭证管理或缓存格式。后续浮岛回归的真实安装验收只增加了 DeepSeek 密钥读取的调度超时和启动隔离；密钥存储位置、内容、额度算法与缓存格式仍未改变。

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

拖动从浮岛 Shape 内、三个 Logo 圆环以外的任意玻璃空白处开始；顶部短横不再存在。Automatic 允许横向跨侧，固定模式忽略横向位移但继续响应纵向移动。松手保存屏幕编号、最终侧边和归一化垂直位置；显示器消失时回退到可用屏幕。

### 视觉与可访问性

`AIMeterVisualTheme` 集中管理深色玻璃、卡片、文字、重点色、间距和圆角。减少透明度时使用实色表面；“不只依赖颜色区分”或增强对比度开启时增加轮廓。进度环正常状态使用 mint→violet，警告和危险保留语义色；在不只依赖颜色时，缓存、警告、危险和不可用状态还显示独立状态图形，并写入 VoiceOver 文案。

玻璃背景拖动区同时支持鼠标、VoiceOver adjustable/custom action 和普通键盘方向键；动态 value 朗读实际侧边与垂直位置。详情在鼠标悬停、键盘控件聚焦、DeepSeek 登录交互或 VoiceOver 运行时暂停自动隐藏。每次 provider 切换都会重置交互状态，并以 selection ID 拒绝退休视图的迟到回调；详情关闭后 VoiceOver 焦点回到原服务按钮。

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
| `b8bd416` | 最终审查修正、可访问性、文档与完整回归 |
| `f9bff5c` | 安装、视觉与发布验证记录 |
| `90dd439` | 合并到 `main` |

## 自动化、构建与安装验收

分支收尾前的完整证据：

- `bash scripts/test.sh`：137 个测试、29 个测试组、0 个失败；4 个环境相关检查按设计跳过；
- `bash scripts/build-app.sh`：Release 构建、App Icon 生成、Bundle 组装和 ad-hoc 签名通过；
- `codesign --verify --deep --strict`：通过；
- 可执行文件：arm64 Mach-O；App Icon：1,039,493 字节 ICNS；
- 右侧和左侧真实屏幕截图复核：`108 × 356` 浮岛贴边无透明接缝，连续轮廓、阴影和 Logo 光学尺寸正确镜像；
- Markdown 相对链接和 `git diff --check`：通过。

安装验收：

- 旧 `/Applications/AI Meter.app` 已移动到可恢复备份 `/private/tmp/AI Meter.app.pre-visual-system-20260831-103946`；
- `ditto` 安装后严格签名验证通过；
- `main` 最终构建产物与安装产物的可执行文件 SHA-256 均为 `0421b42df073963aa15d95da840b6e5ee7c1a83cd58399ac278eea6bde628eb9`；
- 构建产物与安装产物的 App Icon SHA-256 均为 `480201629a240a551b1a8f346d643842d94dc6be1af408892c6744b6c251b585`；
- 已安装应用正常启动，右侧贴边浮岛可见且无旧尺寸窗口残留；
- 由于本机 ad-hoc 签名随可执行文件更新，首次真实启动会出现 Keychain 重新授权框；未代替用户输入系统密码，用户指南已记录选择 **Always Allow** 的一次性步骤。

主分支集成结果：

- 功能分支已通过 `90dd439` 合并到 `main`；
- 合并后的 `bash scripts/test.sh`：137 个测试、29 个测试组、0 个失败，4 个环境相关检查按设计跳过；
- 合并后的 Release 构建、`Info.plist`、arm64 可执行文件、ICNS 和严格代码签名验证通过；
- 最终构建已覆盖安装到 `/Applications/AI Meter.app`，安装产物与构建产物哈希一致。

## 2026-08-31 浮岛回归修复验证

### 根因与修复范围

真实安装版的回归被归为四类：批准的反向半圆路径被近似为方形肩部；唯一的顶部短横既可见又把拖动限制在过小命中区；玻璃拖动的布局/命中区在缩放与固定侧条件下没有共享同一坐标系；`LSUIElement` 菜单栏面板中的 `SettingsLink` 没有可靠地激活并置前 Settings 场景。

修复恢复了单条可镜像的 `108 × 356` 连续路径，去掉短横，以排除三个 Logo 圆环的玻璃 Shape 定义拖动起点，并让设置命令先激活应用再调用官方 `openSettings`。首次安装验收进一步证明，非激活式 `NSPanel` 仅绑定 SwiftUI 手势仍不够可靠；提交 `1866226` 因而由 AppKit 浮窗事件层接管鼠标按下、拖动和松开，同时继续复用同一纯值命中区与现有吸附布局。该提交也把启动期钥匙串读取移出主线程，并给 DeepSeek 密钥读取增加两秒超时和单次在途保护，避免系统钥匙串异常拖住 Claude、Codex 和整个界面。

提交 `81f5d34`、`99b0234`、`a475bcf`、`a71d53c`、`e5a7da2` 与 `1866226` 分别记录轮廓、背景命中、共享坐标/固定侧、设置呈现以及真实拖动/启动韧性的实现。

### 自动化与 Release 证据

- `bash scripts/test.sh`（2026-08-31 最终候选）：148 个测试、33 个测试组、0 个失败；3 个依赖本机已安装 CLI 或钥匙串环境的 smoke 检查按设计跳过。
- 新增回归覆盖：AppKit/SwiftUI 坐标转换与拖动位移、Logo/透明区排除、启动初始化不读取钥匙串，以及阻塞密钥读取在 2 秒内独立超时。
- `bash scripts/build-app.sh`：Release Swift 构建、图标生成、Bundle 组装与 ad-hoc 签名通过。
- `codesign --verify --deep --strict --verbose=2 "dist/AI Meter.app"`：有效且满足 designated requirement；`plutil -lint`：`OK`。
- `file`：`AIMeterApp` 为 `Mach-O 64-bit executable arm64`；`AppIcon.icns` 为 1,039,493 字节的 macOS ICNS。

### 安装与真实界面验收

最终安装前的应用被可恢复地移至 `/private/tmp/AI Meter.app.pre-secret-timeout-20260831-1301`，然后用 `ditto` 安装 Release Bundle；更早的阶段性安装同样保留在 `/private/tmp`。安装后的严格签名通过。构建与安装的 SHA-256 一致：可执行文件均为 `612268186b9c181677c5b5a4a1f271512ddecc9893b33a5c901e4009e70694d8`，App Icon 仍为 `480201629a240a551b1a8f346d643842d94dc6be1af408892c6744b6c251b585`。

在已安装应用中实际检查右、左两侧：两种方向均显示连续的反向半圆玻璃轮廓，没有方块、框线、接缝或透明贴边；Claude 圆环上方没有短横。依次点击 Claude、Codex、DeepSeek Logo 时，三者均打开各自详情，未把点击解释为拖动。菜单栏的 Settings 命令首次打开 `AI Meter Settings`，再次执行仍回到同一 `com_apple_SwiftUI_Settings_window` 场景。Automatic 模式下，应用公开的辅助功能边缘动作将位置从 Left 切到 Right，垂直动作将位置从 50% 调至 60%；退出并重新启动后仍恢复为 Right / 60%。

最终安装版使用真实指针事件完成三处拖动：顶部玻璃把垂直位置从 60% 移到 48%，第一、第二圆环之间从 48% 移到 41%，底部玻璃从 41% 移到 53%；最终候选再次从顶部把 53% 移到 45%。点击 Codex Logo 后详情状态由关闭变为打开，位置保持 45%，证明 Logo 点击没有被解释为拖动。悬停回调在拖动期间会检查 `isDragging`，不会用箭头或张手光标覆盖闭手状态。

正常模式安装后界面立即出现；后台 DeepSeek 钥匙串读取在本机发生等待时，两秒后只将 DeepSeek 标记为缓存数据/请求超时，Claude 0%、Codex 4% 和 DeepSeek 缓存余额 ¥77.99 均正常展示，不再长期停在三项 `Refreshing`。这次处理没有读取、打印、迁移或删除 API Key。

## 安全与隐私

- 浮岛偏好只保存非敏感几何状态；
- App Icon 生成不依赖网络或随机输入；
- 详情改版不改变 CLI 凭证、Keychain、WebKit Cookie 或业务缓存边界；
- 文档和截图不得包含真实 API Key、Cookie、授权头或未去敏账户响应。

## 已知边界

- 拖动跨越未连接的显示器空隙时，松手会选择最后可解析的屏幕；
- 图标采用本机 ad-hoc 签名打包，公开分发仍需要 Developer ID 和 Apple 公证；
- 浮岛 Shape 和 Logo 倍率以当前三份品牌资源为基准，替换源资源后应重新做视觉验收。
