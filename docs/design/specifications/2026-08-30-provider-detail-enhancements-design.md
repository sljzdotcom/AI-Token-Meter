# AI Meter Provider Detail Enhancements Design

## Goal

Extend AI Meter's provider details without weakening the accuracy and privacy guarantees already established:

1. Show Codex earned rate-limit reset credits as read-only information, including the available count and each available credit's expiration date.
2. Show DeepSeek's official recent-30-day API usage in a native analytics panel after one official website sign-in.
3. Reduce the floating strip to larger provider logos only while preserving meaningful progress arcs.
4. Represent DeepSeek balance depletion against a configurable CNY baseline, defaulting to ¥100.

## Confirmed User Experience

### Floating Strip

All three circles show only a larger provider logo. Visible provider names, percentages, and balance text are removed from the circle centers. Accessibility labels continue to announce provider name, current value, availability, and whether the detail panel is open.

- Claude and Codex retain progress arcs based on their official percentage limits.
- DeepSeek uses a balance-depletion arc. With the default ¥100 baseline and a ¥77.99 balance, the arc displays approximately 22% used.
- DeepSeek's baseline is configurable in settings and defaults to ¥100.
- The balance-depletion fraction is `1 - currentBalance / configuredBaseline`, clamped to 0...1. A balance at or above the baseline shows 0%; a zero balance shows 100%.

### Codex Detail

The Codex detail card retains the existing general rate-limit view and adds a read-only reset-credit section:

- `Available reset credits: N`
- One row per available credit whose details are supplied by the backend.
- Each row shows the backend display title when present and the expiration date.
- A credit with no expiration is labelled `No expiration provided`.
- If the backend supplies only `availableCount`, the card shows the count and `Expiration details unavailable`.
- AI Meter never exposes a redeem button, never invokes `account/rateLimitResetCredit/consume`, and does not persist the opaque credit ID.

### DeepSeek Detail

Clicking the DeepSeek logo opens a large native analytics card to the left of the floating strip. It follows the approved concept layout:

- Header with DeepSeek logo, `Last 30 days API usage`, close button, and refresh action.
- Summary cards for cost, request count, and token count.
- Native daily cost bar chart over the latest 30 calendar days.
- Current balance, configured baseline, data source, and last successful update.
- `View on DeepSeek` fallback action.

The large analytics card pauses automatic dismissal while the pointer is inside it. Leaving the card restarts the configured detail timeout. Clicking outside either panel or using the close button dismisses it immediately. An active official sign-in flow is never auto-dismissed.

## Architecture

### Provider-Specific Supplemental Data

Keep `UsageMetric` focused on numeric usage and balance values. Add optional provider-specific supplements to `UsageSnapshot` rather than overloading primary or secondary metrics:

- `CodexResetCreditsSummary`
  - `availableCount: Int`
  - `credits: [CodexResetCreditDisplay]`
  - each display row contains only title and optional expiration date
  - `hasCompleteDetails: Bool`
- `DeepSeekUsageHistory`
  - normalized latest-30-day daily records
  - aggregate cost, requests, and tokens
  - source update time and synchronization state

The new fields are optional with backward-compatible Codable decoding so existing cache files remain readable.

### Codex Collection

Continue using the official `account/rateLimits/read` app-server method. Decode its optional `rateLimitResetCredits` object alongside the existing top-level general rate limit.

Collection rules:

1. Preserve `availableCount` even when detail rows are absent or capped.
2. Retain only rows whose status is `available`.
3. Map title and `expiresAt` into display-only values.
4. Discard credit IDs, grant timestamps, descriptions not used by the UI, redemption states, and unknown reset types.
5. Treat a missing summary as “feature not supplied”, not as zero credits.

### DeepSeek Official Web Synchronization

The public DeepSeek API documents current balance but not historical account usage. Exact automatic history therefore uses an authenticated official-web session with a controlled fallback:

1. A dedicated persistent `WKWebView` opens the official DeepSeek Usage URL.
2. The user completes the first sign-in directly on the official page. AI Meter does not observe form values.
3. Page-origin synchronization code reads the same usage data used by the official dashboard and returns only normalized daily aggregates through a narrow message bridge.
4. AI Meter never exports cookies, authorization headers, passwords, or raw responses from WebKit storage.
5. If the page contract changes, synchronization fails closed: show the last successful cache, mark it stale, and keep `View on DeepSeek` available.

The first implementation may support the current official dashboard contract and is explicitly maintainable rather than pretending the undocumented contract is permanent.

### DeepSeek History Cache

Store normalized, non-credential history under AI Meter's Application Support directory. On detail open:

1. Render cached history immediately.
2. If no cache exists or the cache is older than 30 minutes, synchronize in the background.
3. A manual refresh bypasses the freshness interval.
4. Normalize all points to local calendar days and exactly the latest 30 dates, inserting zero-valued missing days.
5. A failed refresh retains the last successful history and surfaces a short sanitized status.

## UI Components

- `ProviderLogo`: renders bundled local brand assets consistently inside all circles and detail headers.
- `UsageRing`: renders the provider logo in the center and accepts the same presentation fraction for its arc; it no longer renders center text.
- `CodexResetCreditsView`: read-only count and expiration rows.
- `DeepSeekAnalyticsView`: summary cards, native Swift Charts bar chart, balance/baseline footer, refresh and official-page actions.
- `DeepSeekWebSession`: owns the persistent WebKit session and narrowly scoped synchronization bridge.

Brand assets are bundled with the application and never downloaded at runtime.

## Error Handling and Privacy

- A missing Codex credits summary hides the section; an explicit zero shows `0`.
- Incomplete Codex credit details display an explanatory fallback instead of inventing dates.
- DeepSeek sign-in expiry shows `Sign in to refresh usage`; balance collection through the API key continues independently.
- DeepSeek website contract failures do not clear cached history or make other providers unavailable.
- Logs contain only operation names, sanitized error categories, record counts, and timestamps.
- No raw DeepSeek dashboard payload, cookie, password, authorization header, API key, Codex login token, or reset-credit ID is cached or logged.

## Testing

Use test-driven development for every behavior:

1. Codex app-server fixture with two available credits, distinct expirations, and an unavailable row; assert count, filtering, dates, and absence of IDs from persisted models.
2. Codex fixture with count-only details; assert the incomplete-details state.
3. Backward-compatible snapshot-cache decoding without supplemental fields.
4. DeepSeek daily-history normalization, 30-day clipping, missing-day zero fill, and aggregate calculation.
5. DeepSeek cache freshness, stale fallback, and sanitized synchronization failures.
6. DeepSeek balance-depletion fraction at ¥77.99/¥100, above baseline, zero balance, and invalid baseline.
7. Presentation tests proving all circles render logos only while accessibility still announces values.
8. Detail-session tests proving the large card pauses auto-hide during pointer interaction and sign-in.
9. UI/build verification for Codex credit rows, DeepSeek analytics layout, bundled logos, and Swift Charts rendering.

## Acceptance Criteria

- The floating strip shows three larger logos with no visible center text.
- Claude and Codex arcs remain accurate; DeepSeek shows approximately 22% used for ¥77.99 against ¥100.
- The DeepSeek baseline is editable in settings and survives relaunch.
- Codex displays the available reset-credit count and each supplied expiration date, without any redemption control.
- DeepSeek displays official recent-30-day cost, requests, tokens, and daily cost after one official sign-in.
- Cached DeepSeek analytics remain available during network or website-contract failures with a clear stale status.
- Existing cache files migrate without data loss.
- Full automated tests, release build, signing checks, and installed-app visual acceptance pass before the branch is merged into `main`.
