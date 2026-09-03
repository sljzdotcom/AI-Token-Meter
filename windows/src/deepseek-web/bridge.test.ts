import { expect, it } from "vitest"

import bridgeSource from "./bridge.ts?raw"

type Bridge = {
  capture: (url: string, payload: unknown) => void
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
