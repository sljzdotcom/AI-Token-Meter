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
  expect(screen.getByLabelText("Move Gemini down")).toBeDisabled()
  expect(screen.getByLabelText("Move DeepSeek down")).toBeEnabled()
  fireEvent.click(screen.getByLabelText("Move Gemini up"))
  expect(screen.getAllByRole("checkbox").map(input => input.parentElement?.textContent)).toEqual(["Claude Code", "OpenAI Codex", "Gemini", "DeepSeek"])
  fireEvent.click(screen.getByLabelText("Claude Code"))
  fireEvent.click(screen.getByLabelText("OpenAI Codex"))
  expect(screen.getByLabelText("DeepSeek")).toBeEnabled()
  fireEvent.click(screen.getByLabelText("DeepSeek"))
  expect(screen.getByLabelText("Gemini")).toBeChecked()
  expect(screen.getByLabelText("Gemini")).toBeDisabled()
  fireEvent.click(screen.getByLabelText("Claude Code"))
  expect(screen.getByLabelText("Gemini")).toBeEnabled()
  fireEvent.click(screen.getByRole("button", {name:"Restore default order"}))
  expect(screen.getAllByRole("checkbox").every(input => (input as HTMLInputElement).checked)).toBe(true)
  expect(screen.getByLabelText("Move Gemini down")).toBeDisabled()
})

it("unknown Gemini capability shows unavailable without implying missing installation or authentication", () => {
  render(<SettingsWindow displayFont="System Default" onDisplayFontChange={() => {}} requestedTab="Services" />)
  expect(screen.getByText("Gemini CLI")).toBeVisible()
  expect(screen.getByRole("button", {name:"Check Gemini status"})).toBeEnabled()
  expect(screen.getByRole("button", {name:"Gemini CLI 0.58.0 installation guide"})).toBeEnabled()
  expect(screen.getByText("Requires Node.js 20 or later.")).toBeVisible()
  expect(screen.getByText("npm install -g @google/gemini-cli@0.58.0")).toBeVisible()
  expect(screen.getByText("Run gemini and choose Sign in with Google.")).toBeVisible()
  expect(screen.getByText("Return to AI Token Meter and choose Check Status.")).toBeVisible()
  expect(screen.queryByRole("button", {name:/Install.*Gemini|Sign in.*Gemini/})).not.toBeInTheDocument()
})

it("installed Gemini guidance skips reinstall and directs sign-in or status checking", () => {
  const {rerender} = render(<SettingsWindow displayFont="System Default" onDisplayFontChange={() => {}} requestedTab="Services" serviceStatuses={[{providerId: "gemini", connectionState: "signInRequired"}]} />)
  expect(screen.queryByText("npm install -g @google/gemini-cli@0.58.0")).not.toBeInTheDocument()
  expect(screen.getByText("Run gemini and choose Sign in with Google.")).toBeVisible()
  expect(screen.getByText("Return to AI Token Meter and choose Check Status.")).toBeVisible()

  rerender(<SettingsWindow displayFont="System Default" onDisplayFontChange={() => {}} requestedTab="Services" serviceStatuses={[{providerId: "gemini", connectionState: "connected"}]} />)
  expect(screen.queryByText("Run gemini and choose Sign in with Google.")).not.toBeInTheDocument()
  expect(screen.getByRole("button", {name:"Gemini CLI 0.58.0 installation guide"})).toBeEnabled()
  expect(screen.getByRole("button", {name:"Check Gemini status"})).toBeEnabled()
})
