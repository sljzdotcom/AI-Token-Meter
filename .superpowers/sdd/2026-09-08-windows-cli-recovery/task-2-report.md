# Task 2 Report: Unified Windows CLI discovery failures

## Outcome

- Added `accounts::runtime_discovery::discover_runtime_cli`, the single bounded runtime executor used by account status, quota collection, login launch, and custom-path validation.
- Preserved the existing eight-second / twelve-process discovery budget, three- or four-second per-process limits, bounded output, restricted `CommandInvocation`, native/npm/WSL priority, explicit custom path, and explicit WSL distribution rules.
- Extended the policy result with `Cancelled` and made WSL enumeration return an explicit `Missing` / `Unavailable` / `Cancelled` outcome.
- Quota collection now maps only `CliDiscovery::Missing` to `UsageStatus::NotInstalled`; `Unavailable` maps to retryable `CollectionError::Transport`, while `Cancelled` stays `CollectionError::Cancelled`.
- Account status maps confirmed absence to `NotInstalled` and discovery failure/cancellation to `Unavailable`. Login distinguishes missing from temporarily unavailable and uses the same verified discovery path.

## TDD evidence

RED 1:

`cargo test --test cli_discovery --test runtime_cli_discovery`

Failed because `CliProbe::Cancelled`, `CliDiscovery::Cancelled`, and `accounts::runtime_discovery` did not exist.

RED 2:

`cargo test --test runtime_cli_discovery`

Failed because production collection mapping `candidate_for_collection` did not exist.

RED 3:

`cargo test --test cli_discovery cancelled_wsl_listing_is_not_folded_into_unavailable`

Failed because WSL enumeration had no cancellation-bearing result.

RED 4:

`cargo test accounts::runtime_discovery::tests`

Failed because the real bounded executor did not yet accept controlled discovery inputs for its internal production-path regressions.

GREEN:

- `cargo test accounts::runtime_discovery::tests`: 2 passed; a real existing candidate that cannot start is `Unavailable`, while a complete empty native search is `Missing`.
- `cargo test --test cli_discovery --test runtime_cli_discovery`: 9 passed; includes candidate cancellation, WSL-list cancellation, collection mapping, explicit path, WSL selection/fallback, and unavailable-vs-missing coverage.
- `cargo test` outside the filesystem/network sandbox: all Rust unit, integration, and doc tests passed. The first sandboxed full run failed only because six existing DeepSeek tests could not bind their loopback server (`Operation not permitted`); the permitted rerun passed.

## Limitations

- The macOS host cannot run Windows-only account/application modules or native Windows processes. `cargo check --target x86_64-pc-windows-msvc --tests` was attempted but the host lacks the Windows C/MSVC headers required by `ring` (`assert.h` not found). Native Windows CI remains required.
- No CLI login, installation, trust response, credential read, or provider network action was performed.
- Claude isolated workspace and UI initialization belong to Task 3 and were intentionally not changed here.
