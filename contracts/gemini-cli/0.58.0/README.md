# Gemini CLI 0.58.0 synthetic terminal fixtures

Captured by `scripts/test-gemini-cli-startup.mjs` from the unchanged official npm CLI 0.58.0, in Task 4a's isolated PTY and synthetic OAuth/network environment. Source protocol and package integrity: `docs/development/2026-09-08-gemini-cli-capability.md`.

The four full transcripts preserve actual ANSI redraws and terminal output. OAuth authorization URL query parameters are replaced by `SYNTHETIC_QUERY_REDACTED`; no real credentials or identities are present. `*-model-open.ansi.txt` are exact prefixes ending at the last complete model dialog's bottom border, before Escape/quit removes it. `ready.ansi.txt` is an exact prefix ending after the first ready input frame, before its next redraw. Full transcripts end after exit and must not be interpreted as still-visible quota.

Authenticated model rows: Pro 25% used, Flash 60% used; `Resets: 5:47 PM (1h)` is CLI text, not an absolute timestamp. The footer's 43% is not a model tier. The quota-failure dialog has no Model usage section. Missing/invalid authentication requires cancellation before any `/model` input. These are synthetic account fixtures, not real account or native Windows acceptance evidence.

The authenticated-untrusted transcripts come from the independently reviewed eighth scenario (d11b572): folderTrust remains enabled and GEMINI_CLI_TRUST_WORKSPACE=false explicitly keeps the workspace untrusted, with no trust-rule writes. `untrusted-ready` and `authenticated-untrusted-model-open` use the same prefix extraction rule.

Git attributes preserve ANSI transcript bytes without line-ending conversion or whitespace cleanup. Use the manifest SHA-256 and direct binary/text reads to inspect these protocol fixtures.
