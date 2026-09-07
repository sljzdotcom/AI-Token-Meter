import { fireEvent, render, screen } from "@testing-library/react"
import { expect, it } from "vitest"
import { SettingsWindow } from "./SettingsWindow"

it("shows the bundled app logo beside the accessible Settings title", () => {
  render(<SettingsWindow displayFont="System Default" onDisplayFontChange={() => {}} />)

  const dialog = screen.getByRole("dialog", {name: "AI Token Meter Settings"})
  expect(dialog.querySelector('header img')).toHaveAttribute("aria-hidden", "true")
  expect(dialog.querySelector('header img')).toHaveAttribute("src", expect.stringContaining("128x128.png"))
  expect(screen.getByText("AI Token Meter", {selector: "header strong"})).toBeVisible()

  fireEvent.click(screen.getByRole("tab", {name: "About"}))
  expect(screen.getByRole("link", {name: "GitHub"})).toBeVisible()
})
