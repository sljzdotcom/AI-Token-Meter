export const displayFonts = ["Microsoft YaHei", "SimHei", "KaiTi", "System Default", "Antonio", "DIN Condensed", "Alimama FangYuanTi VF", "Fira Code", "Leigo", "Menlo", "Alimama DaoLiTi"]
const system = "'Segoe UI Variable', 'Segoe UI', sans-serif"
// FontFaceSet.check accepts fallback-only families, so compare real glyph measurements instead.
export function fontAvailable(font: string, measure?: (stack: string) => number): boolean {
  if (font === "System Default") return true
  if (!measure) {
    const canvas = document.createElement("canvas")
    const context = canvas.getContext("2d")
    if (!context) return false
    measure = stack => { context.font = `72px ${stack}`; return context.measureText("用量微软雅黑黑体楷体MWil0123456789").width }
  }
  return ["monospace", "serif"].some(fallback => measure!(`"${font}", ${fallback}`) !== measure!(fallback))
}
export function displayFontStack(font: string, available = true) {
  if (font === "System Default") return system
  const selected = displayFonts.includes(font) && available ? `'${font}', ` : ""
  return `${selected}'Microsoft YaHei', ${system}`
}
