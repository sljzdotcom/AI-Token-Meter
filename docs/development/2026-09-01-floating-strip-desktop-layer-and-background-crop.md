# 浮动条桌面层与背景裁切开发、构建和验收日志

**日期：** 2026-09-01

**分支：** `codex/desktop-only-floating-strip`

**设计：** [桌面层与肩部背景连续性规格](../superpowers/specs/2026-09-01-floating-strip-desktop-layer-and-background-crop-design.md)

**计划：** [桌面层与背景裁切实现计划](../superpowers/plans/2026-09-01-floating-strip-desktop-layer-and-background-crop.md)

**状态：** 实现、自动化验证、Release 构建、候选安装和部分本机验收完成；普通/全屏 Edge 的跨 App 合成层级、Mission Control、真实指针拖动和多显示器仍需人工环境补验。本阶段不合并 `main`。

## 1. 背景、根因与目标

旧实现把 strip 和 detail 面板放在 `.floating` 层并加入 `.fullScreenAuxiliary`，导致它们可能覆盖普通应用、跟进全屏 Space，甚至在 Mission Control 过渡中出现在不应出现的位置。Space 变化也没有统一结束详情会话。深海背景则只做水平方向镜像，没有用同一比例覆盖上下肩部，黑色 gutter 会进入可见轮廓。

本阶段把两个面板统一降到桌面图标层之上、普通窗口层之下，保留普通 Space 的桌面可见性但移除全屏辅助行为；监听活动 Space 变化并关闭详情，同时不改写保存位置；背景改用 `1.22×` 等比裁切，左侧只反转 X，原始 PNG 只读。

## 2. 实现与提交

- `458ab2f fix: place floating panels on desktop layer`
  - 新增 `FloatingPanelPresentationPolicy`；level 为 `desktopIconWindow + 1`，低于 `.normal`。
  - 集合行为为 `.canJoinAllSpaces`、`.stationary`、`.ignoresCycle`，不含 `.fullScreenAuxiliary`。
  - strip 和 detail 经同一入口应用策略。
- `5837fa6 fix: dismiss floating detail across Space changes`
  - 新增可注入、可注销的 `ActiveSpaceChangeObserver`。
  - Space 变化关闭详情、撤销 responder、隐藏详情并重新定位。
- `62a5060 fix: preserve floating placement across Space changes`
  - 审查发现保存屏幕暂不可用时，普通重定位路径会持久化恢复默认位置。
  - 新增 `FloatingStripRecoveryPolicy`；Active Space 路径只重排现场面板，禁止写回屏幕、侧边与垂直位置。
- `a83467d fix: crop strip background across both shoulders`
  - 背景合成改为 `1.22×` 等比缩放，左右只在 X 方向镜像。
  - 玻璃 fallback、图片和 scrim 在一个 `ZStack` 内合成，再用完整 `FloatingStripShape` 统一裁切。

## 3. TDD 红绿证据

窗口策略测试先以 `cannot find 'FloatingPanelPresentationPolicy' in scope` 编译失败；最小策略加入后，3 项策略测试与 4 项交互面板测试通过。

Space 观察器测试先以 `cannot find 'ActiveSpaceChangeObserver' in scope` 编译失败；实现后 2 项通知传递/注销测试通过。首次独立审查发现 Active Space 路径在屏幕缺失边界下可能写回默认位置；新增策略测试先以 `cannot find 'FloatingStripRecoveryPolicy' in scope` 失败，修复后 2 项恢复决策测试通过，定向复审确认原重要问题已解决。

背景任务先因 `FloatingStripBackgroundPresentation.scale` 不存在而编译失败；加入合同但保留旧渲染时，真实 `ImageRenderer` 的上下肩部 8 个蓝色采样断言失败。统一等比裁切后，18 项视觉测试、4 项拖动形状测试和 4 项指针拖动状态测试通过。

任务 1、任务 2 修复和任务 3 的独立审查最终均无阻断或重要问题。原始阶段报告与审查记录保存在本地 `.superpowers/sdd/2026-09-01-floating-strip-desktop-layer-and-background-crop/`。

## 4. 完整自动化验证

首次候选验证命令：

```bash
/usr/bin/time -p bash scripts/test.sh
```

结果：179 项测试、38 个测试套件通过，0 失败，耗时 `real 9.73s`。4 项环境门控检查按设计跳过：1 项真实 Keychain 隔离读写，以及 Claude auth、Claude collector、Codex collector 三项已安装 CLI 冒烟检查。WebKit/Launch Services 的受限缓存警告不影响退出码或测试结论。

静态合同与资产保护：

```bash
rg -n 'fullScreenAuxiliary|panel\.level = \.floating' Sources/AIMeterApp/System
shasum -a 256 Sources/AIMeterApp/Resources/Backgrounds/floating-strip-deep-sea.png
git diff --exit-code -- Sources/AIMeterApp/Resources/Backgrounds/floating-strip-deep-sea.png
```

- `rg` 退出码为 1、无命中，证明目标目录不再保留旧符号。
- 当前资产与基线提交 `374ffd6` 的 SHA-256 都是 `43ae960bf58a5ddcf2b416362c00d7bfcdcc5764f9af50316285454b2a813b6d`。
- 资产 diff 退出码为 0；Git 变更不包含该 PNG。

## 5. Release 构建、签名与安装身份

复用项目既有流程：

```bash
/usr/bin/time -p bash scripts/build-app.sh
codesign --verify --deep --strict --verbose=2 "dist/AI Meter.app"
plutil -lint "dist/AI Meter.app/Contents/Info.plist"
file "dist/AI Meter.app/Contents/MacOS/AIMeterApp"
```

- Release 构建退出码：0；脚本报告构建耗时 8.02 秒，总耗时 `real 9.41s`。
- 候选 bundle：`dist/AI Meter.app`。
- 签名：ad-hoc；严格验证为 `valid on disk`、`satisfies its Designated Requirement`。
- Info.plist：`OK`；可执行文件为 `Mach-O 64-bit executable arm64`。
- 可执行文件大小：2,923,504 bytes。
- 候选可执行文件 SHA-256：`4fc3cbc757e044710346fa27d52454ceb81a4f975c637d0bb81adf1276cf31ac`。
- 候选 bundle 文件清单指纹：`f338b31d8b65baaadf1e6c3a1c47cc9f63ca5c9d9e0c860172e9bd08e15f27c3`。

安装前完全退出 `AIMeterApp`，把旧版移动到可恢复位置：

```text
/private/tmp/AI-Meter-app-backup-20260901-0832-task4/AI Meter.app
```

随后用 `ditto` 安装到 `/Applications/AI Meter.app`。安装版再次通过严格签名与 plist 校验；候选与安装版主可执行文件 SHA-256 都是 `4fc3cbc757e044710346fa27d52454ceb81a4f975c637d0bb81adf1276cf31ac`，`cmp` 退出码为 0；安装版 bundle 文件清单指纹同为 `f338b31d8b65baaadf1e6c3a1c47cc9f63ca5c9d9e0c860172e9bd08e15f27c3`，目录级 `diff -qr` 无输出。

## 6. 本机验收矩阵

Computer Use 只能可靠捕获目标 App 的窗口/辅助功能树，不能把其他 App 的非激活 `NSPanel` 合成进 Edge、Finder 或 Mission Control 截图。下表只把直接观察到的事实标为通过；没有跨 App 合成证据的层级项目不推断为通过。

| 项目 | 结果 | 直接证据 |
| --- | --- | --- |
| Finder 桌面与三枚 Logo | 通过 | Finder 辅助功能树显示 `Desktop`；已安装 App 显示 Claude、Codex、DeepSeek 三个按钮，三者分别点击后都进入 `Detail open`，切换 Provider 会关闭前一详情。 |
| 普通 Edge 覆盖 strip/detail | 环境未覆盖，需人工 | Edge 已恢复普通窗口，但 app-window capture 不合成 AI Meter 的非激活面板，无法用该截图可靠证明 WindowServer 最终层级。自动化策略测试已证明 level 低于 `.normal`。 |
| Edge 全屏完全看不到 strip/detail | 环境未覆盖，需人工 | 已通过 Edge `Enter Full Screen`/`Exit Full Screen` 菜单完成真实全屏过渡；capture 仍不合成其他 App 面板，不能把画面中“未出现”当作充分证据。自动化测试证明集合行为不含 `.fullScreenAuxiliary`。 |
| 返回桌面保留 Antonio、右侧、97% | 通过 | 设置窗口显示 Display font = Antonio、Right 已选；浮岛辅助功能详情为 `Right edge, vertical position 97 percent`；最终 defaults 为 `antonio`、`right`、约 `0.97`。 |
| Space 切换关闭详情 | 通过 | Claude 先为 `Detail open`；Edge 进入全屏 Space 后，AI Meter 辅助功能状态变为三个详情全部 `closed`；位置仍为右侧 97%。 |
| Mission Control | 环境未覆盖，需人工 | Computer Use 调用 Mission Control 超过 6 分钟无返回并被中止；未获得可核实截图，不再重试。 |
| 左右贴边上下肩部 | 通过 | 通过辅助功能自定义动作切到 Left、再恢复 Right；两侧捕获图均直接显示上下肩部连续蓝色波纹，Logo 与圆环方向不镜像。 |
| Logo、圆环、点击 | 通过 | 三枚 Logo 和品牌圆环可见；Claude、Codex、DeepSeek 点击均打开对应详情。 |
| 指针拖动 | 环境未覆盖，需人工 | Computer Use 对非激活 `NSPanel` 的坐标拖动返回 `noWindowsAvailable`。辅助功能 Decrement/Increment 已将位置从 97% 移到 87% 再恢复 97%，自动化拖动形状与指针状态测试通过，但这些不等同于真实指针拖动。 |
| Settings | 通过 | `⌘,` 打开真实 Settings；Show floating meter 开启、Right 选中、Antonio、8 seconds 均可见。 |
| 自动隐藏 | 通过 | Claude 从 `Detail open` 在 9.5 秒后变为 `Detail closed`，符合 8 秒偏好。 |
| 多显示器 | 当前环境未覆盖 | 当前自动化环境没有暴露可操作的第二显示器，未执行跨屏/插拔验收。 |

## 7. 安全、隐私与偏好恢复

- 验收命令和 Computer Use 没有直接读取或输出 DeepSeek API Key、浏览器 Cookie、OAuth Token 或完整账户响应；候选 App 按正常启动流程可能从 Keychain 读取密钥。
- 没有主动修改 Keychain 或服务登录状态；候选 App 的正常刷新可能更新本地用量缓存。安装保留现有 `UserDefaults`，仅在验收中短暂改变并恢复下述界面偏好。
- 验收过程中只短暂切换 Left/Right 和以辅助功能步进移动位置，结束前恢复用户原有 Antonio、Right、97%，Show floating meter 保持开启，自动隐藏保持 8 秒。
- Edge 验收结束后已退出全屏并恢复普通窗口。
- 旧安装 bundle 保存在上述 `/private/tmp` 路径，可用于恢复。

## 8. 文档后的完成前门控

全量测试和 Release 构建均在本任务当前执行中取得上述新鲜证据；根据上层在 Mission Control 调用中止后的明确指示，没有无意义地重复同一测试/构建。文档完成后重新执行静态、资产、签名、安装身份、偏好和工作区门控：

- 9 个目标文档全部存在；`git diff --check` 退出码 0。
- 旧符号仍零命中；PNG SHA-256 仍为 `43ae960b…13b6d`，无 diff。
- 候选与安装版严格签名、Info.plist 再次通过。
- 候选/安装可执行文件 SHA-256 仍同为 `4fc3cbc…31ac`；bundle 清单指纹仍同为 `f338b31d…27c3`；`cmp` 与 `diff -qr` 均退出码 0/无输出。
- 最终偏好仍为 Antonio、Right、约 97%，Show floating meter 开启，自动隐藏 8 秒。
- 工作区只包含任务简报列出的 9 个文档文件，没有生产代码、测试或 PNG 改动。

## 9. 结论与剩余工作

R6 的完成标准已有自动化像素、等比合同、左右实图和资产保护证据，标记完成。R2 的实现与自动化合同已完成，Space 详情关闭也通过本机验收；但普通/全屏 Edge 的跨 App 合成层级、Mission Control 和多显示器未取得可靠直接证据，因此 R2 暂不标记完整验收完成。

当前结论为 `DONE_WITH_CONCERNS`：没有观察到实质功能失败，但发布/合并前仍需在可直接观察整个桌面合成结果的人工环境中补验普通 Edge、Edge 全屏、Mission Control、真实指针拖动和多显示器（如有）。本任务按上层要求不合并 `main`。
