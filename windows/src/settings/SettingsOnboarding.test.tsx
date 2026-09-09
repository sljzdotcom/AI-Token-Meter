import { fireEvent, render, screen, within } from "@testing-library/react"
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
  const codexCard = screen.getByText("OpenAI Codex").closest("article")!
  expect(within(codexCard).getAllByRole("button", {name: /Check/})).toHaveLength(1)
  expect(screen.getByText("Downloads and runs the official installer in Terminal.")).toBeVisible()
})

it("uses Save for a first DeepSeek Key and Replace only when one is already stored", () => {
  const {rerender} = render(<SettingsWindow displayFont="System Default" onDisplayFontChange={() => {}} requestedTab="Services" serviceStatuses={[{providerId: "deepseek", connectionState: "signInRequired"}]} />)
  expect(screen.getByRole("button", {name: "Save DeepSeek API Key"})).toHaveTextContent("Save API Key")

  rerender(<SettingsWindow displayFont="System Default" onDisplayFontChange={() => {}} requestedTab="Services" serviceStatuses={[{providerId: "deepseek", connectionState: "connected"}]} />)
  expect(screen.getByRole("button", {name: "Replace DeepSeek API Key"})).toHaveTextContent("Replace API Key")
})

it("busy service blocks login, duplicate installation and runtime changes", () => {
  render(<SettingsWindow displayFont="System Default" onDisplayFontChange={() => {}} requestedTab="Services" busyServices={["claude", "deepseek"]} verifyingDeepSeekKey serviceStatuses={[{providerId: "claude", connectionState: "notInstalled"}, {providerId: "deepseek", connectionState: "checking"}]} />)
  expect(screen.getByRole("button", {name: "Waiting for Terminal… Claude Code"})).toBeDisabled()
  expect(screen.getByRole("button", {name: "Check Claude Code status"})).toBeDisabled()
  expect(screen.getByRole("button", {name: "Initialize Claude Code quota reading"})).toBeDisabled()
  expect(screen.getAllByRole("combobox")[0]).toBeDisabled()
  expect(screen.getByRole("button", {name: "Verifying DeepSeek API Key"})).toBeDisabled()
  expect(screen.getByRole("button", {name: "Check DeepSeek status"})).toBeDisabled()
})

it("does not describe an ordinary DeepSeek status check as API Key verification", () => {
  render(<SettingsWindow displayFont="System Default" onDisplayFontChange={() => {}} requestedTab="Services" serviceStatuses={[{providerId: "deepseek", connectionState: "checking"}]} />)
  expect(screen.getByRole("button", {name: "Save DeepSeek API Key"})).toBeDisabled()
  expect(screen.queryByRole("button", {name: "Verifying DeepSeek API Key"})).not.toBeInTheDocument()
  expect(screen.getByRole("button", {name: "Check DeepSeek status"})).toBeDisabled()
})

it("connected Claude offers explicit quota initialization with instructions", () => {
  const initialize = vi.fn()
  render(<SettingsWindow displayFont="System Default" onDisplayFontChange={() => {}} requestedTab="Services" onInitializeClaudeUsage={initialize} serviceStatuses={[{providerId: "claude", connectionState: "connected", accountLabel: "member@example.com"}]} />)
  fireEvent.click(screen.getByRole("button", {name: "Initialize Claude Code quota reading"}))
  expect(initialize).toHaveBeenCalledOnce()
  expect(screen.getByText("Opens Claude Code in AI Token Meter’s private empty workspace. Answer any prompt yourself, then choose Check Status.")).toBeVisible()
})
