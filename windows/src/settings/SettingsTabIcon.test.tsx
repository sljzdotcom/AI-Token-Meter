import { act, fireEvent, render, screen } from "@testing-library/react"
import { afterEach, describe, expect, it } from "vitest"
import { setLocale } from "../localization"
import { SettingsWindow } from "./SettingsWindow"

afterEach(() => { setLocale("en") })

const renderSettings = () => render(
  <SettingsWindow displayFont="System Default" onDisplayFontChange={() => {}} />,
)

describe("Windows Settings tab icons", () => {
  it.each([
    ["Appearance", "appearance"],
    ["Monitoring", "monitoring"],
    ["Services", "services"],
    ["About", "about"],
  ])("keeps the English %s tab name while showing its decorative icon", (name, icon) => {
    renderSettings()

    const tab = screen.getByRole("tab", { name })
    const svg = tab.querySelector(`svg[data-settings-tab-icon="${icon}"]`)
    expect(svg).toBeVisible()
    expect(svg).toHaveAttribute("aria-hidden", "true")
    expect(svg).toHaveAttribute("width", "16")
    expect(svg).toHaveAttribute("height", "16")
    expect(svg).toHaveAttribute("stroke", "currentColor")
  })

  it.each(["外观", "监测", "服务", "关于"])(
    "keeps the Chinese %s tab name without exposing the icon to assistive technology",
    (name) => {
      act(() => setLocale("zh-CN"))
      renderSettings()

      const tab = screen.getByRole("tab", { name })
      expect(tab.querySelector("svg")).toHaveAttribute("aria-hidden", "true")
    },
  )

  it("preserves mouse and keyboard tab navigation", () => {
    renderSettings()

    fireEvent.click(screen.getByRole("tab", { name: "Monitoring" }))
    expect(screen.getByText("Refresh interval")).toBeVisible()

    const monitoringTab = screen.getByRole("tab", { name: "Monitoring" })
    monitoringTab.focus()
    fireEvent.keyDown(monitoringTab, { key: "ArrowRight" })
    expect(screen.getByRole("tab", { name: "Services" })).toHaveAttribute("aria-selected", "true")
    expect(screen.getByRole("button", { name: "Save DeepSeek API Key" })).toBeVisible()
  })
})
