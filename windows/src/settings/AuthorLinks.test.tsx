import { act, fireEvent, render, screen } from "@testing-library/react"
import { afterEach, describe, expect, it, vi } from "vitest"
import { setLocale } from "../localization"
import { AuthorLinks } from "./AuthorLinks"

afterEach(() => setLocale("en"))

describe("About author links", () => {
  it("renders semantic fixed links without opening a browser", () => {
    const open = vi.fn<() => Promise<void>>()
    render(<AuthorLinks onOpen={open} />)

    expect(screen.getByRole("link", {name: /MillerPanYue/})).toHaveAttribute("href", "https://twitter.com/MillerPanYue")
    expect(screen.getByRole("link", {name: "GitHub"})).toHaveAttribute("href", "https://github.com/sljzdotcom/AI-Token-Meter")
    expect(screen.getByRole("link", {name: /Telegram.*sljzdotcom/})).toHaveAttribute("href", "https://t.me/sljzdotcom")
    expect(open).not.toHaveBeenCalled()
  })

  it.each([
    ["en", "Author links"],
    ["zh-CN", "作者链接"],
  ] as const)("keeps the %s group accessible without a visible heading", (locale, groupName) => {
    act(() => setLocale(locale))
    render(<AuthorLinks onOpen={() => {}} />)

    expect(screen.getByRole("region", {name: groupName})).toBeVisible()
    expect(screen.queryByText(groupName)).not.toBeInTheDocument()
  })

  it("opens only after activation and displays a recoverable failure", async () => {
    const open = vi.fn<(target: "twitter" | "github" | "telegram") => Promise<void>>().mockRejectedValue(new Error("blocked"))
    render(<AuthorLinks onOpen={open} />)

    fireEvent.click(screen.getByRole("link", {name: "GitHub"}))
    await act(async () => { await Promise.resolve() })

    expect(open).toHaveBeenCalledWith("github")
    expect(screen.getByRole("status")).toHaveTextContent("The author link could not be opened.")
  })

  it("localizes the link context and failure feedback in Chinese", async () => {
    act(() => setLocale("zh-CN"))
    render(<AuthorLinks onOpen={() => Promise.reject(new Error("blocked"))} />)

    expect(screen.getByRole("region", {name: "作者链接"})).toBeVisible()
    expect(screen.queryByText("作者链接")).not.toBeInTheDocument()
    fireEvent.click(screen.getByRole("link", {name: /MillerPanYue/}))
    await act(async () => { await Promise.resolve() })
    expect(screen.getByRole("status")).toHaveTextContent("无法打开作者链接。")
  })

  it("keeps feedback recoverable when the browser boundary throws synchronously", async () => {
    render(<AuthorLinks onOpen={() => { throw new Error("blocked") }} />)

    fireEvent.click(screen.getByRole("link", {name: /MillerPanYue/}))
    await act(async () => { await Promise.resolve() })

    expect(screen.getByRole("status")).toHaveTextContent("The author link could not be opened.")
  })
})
