import { expect, it, vi } from "vitest"

import bridgeSource from "./bridge.ts?raw"

type Bridge = {
  capture: (url: string, payload: unknown) => void
  waitUntilUsablePage: (timeoutMs: number, pollIntervalMs: number) => Promise<boolean>
}

it("emits only daily aggregates after matching official amount and cost responses", () => {
  const emitted: string[] = []
  const factory = (0, eval)(bridgeSource) as (nonce: string, emit: (payload: string) => void) => Bridge
  const bridge = factory("0123456789abcdef0123456789abcdef", (payload) => emitted.push(payload))
  const query = "start=1788393600&end=1788566400&tz=0"

  bridge.capture(`https://platform.deepseek.com/api/v0/usage/by_api_key/amount?${query}`, {
    code: 0,
    data: { biz_code: 0, biz_data: { series: [
      {
        api_key: { tracking_id: "must-not-leak", name: "private-key-name", sensitive_id: "private" },
        model: "deepseek-v4-flash",
        buckets: [{ time: 1788393600, usage: {
          PROMPT_CACHE_HIT_TOKEN: "100",
          PROMPT_CACHE_MISS_TOKEN: "20",
          RESPONSE_TOKEN: "30",
          REQUEST: "4",
        } }],
      },
    ] } },
  })
  expect(emitted).toEqual([])

  bridge.capture(`https://platform.deepseek.com/api/v0/usage/by_api_key/cost?${query}`, {
    code: 0,
    data: { biz_code: 0, biz_data: { data: [{ currency: "CNY", series: [
      {
        api_key: { tracking_id: "must-not-leak", name: "private-key-name", sensitive_id: "private" },
        model: "deepseek-v4-flash",
        buckets: [{ time: 1788393600, cost: "1.25" }],
      },
    ] }] } },
  })

  expect(emitted).toHaveLength(1)
  expect(JSON.parse(emitted[0])).toEqual({
    schemaVersion: 1,
    days: [{ date: "2026-09-03", costCny: 1.25, requests: 4, tokens: 150 }],
  })
  expect(emitted[0]).not.toContain("must-not-leak")
  expect(emitted[0]).not.toContain("private-key-name")
})

it("ignores lookalike origins, unrelated endpoints, and application errors", () => {
  const emitted: string[] = []
  const factory = (0, eval)(bridgeSource) as (nonce: string, emit: (payload: string) => void) => Bridge
  const bridge = factory("0123456789abcdef0123456789abcdef", (payload) => emitted.push(payload))
  const payload = { code: 0, data: { biz_code: 0, biz_data: { series: [] } } }

  bridge.capture("https://platform.deepseek.com.evil.example/api/v0/usage/by_api_key/amount", payload)
  bridge.capture("https://platform.deepseek.com/api/v0/users/get_user_summary", payload)
  bridge.capture("https://platform.deepseek.com/api/v0/usage/by_api_key/amount", {
    code: 0,
    data: { biz_code: 40003, biz_data: null },
  })

  expect(emitted).toEqual([])
})

it("does not treat an empty SPA shell as a usable official page", async () => {
  vi.useFakeTimers()
  document.body.innerHTML = '<div id="root"></div>'
  const factory = (0, eval)(bridgeSource) as (nonce: string, emit: (payload: string) => void) => Bridge
  const bridge = factory("0123456789abcdef0123456789abcdef", () => {})

  const usable = bridge.waitUntilUsablePage(30, 10)
  await vi.advanceTimersByTimeAsync(31)

  await expect(usable).resolves.toBe(false)
  vi.useRealTimers()
})

it("does not treat a rendered same-origin error surface as a usable account page", async () => {
  vi.useFakeTimers()
  document.body.innerHTML = '<div id="root"><section role="alert"><h1>Unavailable</h1></section></div>'
  const factory = (0, eval)(bridgeSource) as (nonce: string, emit: (payload: string) => void) => Bridge
  const bridge = factory("0123456789abcdef0123456789abcdef", () => {})

  const usable = bridge.waitUntilUsablePage(30, 10)
  await vi.advanceTimersByTimeAsync(31)

  await expect(usable).resolves.toBe(false)
  vi.useRealTimers()
})

it("does not accept a navigation-shaped error shell with retry controls", async () => {
  vi.useFakeTimers()
  document.body.innerHTML = `
    <div id="root">
      <nav><a href="/">Home</a></nav>
      <main role="alert"><button type="button">Retry</button></main>
    </div>`
  const factory = (0, eval)(bridgeSource) as (nonce: string, emit: (payload: string) => void) => Bridge
  const bridge = factory("0123456789abcdef0123456789abcdef", () => {})
  bridge.capture("https://platform.deepseek.com/api/v0/usage/by_api_key/amount?start=1&end=2&tz=0", {
    code: 0,
    data: { biz_code: 0, biz_data: { series: [] } },
  })

  const usable = bridge.waitUntilUsablePage(30, 10)
  await vi.advanceTimersByTimeAsync(31)

  await expect(usable).resolves.toBe(false)
  vi.useRealTimers()
})

it("allows a rendered usage application to contain a non-blocking alert", async () => {
  document.body.innerHTML = `
    <div id="root">
      <nav><a href="/usage"></a></nav>
      <main><button type="button"></button></main>
      <aside role="alert"></aside>
    </div>`
  const factory = (0, eval)(bridgeSource) as (nonce: string, emit: (payload: string) => void) => Bridge
  const bridge = factory("0123456789abcdef0123456789abcdef", () => {})
  bridge.capture("https://platform.deepseek.com/api/v0/usage/by_api_key/amount?start=1&end=2&tz=0", {
    code: 0,
    data: { biz_code: 0, biz_data: { series: [] } },
  })

  await expect(bridge.waitUntilUsablePage(30, 10)).resolves.toBe(true)
})

it("does not accept an empty application root after only an official usage response", async () => {
  vi.useFakeTimers()
  document.body.innerHTML = '<div id="root"></div>'
  const factory = (0, eval)(bridgeSource) as (nonce: string, emit: (payload: string) => void) => Bridge
  const bridge = factory("0123456789abcdef0123456789abcdef", () => {})
  bridge.capture("https://platform.deepseek.com/api/v0/usage/by_api_key/amount?start=1&end=2&tz=0", {
    code: 0,
    data: { biz_code: 0, biz_data: { series: [] } },
  })

  const usable = bridge.waitUntilUsablePage(30, 10)
  await vi.advanceTimersByTimeAsync(31)

  await expect(usable).resolves.toBe(false)
  vi.useRealTimers()
})

it("recognizes an initialized application after an exact official usage response", async () => {
  vi.useFakeTimers()
  document.body.innerHTML = `
    <div id="root">
      <nav><a href="/usage"></a></nav>
      <main><button type="button"></button></main>
    </div>`
  const factory = (0, eval)(bridgeSource) as (nonce: string, emit: (payload: string) => void) => Bridge
  const bridge = factory("0123456789abcdef0123456789abcdef", () => {})

  const usable = bridge.waitUntilUsablePage(100, 10)
  bridge.capture("https://platform.deepseek.com/api/v0/usage/by_api_key/amount?start=1&end=2&tz=0", {
    code: 0,
    data: { biz_code: 0, biz_data: { series: [] } },
  })
  await vi.advanceTimersByTimeAsync(11)

  await expect(usable).resolves.toBe(true)
  vi.useRealTimers()
})

it("recognizes a rendered login form without reading its field values", async () => {
  document.body.innerHTML = '<div id="root"><form><input type="email"><button type="submit"></button></form></div>'
  const factory = (0, eval)(bridgeSource) as (nonce: string, emit: (payload: string) => void) => Bridge
  const bridge = factory("0123456789abcdef0123456789abcdef", () => {})

  await expect(bridge.waitUntilUsablePage(30, 10)).resolves.toBe(true)
})
