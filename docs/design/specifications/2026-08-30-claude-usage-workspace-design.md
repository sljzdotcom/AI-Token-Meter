# Claude Usage Workspace Design

## Context

AI Meter reads Claude subscription usage by starting the installed Claude Code CLI in a PTY, checking `claude auth status`, and sending the built-in `/usage` command. The installed application currently inherits `/` as its working directory. Claude Code 2.1.241 asks whether that workspace is trusted before it accepts slash commands, so AI Meter's `/usage` input is consumed by the trust prompt and the collector times out.

Authentication is not the failing component. `claude auth status` succeeds when run in the user's normal macOS context. The failure is isolated to the interactive usage command and is reproducible with the production collector.

## Goals

- Never ask Claude Code to trust `/`, the user's home directory, or an unrelated project.
- Run Claude usage collection in a dedicated, empty AI Meter workspace.
- Require an explicit, one-time user confirmation before Claude trusts that workspace.
- Keep using the official Claude Code CLI and its built-in `/usage` command.
- Make first-run setup and subsequent collection failures distinguishable in AI Meter.
- Preserve the existing Claude usage parser and provider presentation once collection succeeds.

## Non-goals

- AI Meter will not read or reuse Claude OAuth tokens directly.
- AI Meter will not call undocumented Anthropic account endpoints.
- AI Meter will not pass `--dangerously-skip-permissions` or automatically answer a trust prompt.
- AI Meter will not trust a workspace on the user's behalf.
- This change will not redesign the provider card or floating detail panel.

## Design

### Dedicated workspace

The app resolves a stable directory under its user-scoped Application Support container:

`~/Library/Application Support/AI Meter/ClaudeUsageWorkspace`

The directory contains no project files and exists only to give Claude Code a narrow workspace for `/usage`. A small workspace-resolver component creates the directory when needed and returns its URL. Directory creation failures surface as a collection transport failure without exposing private paths in the UI.

`CommandRequest` gains an optional working-directory URL. `PTYCommandRunner` assigns it to `Process.currentDirectoryURL` before launch. Existing collectors continue inheriting their current directory unless they explicitly provide one.

### Claude collector behavior

`ClaudeCollector` uses the dedicated workspace for both `auth status` and the interactive usage session. The interactive command also requests Claude's screen-reader output so the terminal text is deterministic and less dependent on animation or cursor positioning. Project customizations remain disabled for this isolated usage session so unrelated hooks, MCP servers, and project instructions cannot delay or alter collection.

The collector never sends `y` or otherwise responds to a trust question. If the sanitized output contains Claude's workspace trust prompt, the collector returns a dedicated setup-required error rather than waiting for a generic timeout.

### One-time setup

When setup is required, AI Meter presents an actionable Claude state instead of `Unavailable`: the user is told that the isolated usage workspace needs one-time approval. The setup action opens Terminal in that exact directory and starts Claude Code. The user reviews the displayed path, chooses **Yes, I trust this folder**, and exits Claude. No password, OAuth token, or account credential is handled by AI Meter.

After setup, the user can refresh AI Meter or wait for the next scheduled refresh. The collector starts in the now-trusted isolated directory and `/usage` returns normally.

For the initial repair and acceptance test, the same Terminal setup flow may be launched manually after the updated app is installed.

## Data flow

1. Refresh coordinator asks `ClaudeCollector` for a snapshot.
2. Workspace resolver creates or returns the isolated Claude usage workspace.
3. Collector runs `claude auth status` in that workspace.
4. If logged out, the existing sign-in state is returned.
5. If logged in, collector starts Claude in the same workspace and sends `/usage`, followed by `/exit`.
6. If Claude displays a trust prompt, collection returns setup-required immediately.
7. Otherwise the existing parser converts `/usage` output into a `UsageSnapshot`.
8. AI Meter displays the snapshot or the actionable setup state.

## Error handling

- CLI missing: retain the existing not-installed state.
- Authentication missing: retain the existing sign-in state.
- Workspace trust missing: show one-time setup required, not request timed out.
- Workspace creation failure: show unavailable with a non-sensitive setup error.
- Genuine process timeout after trust: retain request timed out.
- Unrecognized `/usage` output: retain the existing unsupported-output handling and cached snapshot behavior.

No raw CLI output, OAuth material, API keys, full home paths, or trust responses are written to application logs.

## Testing

Implementation follows red-green-refactor:

1. A command-runner test proves a supplied working directory reaches the child process.
2. A Claude collector test proves authentication and usage requests receive the dedicated workspace.
3. A Claude collector test proves a trust prompt becomes setup-required instead of timing out or unrecognized output.
4. Presentation tests prove setup-required is actionable and distinct from sign-in and unavailable states.
5. Existing unit and integration tests remain green.
6. A real CLI smoke test runs after the isolated workspace has been trusted.
7. The signed application is reinstalled and its Claude card/detail state is verified through the macOS UI.

## Acceptance criteria

- The installed app never launches Claude with `/` as its workspace.
- The first untrusted run gives a clear one-time setup state.
- AI Meter never approves workspace trust automatically.
- After explicit trust, Claude usage appears instead of `Unavailable` or `Request timed out`.
- Codex and DeepSeek collection behavior is unchanged.
- Full tests, app build, signature checks, and manual UI verification pass.
