import { useState } from "react"
import { fireEvent, render, screen } from "@testing-library/react"
import { expect, it } from "vitest"
import { SettingsWindow } from "./SettingsWindow"
import { defaultStripPreferences } from "../state/stripPreferences"

function StatefulSettings() {
  const [preferences, setPreferences] = useState(defaultStripPreferences)
  return <SettingsWindow displayFont="System Default" onDisplayFontChange={() => {}} stripPreferences={preferences} onStripPreferencesChange={setPreferences} />
}

it("fourth provider is named and can move while only the last visible provider is protected", () => {
  render(<StatefulSettings />)
  expect(screen.getAllByRole("checkbox")).toHaveLength(4)
  expect(screen.getByLabelText("Move Google Antigravity down")).toBeDisabled()
  expect(screen.getByLabelText("Move DeepSeek down")).toBeEnabled()
  fireEvent.click(screen.getByLabelText("Move Google Antigravity up"))
  expect(screen.getAllByRole("checkbox").map(input => input.parentElement?.textContent)).toEqual(["Claude Code", "OpenAI Codex", "Google Antigravity", "DeepSeek"])
  fireEvent.click(screen.getByLabelText("Claude Code"))
  fireEvent.click(screen.getByLabelText("OpenAI Codex"))
  expect(screen.getByLabelText("DeepSeek")).toBeEnabled()
  fireEvent.click(screen.getByLabelText("DeepSeek"))
  expect(screen.getByLabelText("Google Antigravity")).toBeChecked()
  expect(screen.getByLabelText("Google Antigravity")).toBeDisabled()
  fireEvent.click(screen.getByLabelText("Claude Code"))
  expect(screen.getByLabelText("Google Antigravity")).toBeEnabled()
  fireEvent.click(screen.getByRole("button", {name:"Restore default order"}))
  expect(screen.getAllByRole("checkbox").every(input => (input as HTMLInputElement).checked)).toBe(true)
  expect(screen.getByLabelText("Move Google Antigravity down")).toBeDisabled()
})

it("edits independent bounded show and hide delays", () => {
  render(<StatefulSettings />)
  const reveal = screen.getByLabelText("Show delay (ms)") as HTMLInputElement
  const collapse = screen.getByLabelText("Hide delay (ms)") as HTMLInputElement
  expect(reveal.value).toBe("150")
  expect(collapse.value).toBe("800")
  fireEvent.change(reveal, {target: {value: "2500"}})
  fireEvent.change(collapse, {target: {value: "-1"}})
  expect(reveal.value).toBe("2000")
  expect(collapse.value).toBe("0")
})

it("unknown Gemini capability shows unavailable without implying missing installation or authentication", () => {
  render(<SettingsWindow displayFont="System Default" onDisplayFontChange={() => {}} requestedTab="Services" />)
  expect(screen.getByText("Google Antigravity")).toBeVisible()
  expect(screen.getByRole("button", {name:"Check Antigravity status"})).toBeEnabled()
  expect(screen.getByRole("button", {name:"Antigravity CLI installation guide"})).toBeEnabled()
  expect(screen.getByText("irm https://antigravity.google/cli/install.ps1 | iex")).toBeVisible()
  expect(screen.getByText("Run agy and complete Google sign-in.")).toBeVisible()
  expect(screen.getByText("Return to AI Token Meter and choose Check Status.")).toBeVisible()
  expect(screen.queryByRole("button", {name:/Install.*Gemini|Sign in.*Gemini/})).not.toBeInTheDocument()
})

it("installed Gemini guidance skips reinstall and directs sign-in or status checking", () => {
  const {rerender} = render(<SettingsWindow displayFont="System Default" onDisplayFontChange={() => {}} requestedTab="Services" serviceStatuses={[{providerId: "gemini", connectionState: "signInRequired"}]} />)
  expect(screen.queryByText("irm https://antigravity.google/cli/install.ps1 | iex")).not.toBeInTheDocument()
  expect(screen.getByText("Run agy and complete Google sign-in.")).toBeVisible()
  expect(screen.getByText("Return to AI Token Meter and choose Check Status.")).toBeVisible()

  rerender(<SettingsWindow displayFont="System Default" onDisplayFontChange={() => {}} requestedTab="Services" serviceStatuses={[{providerId: "gemini", connectionState: "connected"}]} />)
  expect(screen.queryByText("Run agy and complete Google sign-in.")).not.toBeInTheDocument()
  expect(screen.getByRole("button", {name:"Antigravity CLI installation guide"})).toBeEnabled()
  expect(screen.getByRole("button", {name:"Check Antigravity status"})).toBeEnabled()
})
