# Codex Detail and DeepSeek Sync Design

Date: 2026-08-31
Status: approved for implementation

## Goals

1. Make the Codex detail panel quota-first and add three clearly labelled local activity summaries.
2. Make DeepSeek's signed-in usage page automatically populate the native 30-day cards and chart.
3. Keep documentation, privacy boundaries, test counts, and release history aligned with the shipped behavior.

## Codex detail panel

The panel follows approved layout A: official account data remains visually primary, followed by a separate `Last 30 days · This Mac` section.

Official data:

- current and weekly quota windows, percentages, and reset times from `codex app-server`;
- available reset-credit count and each known expiry date.

Local data:

- Token: sum of `tokens_used` for local Codex threads active in the rolling 30-day window;
- Current streak: consecutive local calendar days with Codex activity, ending today or yesterday;
- Longest session: largest non-negative `updated_at - created_at` duration among qualifying local threads.

The local section reads only aggregate columns from Codex's local SQLite state. It never reads prompts, responses, titles, previews, or credentials. Missing or incompatible local state must not make official quota collection fail.

## DeepSeek automatic usage sync

The embedded official page remains the authentication boundary. AI Meter does not request or persist the platform login token. Its page bridge observes only HTTPS JSON responses from `platform.deepseek.com`, then extracts aggregate daily fields.

The current platform returns two separate series:

- `/api/v0/usage/by_api_key/amount`: daily request and token buckets;
- `/api/v0/usage/by_api_key/cost`: daily CNY cost buckets.

AI Meter will parse both schemas, ignore API-key identity fields, merge series by local calendar day, normalize exactly 30 days, and persist only daily cost/request/token totals plus the update time. A partial response remains in memory until both facets arrive; it must not overwrite a complete cached history.

## Failure behavior

- Codex official quota remains usable when local aggregation is unavailable.
- DeepSeek keeps the last complete cache if either usage facet is missing or malformed.
- Undocumented DeepSeek endpoints are treated as change-prone; refresh failures show a stale state without exposing response bodies.

## UI and accessibility

- Codex detail panel expands to fit the official cards, reset credits, three local statistics, source note, and updated time.
- The local statistics use concise labels and accessibility text that identifies them as this-Mac estimates.
- DeepSeek continues to show the embedded official sign-in page only until a complete native history exists.

## Verification

- Unit tests cover Codex aggregate parsing, date-window/streak/duration behavior, fallback behavior, both DeepSeek response schemas, facet merging, and privacy-safe persistence.
- The full Swift test suite, release build, installed-app smoke check, and visual detail-panel check must pass before merge.
