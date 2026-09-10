# Compact 圆环两侧留白减半

关联`REQ-20260910-006`，规格见[设计说明](../design/specifications/2026-09-10-compact-strip-padding-halving-design.md)，步骤见[实施计划](../design/implementation-plans/2026-09-10-compact-strip-padding-halving.md)。

## 精确变化

Compact的48pt/px圆环保持不变，逻辑宽度由65改为56.5，因此每侧留白从8.5精确减为4.25。1至4个Provider的高度仍为170/228/286/344，间距仍为10；Logo、线宽、Comfortable 108宽和折叠把手12宽不变。macOS和Windows使用同一逻辑尺寸，Windows再按DPI换算到最近物理像素。

轮廓的屏幕贴边端仍在x=0，内侧端移至x=56.5。与圆环中心相关的肩部控制点左移4.25，与内侧边界相关的端点收进8.5；上下曲线共用镜像数值，左右贴边继续从同一路径水平翻转。

## 失败先行与专项证据

测试先改为56.5时，macOS尺寸测试收到旧65和旧8.5单侧留白；Windows前端5项收到旧`65px`；Windows原生3项收到旧`65.0`。这些失败只指向Compact横向合同，圆环和高度断言未改变。

实现后，macOS 36项偏好、路径、布局与2×渲染专项通过；Windows前端17项、production build及原生偏好9项通过。真实Chrome又验证16组左右贴边、1至4个Provider的56.5宽、4.25单侧留白、按钮无裁切、点击顺序和一次玻璃拖动，并同时复验Antigravity状态及详情底板。

同倍率2×渲染中，旧65pt图为130×572像素，新56.5pt图为113×572像素；48pt圆环与572像素总高保持。对比图见[Compact留白前后对比](../assets/screenshots/compact-strip-padding-before-after.png)。

实现提交为`76ac9e9`。物理Windows 11的多屏、真实指针和不同显示器DPI继续属于现场观察边界；自动化已覆盖1×、1.25×、1.5×、2×尺寸换算与吸附位置。
