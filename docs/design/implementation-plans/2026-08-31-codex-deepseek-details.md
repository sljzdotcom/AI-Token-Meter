# Codex Detail and DeepSeek Sync Implementation Plan

Date: 2026-08-31  
Branch: `codex/codex-deepseek-details`

## Checkpoint 1 — contracts and tests

- Add a privacy-safe Codex local activity domain model.
- Add failing tests for SQLite aggregate output parsing and rolling-window calculations.
- Add failing tests for the current DeepSeek amount/cost bucket schemas and two-facet merging.

## Checkpoint 2 — data collection

- Query only aggregate columns from the local Codex state database with a bounded, read-only operation.
- Attach optional local activity to the official Codex snapshot without weakening official quota failure handling.
- Parse and merge DeepSeek amount/cost response facets, then save only complete normalized history.

## Checkpoint 3 — interface

- Implement the approved quota-first Codex detail layout.
- Add the three local activity cards and local-source disclosure.
- Adjust panel sizing and demo data for visual verification.

## Checkpoint 4 — documentation and release verification

- Update README, provider/user documentation, architecture, privacy, testing, development status, change log, and commit history.
- Run the complete test suite and release build.
- Install and launch the app, verify Codex and DeepSeek details, and confirm the DeepSeek cache is created from the official page.
- Commit each checkpoint and merge the verified branch into `main`.
