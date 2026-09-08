# Cross-platform contracts

- [Snapshot JSON Schema](schemas/usage-snapshot.schema.json): version 1, with optional Gemini visible-tier metrics.
- [Provider presentation](presentation/providers.json): shared identity, order, colors and semantics.
- [Feature parity](parity/features.yml): platform delivery evidence and limits.
- Snapshot fixtures: [Gemini unavailable](fixtures/gemini-unavailable.json), [Gemini fresh](fixtures/gemini-fresh.json), plus the existing Claude, Codex, DeepSeek and authentication examples under `fixtures/`.
- [Gemini CLI 0.58.0 terminal transcripts](gemini-cli/0.58.0/README.md): synthetic official-CLI capture provenance, raw redraws, ready/dialog prefixes and SHA-256 manifest.

Validate with `ruby scripts/check-cross-platform-contracts.rb .`; run `bash scripts/test-cross-platform-contracts.sh` to verify rejection of malformed contract inputs.
