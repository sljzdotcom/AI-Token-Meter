# Task 4 Report: Rolling stable appcast contract

## Scope

- Changed only `Tests/AIMeterAppTests/SoftwareUpdatePackagingTests.swift`.
- Did not modify `appcast.xml`, production code, release assets, signing material, or macOS application behavior.
- Replaced the stale `0.2.2`/build `6` assertions with validation of every published item: non-empty numeric dotted version, positive integer build, exact official GitHub archive URL corresponding to that version, 64-byte base64 EdDSA signature syntax, positive archive length, and macOS `14.0` minimum.
- Added isolated corrupt fixtures for URL/version mismatch, invalid build, invalid signature encoding, zero length, and wrong minimum OS.

## RED evidence

1. Baseline evidence supplied with the task: `bash scripts/test.sh --filter SoftwareUpdatePackagingTests` failed three `stableAppcastContract` assertions because the rolling feed now starts at `0.5.0` instead of the hard-coded `0.2.2`/build `6`/old URL. Full prior log: `/private/tmp/win-cli-swift-validation.log`; native CI: `34183253412`.
2. After adding the new behavior test before its helper, `bash scripts/test.sh --filter SoftwareUpdatePackagingTests` failed compilation with `cannot find 'validateStableAppcast' in scope` at both the real-feed and corrupt-fixture assertions.
3. Mutation check: temporarily replaced strict validation with naive field-presence checks, then ran `bash scripts/test.sh --filter corruptStableAppcastIsRejected`. It failed all 5 fixture cases with 5 issues, proving field-presence behavior accepts corrupted metadata. The mutation was removed immediately.

## GREEN evidence

- `bash scripts/test.sh --filter SoftwareUpdatePackagingTests` — exit 0; 8 tests in 1 suite passed, including 5 corrupt fixture cases; contract, portability, release-feed, docs, and public-release gates also passed.
- `bash scripts/test.sh` — exit 0; 420 tests in 81 suites plus 13 PTY tests passed; all trailing contract, portability, Windows asset normalization, release-feed, documentation, and public-release gates passed.
- `git diff --check` — exit 0.

## Limits

- Signature validation here checks required non-empty canonical base64 shape and the 64-byte EdDSA signature size; it does not claim cryptographic authenticity. The existing release archive/public release gates remain responsible for verification against the public key.
- No remote assets were downloaded and no private signing keys were accessed.
