// @ts-nocheck -- This file is injected verbatim into the isolated DeepSeek WebView2 page.
((nonce, emit) => {
  "use strict"
  const amountByRange = new Map()
  const costByRange = new Map()
  const emittedRanges = new Set()
  const officialOrigin = "https://platform.deepseek.com"
  const amountPath = "/api/v0/usage/by_api_key/amount"
  const costPath = "/api/v0/usage/by_api_key/cost"

  const rangeKey = (url) => ["start", "end", "tz"]
    .map((name) => `${name}=${url.searchParams.get(name) ?? ""}`)
    .join("&")

  const dateForBucket = (unixSeconds, timezoneSeconds) => {
    const shifted = (Number(unixSeconds) + Number(timezoneSeconds || 0)) * 1000
    const date = new Date(shifted)
    return Number.isFinite(date.valueOf()) ? date.toISOString().slice(0, 10) : null
  }

  const safeInteger = (value) => {
    const parsed = Number(value)
    return Number.isSafeInteger(parsed) && parsed >= 0 ? parsed : 0
  }

  const safeCost = (value) => {
    const parsed = Number(value)
    return Number.isFinite(parsed) && parsed >= 0 ? parsed : 0
  }

  const parseAmount = (bizData, timezoneSeconds) => {
    const days = new Map()
    for (const series of Array.isArray(bizData?.series) ? bizData.series : []) {
      for (const bucket of Array.isArray(series?.buckets) ? series.buckets : []) {
        const date = dateForBucket(bucket?.time, timezoneSeconds)
        if (!date) continue
        const usage = bucket?.usage ?? {}
        const current = days.get(date) ?? { requests: 0, tokens: 0 }
        current.requests += safeInteger(usage.REQUEST)
        current.tokens += safeInteger(usage.PROMPT_CACHE_HIT_TOKEN)
          + safeInteger(usage.PROMPT_CACHE_MISS_TOKEN)
          + safeInteger(usage.RESPONSE_TOKEN)
        days.set(date, current)
      }
    }
    return days
  }

  const parseCost = (bizData, timezoneSeconds) => {
    const days = new Map()
    const currencies = Array.isArray(bizData?.data) ? bizData.data : []
    const cny = currencies.find((entry) => entry?.currency === "CNY")
    for (const series of Array.isArray(cny?.series) ? cny.series : []) {
      for (const bucket of Array.isArray(series?.buckets) ? series.buckets : []) {
        const date = dateForBucket(bucket?.time, timezoneSeconds)
        if (!date) continue
        days.set(date, (days.get(date) ?? 0) + safeCost(bucket?.cost))
      }
    }
    return days
  }

  const maybeEmit = (key) => {
    if (emittedRanges.has(key) || !amountByRange.has(key) || !costByRange.has(key)) return
    const amount = amountByRange.get(key)
    const cost = costByRange.get(key)
    const dates = [...new Set([...amount.keys(), ...cost.keys()])].sort()
    const days = dates.map((date) => ({
      date,
      costCny: cost.get(date) ?? 0,
      requests: amount.get(date)?.requests ?? 0,
      tokens: amount.get(date)?.tokens ?? 0,
    }))
    emittedRanges.add(key)
    emit(JSON.stringify({ schemaVersion: 1, days }))
  }

  const capture = (value, payload) => {
    let url
    try {
      url = new URL(value, window.location.href)
    } catch {
      return
    }
    if (url.origin !== officialOrigin || ![amountPath, costPath].includes(url.pathname)) return
    if (payload?.code !== 0 || payload?.data?.biz_code !== 0 || !payload?.data?.biz_data) return
    const key = rangeKey(url)
    const timezoneSeconds = safeInteger(Math.abs(Number(url.searchParams.get("tz"))))
      * (Number(url.searchParams.get("tz")) < 0 ? -1 : 1)
    if (url.pathname === amountPath) {
      amountByRange.set(key, parseAmount(payload.data.biz_data, timezoneSeconds))
    } else {
      costByRange.set(key, parseCost(payload.data.biz_data, timezoneSeconds))
    }
    maybeEmit(key)
  }

  if (/^[a-f0-9]{32}$/.test(nonce) && window.location.origin === officialOrigin) {
    const originalFetch = window.fetch.bind(window)
    window.fetch = async (...arguments_) => {
      const response = await originalFetch(...arguments_)
      try {
        const request = arguments_[0]
        const url = typeof request === "string" ? request : request?.url
        capture(url, await response.clone().json())
      } catch {
        // Official page behavior must never be changed by optional history capture.
      }
      return response
    }

    const originalOpen = XMLHttpRequest.prototype.open
    XMLHttpRequest.prototype.open = function (method, url, ...rest) {
      this.__aimeterHistoryURL = String(url)
      this.addEventListener("load", () => {
        try {
          capture(this.__aimeterHistoryURL, JSON.parse(this.responseText))
        } catch {
          // Ignore non-JSON and future response shapes without exposing raw data.
        }
      }, { once: true })
      return originalOpen.call(this, method, url, ...rest)
    }
  }

  return { capture }
})
