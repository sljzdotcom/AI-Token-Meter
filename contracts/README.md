# Cross-platform contracts

- [Snapshot JSON Schema](schemas/usage-snapshot.schema.json): version 1, with two Gemini quota metrics and optional bounded Antigravity CLI model information.
- [Provider presentation](presentation/providers.json): shared identity, order, colors and semantics.
- [Feature parity](parity/features.yml): platform delivery evidence and limits.
- Snapshot fixtures: [Gemini unavailable](fixtures/gemini-unavailable.json), [Gemini fresh](fixtures/gemini-fresh.json), plus the existing Claude, Codex, DeepSeek and authentication examples under `fixtures/`.
- [Antigravity CLI 1.1.28 usage contract](antigravity-cli/1.1.28/README.md): synthetic, account-free upstream `/usage` output; production validates the full shape and publishes only the two Gemini windows.
- [Legacy Gemini CLI 0.58.0 terminal transcripts](gemini-cli/0.58.0/README.md): historical 0.6.0 release evidence; no longer used by production collection.

Validate with `ruby scripts/check-cross-platform-contracts.rb .`; run `bash scripts/test-cross-platform-contracts.sh` to verify rejection of malformed contract inputs.
