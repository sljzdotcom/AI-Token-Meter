import { act, render, screen } from "@testing-library/react"
import { afterEach, expect, it } from "vitest"
import { setLocale } from "../localization"
import { SettingsWindow } from "./SettingsWindow"

afterEach(() => setLocale("en"))

it.each([
  ["en", "Author · Miller"],
  ["zh-CN", "作者 · Miller"],
] as const)("omits the author row from the %s About panel", (locale, authorLine) => {
  act(() => setLocale(locale))
  render(
    <SettingsWindow
      displayFont="System Default"
      onDisplayFontChange={() => {}}
      requestedTab="About"
    />,
  )

  expect(screen.queryByText(authorLine)).not.toBeInTheDocument()
  expect(screen.getByRole("link", {name: /Telegram.*sljzdotcom/})).toBeVisible()
})
