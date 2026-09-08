import { fireEvent, render, screen } from "@testing-library/react"
import { expect, it, vi } from "vitest"
import { SettingsWindow } from "./SettingsWindow"

it("missing service offers an enabled install action and preserves unavailable recovery", () => {
  const install = vi.fn()
  const check = vi.fn()
  render(<SettingsWindow displayFont="System Default" onDisplayFontChange={() => {}} requestedTab="Services" onBeginServiceInstallation={install} onCheckServiceStatus={check} serviceStatuses={[{providerId: "claude", connectionState: "notInstalled"}, {providerId: "codex", connectionState: "unavailable"}]} />)
  const button = screen.getByRole("button", {name: "Install CLI Claude Code"})
  expect(button).toBeEnabled()
  fireEvent.click(button)
  expect(install).toHaveBeenCalledWith("claude")
  fireEvent.click(screen.getByRole("button", {name: "Check Status OpenAI Codex"}))
  expect(check).toHaveBeenCalledWith("codex")
  expect(screen.getByText("Downloads and runs the official installer in Terminal.")).toBeVisible()
})

it("busy service blocks login, duplicate installation and runtime changes", () => {
  render(<SettingsWindow displayFont="System Default" onDisplayFontChange={() => {}} requestedTab="Services" busyServices={["claude"]} serviceStatuses={[{providerId: "claude", connectionState: "notInstalled"}]} />)
  expect(screen.getByRole("button", {name: "Waiting for Terminal… Claude Code"})).toBeDisabled()
  expect(screen.getByRole("button", {name: "Check Claude Code status"})).toBeDisabled()
  expect(screen.getByRole("button", {name: "Initialize Claude Code quota reading"})).toBeDisabled()
  expect(screen.getAllByRole("combobox")[0]).toBeDisabled()
})

it("connected Claude offers explicit quota initialization with instructions", () => {
  const initialize = vi.fn()
  render(<SettingsWindow displayFont="System Default" onDisplayFontChange={() => {}} requestedTab="Services" onInitializeClaudeUsage={initialize} serviceStatuses={[{providerId: "claude", connectionState: "connected", accountLabel: "member@example.com"}]} />)
  fireEvent.click(screen.getByRole("button", {name: "Initialize Claude Code quota reading"}))
  expect(initialize).toHaveBeenCalledOnce()
  expect(screen.getByText("Opens Claude Code in AI Token Meter’s private empty workspace. Answer any prompt yourself, then choose Check Status.")).toBeVisible()
})
