import { render, screen } from "@testing-library/react"
import { describe, expect, it } from "vitest"

import { App } from "./App"

describe("Windows meter shell", () => {
  it("renders the three providers with stable accessible names", () => {
    render(<App />)

    expect(screen.getByLabelText("Claude Code usage")).toBeVisible()
    expect(screen.getByLabelText("OpenAI Codex usage")).toBeVisible()
    expect(screen.getByLabelText("DeepSeek usage")).toBeVisible()
  })

  it("does not invent a numeric quota before collectors report data", () => {
    render(<App />)

    expect(screen.queryByText(/\d+%/)).not.toBeInTheDocument()
    expect(screen.getAllByText("Unavailable")).toHaveLength(3)
  })
})
