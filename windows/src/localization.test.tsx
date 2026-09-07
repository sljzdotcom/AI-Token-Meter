import { act, fireEvent, render, screen } from "@testing-library/react"
import { afterEach, expect, it } from "vitest"
import { SettingsWindow } from "./settings/SettingsWindow"
import { ProviderDetail } from "./details/ProviderDetail"
import { setLocale, t, dictionaries } from "./localization"
import { displayFontStack, fontAvailable } from "./displayFonts"

afterEach(() => { setLocale("en") })

it("updates already open Settings and detail text immediately without changing the font choice", () => {
  render(<><SettingsWindow displayFont="Antonio" onDisplayFontChange={() => {}} />
    <ProviderDetail snapshot={{schemaVersion: 1, providerId: "claude", displayName: "Claude Code", status: "unavailable", fetchedAt: "2026-09-07T00:00:00Z", staleAfterSeconds: 300}}
      onPointerEnter={() => {}} onPointerLeave={() => {}} onInteractionStart={() => {}} onInteractionEnd={() => {}} /></>)
  expect(screen.getByRole("tab", { name: "Appearance" })).toBeInTheDocument()
  act(() => setLocale("zh-CN"))
  expect(screen.getByRole("tab", { name: "外观" })).toBeInTheDocument()
  expect(screen.getByRole("combobox", { name: "显示字体" })).toHaveValue("Antonio")
  expect(screen.getByText("官方额度")).toBeInTheDocument()
  expect(screen.getAllByText("暂不可用").length).toBeGreaterThan(0)
  fireEvent.click(screen.getByRole("tab", { name: "监测" }))
  expect(screen.getByText("刷新间隔")).toBeInTheDocument()
})

it("uses complete dictionaries and falls back to English for unknown preferences", () => {
  expect(Object.keys(dictionaries.en).sort()).toEqual(Object.keys(dictionaries["zh-CN"]).sort())
  setLocale("unsupported")
  expect(t("Appearance")).toBe("Appearance")
  setLocale("zh-CN")
  expect(t("Last {days} days · This PC", {days: 30})).toBe("最近 30 天 · 本机")
})

it("compares actual glyph metrics against both generic fallbacks to detect missing fonts", () => {
  const measure = (font: string) => font.startsWith('"KaiTi"') ? 250 : font.endsWith("monospace") ? 200 : 300
  expect(fontAvailable("KaiTi", measure)).toBe(true)
  expect(fontAvailable("SimHei", measure)).toBe(false)
  expect(displayFontStack("SimHei", false)).toContain("Microsoft YaHei")
  expect(displayFontStack("SimHei", false)).not.toContain("SimHei")
})

it("shows the borrowed primary display's edge while preserving the offline selection", () => {
  render(<SettingsWindow displayFont="Menlo" onDisplayFontChange={() => {}}
    displays={{version:1,mode:"selected",selectedId:"offline",placements:{primary:{edge:"left",verticalPerMille:200},offline:{edge:"right",verticalPerMille:700}}}}
    availableDisplays={[{id:"primary",name:"Built-in",isPrimary:true}]} />)
  expect(screen.getByRole("combobox", {name:"Display"})).toHaveValue("offline")
  expect(screen.getByRole("combobox", {name:"Screen edge"})).toHaveValue("left")
})
