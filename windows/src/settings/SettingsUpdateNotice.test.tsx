import { act, render, screen } from "@testing-library/react"
import { afterEach, describe, expect, it } from "vitest"

import { setLocale } from "../localization"
import { SettingsWindow, type UpdateState } from "./SettingsWindow"

const baseProps = {
  displayFont: "System Default",
  onDisplayFontChange: () => {},
  requestedTab: "About" as const,
}

afterEach(() => {
  act(() => setLocale("en"))
})

describe("available update notice", () => {
  it.each([
    ["en", "Version 0.6.0 is available."],
    ["zh-CN", "有新版本 0.6.0 可用。"],
  ] as const)("emphasizes the %s available message and enables updating", (locale, message) => {
    act(() => setLocale(locale))
    render(<SettingsWindow {...baseProps} updateState={{phase: "available", currentVersion: "0.5.0", availableVersion: "0.6.0"}} />)

    expect(screen.getByText(message)).toHaveClass("update-status", "update-status--available")
    expect(screen.getByRole("button", {name: locale === "en" ? "Update Now" : "立即更新"})).toBeEnabled()
  })

  it("adds emphasis only after checking finds an update and removes it for every other state", () => {
    const states: UpdateState[] = [
      {phase: "checking", currentVersion: "0.5.0"},
      {phase: "idle", currentVersion: "0.5.0"},
      {phase: "upToDate", currentVersion: "0.5.0"},
      {phase: "downloading", currentVersion: "0.5.0", progressPercent: 25},
      {phase: "installing", currentVersion: "0.5.0"},
      {phase: "failed", currentVersion: "0.5.0", message: "Update check failed."},
    ]
    const {rerender} = render(<SettingsWindow {...baseProps} updateState={states[0]} />)
    expect(screen.getByText("Updates are checked only when you ask.")).not.toHaveClass("update-status--available")

    rerender(<SettingsWindow {...baseProps} updateState={{phase: "available", currentVersion: "0.5.0", availableVersion: "0.6.0"}} />)
    expect(screen.getByText("Version 0.6.0 is available.")).toHaveClass("update-status--available")
    expect(screen.getByRole("button", {name: "Update Now"})).toBeEnabled()

    for (const state of states) {
      rerender(<SettingsWindow {...baseProps} updateState={state} />)
      expect(document.querySelector(".update-status")).not.toHaveClass("update-status--available")
      expect(screen.getByRole("button", {name: state.phase === "downloading" || state.phase === "installing" ? "Installing…" : "Update Now"})).toBeDisabled()
    }
  })
})
