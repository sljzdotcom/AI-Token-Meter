# 双平台展开态顶部横线移除

关联 REQ-20260908-011，日期2026-09-08。用户已批准两端去掉顶部小横线，保留原拖动行为及折叠态竖线，暂不发布。

## 基线与设计

从REQ-010最终交付 `6917af8` 接续，在独立分支 `codex/remove-strip-drag-hint` 以 `e843750` 引入批准 `4cf994a`；冲突只添加011批准行，完整保留010已完成记录与009协调证据。规格/计划检查点 `393a7eb`。

- [规格](../design/specifications/2026-09-08-remove-strip-drag-hint-design.md)
- [计划](../design/implementation-plans/2026-09-08-remove-strip-drag-hint.md)

macOS顶部18×3 Capsule关闭命中；Windows装饰为aria-hidden，拖动事件在父容器。删除范围仅为这两处装饰及其专用样式，不改背景、轮廓、窗口尺寸、Logo布局、Provider点击、贴边/多屏/位置持久化或折叠态3×40竖线。

## 验证与交付

初步红绿证据已核对：Windows新矩阵4项因旧装饰元素失败，修复后11项组件测试通过；macOS实际View与实际Surface顶部区域比较，在四种布局各检测到212个装饰像素而失败，删除后49项原生渲染/拖动/布局等回归通过。原始日志`/private/tmp/req011-windows-red.log`、`req011-windows-green.log`、`req011-swift-red.log`、`req011-swift-green.log`。

控制者另用应用内浏览器在1280×720查看实际comparison页面前后截图：四种布局横线均消失，外轮廓/背景/Logo肉眼一致。四个浮动条和12个Provider按钮的完整boundingRect在前后逐项完全相同（紧凑78×286、环48；舒适108×356、环60），装饰DOM数为0。这是实际浏览器渲染核验；背景拖动与Provider回调隔离由组件事件回归验证。

实现检查点 `e963301`：生产仅删5行Swift装饰、1行DOM和13行专用CSS。控制者查看原生 `/private/tmp/req011-native-red/expanded-compact-left.png`、对应green以及`folded-compact-left.png`，确认顶部横线消失且折叠竖线保留。四种布局原生渲染均有PNG证据；ImageRenderer未绘制原生Button内容，故原生截图只用于装饰区域核验，不能据此声称完整Logo视觉验收。Provider布局/命中依赖相关原生回归，Windows完整Logo外观由上述浏览器截图确认。

Windows前端全量96项和生产构建已通过。首轮全量Swift在与Rust冷编译并行时，两个未修改CLI用例失败：Claude usage `.timedOut`（5.051秒），Codex app server强制终止用例 `.transportFailure`（1.764秒）；429项主测试整体失败，原始证据`/private/tmp/req011-swift-full.log`保留。正在隔离诊断及顺序复验，不据并行负载直接断言根因、不删除断言或以部分通过冒充全量通过；历史时序不稳定见REQ-20260906-003。

冷编译结束后，原CLICollectorTests隔离15/15通过（15.193秒）；顺序重跑完整Swift得到429项主测试/84套件加13项PTY，共442项通过，完整脚本退出0；6个既有环境门控跳过保持原状态，本次未新增跳过。后续合同、可移植性、资产标准化、feed、196份文档与公开安全门禁均通过。分别保留`/private/tmp/req011-swift-cli-isolated.log`和`/private/tmp/req011-swift-full-sequential.log`；未修改CLI源码、超时或断言，不将历史REQ-20260906-003标成已修复。

宿主Rust首次6项失败均为本机夹具无法绑定localhost（权限拒绝）；获得本机监听权限后，同一代码完整运行221项通过。保留`/private/tmp/req011-rust-full.log`和`/private/tmp/req011-rust-full-loopback.log`。macOS Release App/Widget构建通过（17.21秒，`/private/tmp/req011-swift-release.log`）。

复现命令（首次和最终结果按上文区分）：

```sh
scripts/check-docs.sh
AI_METER_TEST_BUILD_DIR=/private/tmp/req011-swift scripts/test.sh
npm --prefix windows test
npm --prefix windows run build
cargo test --offline --locked --manifest-path windows/src-tauri/Cargo.toml
CLANG_MODULE_CACHE_PATH=/private/tmp/req011-swift/clang-module-cache swift build --disable-sandbox --cache-path /private/tmp/req011-swift/swiftpm-state/cache --config-path /private/tmp/req011-swift/swiftpm-state/config --security-path /private/tmp/req011-swift/swiftpm-state/security --scratch-path /private/tmp/req011-swift/build -c release
```

独立任务审查与最终整分支 `6917af8..abf0c1f` 审查均为0 Critical/Important/Minor。控制者核对原始日志、差异、196份文档门禁与台账，本需求2026-09-08完成开发，待协调入口整合；本分支保留REQ-010完整交付。生产提交`e963301`，规格`393a7eb`，过程证据`58b5259`、`abf0c1f`。临时浏览器页及本机测试服务已关闭。

原生测试使用独立UserDefaults及dummy SecretStore，不调用start/采集/同步；AppModel构造仍沿用既有历史读取及WKWebView创建，不能描述为完全不访问本机应用存储，截图也不呈现其历史数据。本机浏览器/宿主Rust验证与原生Windows WebView2/DPI现场验收分别记录，不以宿主测试替代原生Windows结果。

本次不发布、安装替换应用或操作真实账号；0.5.1、版本、签名和更新源保持原状态。由协调入口整合010/011，本开发分支不直接合并main。

## 主分支整合交接（REQ-20260908-013）

用户将全部验证、审查、Git整合与收尾职责交给开发入口；开发入口接管主工作区已开始的 `dbd8da1` 合并 `4cb5778`，不重新merge/abort/reset。暂存产品树与已审查交付完全一致，保留Gemini012及新协作规则。

此次接管沿用同一合并结果已完成的验证，控制者读取原始证据并检查产品差异：完整Swift429主测试+13PTY=442、合同/脚本/196份文档/公开安全门禁通过（`/private/tmp/ai-meter-coordinator-011-full.log`）；交接报告前端96项、生产构建、真实Chrome密度门禁21个生命周期用例和632文本角色通过；Release编译17.29秒通过（`/private/tmp/ai-meter-coordinator-011-release.log`）。这些检查在职责修正前由协调入口启动，此后不再由协调入口重复验证。开发入口仅补文档并再次运行文档门禁；原生Windows及既有CLI时序边界保持。暂不发布、安装或修改版本/更新源。

开发入口最终合并提交：`0a00aae`；2026-09-08 已完成本地main整合与职责规则提交。保留原开发分支及应用管理的工作区，供下一阶段研究使用。
