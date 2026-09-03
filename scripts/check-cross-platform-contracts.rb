#!/usr/bin/env ruby
# frozen_string_literal: true

require "json"
require "open3"
require "pathname"
require "time"

root = Pathname.new(ARGV.fetch(0, File.expand_path("..", __dir__))).expand_path
errors = []

def read_json(path, errors)
  JSON.parse(path.read)
rescue Errno::ENOENT
  errors << "Missing cross-platform contract: #{path}"
  nil
rescue JSON::ParserError => error
  errors << "Invalid JSON in #{path}: #{error.message}"
  nil
end

def git_tracks_executable?(root, path)
  relative_path = path.relative_path_from(root).to_s
  output, status = Open3.capture2(
    "git",
    "-C",
    root.to_s,
    "ls-files",
    "--stage",
    "--",
    relative_path,
  )
  status.success? && output.each_line.any? { |line| line.start_with?("100755 ") }
rescue Errno::ENOENT
  false
end

version_path = root + "VERSION"
plist_path = root + "Sources/AIMeterApp/Resources/Info.plist"
shared_version = version_path.file? ? version_path.read.strip : nil
plist_version = plist_path.file? ? plist_path.read[/<key>CFBundleShortVersionString<\/key>\s*<string>([^<]+)<\/string>/m, 1] : nil
errors << "VERSION is missing or empty" if shared_version.nil? || shared_version.empty?
if shared_version && plist_version && shared_version != plist_version
  errors << "Shared version #{shared_version} does not match macOS bundle version #{plist_version}"
end

windows_package = read_json(root + "windows/package.json", errors)
tauri_config = read_json(root + "windows/src-tauri/tauri.conf.json", errors)
tauri_release_config = read_json(root + "windows/src-tauri/tauri.release.conf.json", errors)
tauri_preview_release_config = read_json(root + "windows/src-tauri/tauri.preview.release.conf.json", errors)
cargo_path = root + "windows/src-tauri/Cargo.toml"
cargo_version = cargo_path.file? ? cargo_path.read[/\A\[package\].*?^version\s*=\s*"([^"]+)"/m, 1] : nil
if shared_version
  {
    "Windows package" => windows_package&.fetch("version", nil),
    "Tauri config" => tauri_config&.fetch("version", nil),
    "Cargo package" => cargo_version,
  }.each do |label, platform_version|
    errors << "#{label} version #{platform_version || "missing"} does not match #{shared_version}" unless platform_version == shared_version
  end
end
errors << "Windows npm lockfile is missing" unless (root + "windows/package-lock.json").file?
errors << "Windows Cargo lockfile is missing" unless (root + "windows/src-tauri/Cargo.lock").file?

if tauri_config
  updater = tauri_config.dig("plugins", "updater")
  errors << "Windows updater public key is missing" if updater&.fetch("pubkey", "").empty?
  endpoints = updater&.fetch("endpoints", [])
  unless endpoints == ["https://github.com/sljzdotcom/AI-Token-Meter/releases/latest/download/latest.json"]
    errors << "Windows updater endpoint must use the fixed public GitHub Release feed"
  end
  if tauri_config.dig("bundle", "createUpdaterArtifacts") == true
    errors << "Ordinary Windows builds must not require updater signing credentials"
  end
end

unless tauri_release_config&.dig("bundle", "createUpdaterArtifacts") == true
  errors << "Windows release config must create signed updater artifacts"
end
unless tauri_preview_release_config&.dig("bundle", "createUpdaterArtifacts") == true
  errors << "Windows preview release config must create signed updater artifacts"
end
preview_endpoints = tauri_preview_release_config&.dig("plugins", "updater", "endpoints")
unless preview_endpoints == ["https://github.com/sljzdotcom/AI-Token-Meter/releases/download/windows-preview-feed/latest-preview.json"]
  errors << "Windows preview updater must use its fixed public preview feed"
end

windows_ci_path = root + ".github/workflows/windows-ci.yml"
if windows_ci_path.file?
  windows_ci = windows_ci_path.read
  if windows_ci.include?("--no-bundle")
    errors << "Windows CI must build the NSIS installer, not only the desktop executable"
  end
  errors << "Windows CI must upload the NSIS installer artifact" unless windows_ci.include?("actions/upload-artifact@") && windows_ci.include?("bundle/nsis")
else
  errors << "Windows CI workflow is missing"
end

release_workflow_path = root + ".github/workflows/release.yml"
if release_workflow_path.file?
  release_workflow = release_workflow_path.read
  errors << "Cross-platform release must require the Tauri signing secret" unless release_workflow.include?("secrets.TAURI_SIGNING_PRIVATE_KEY")
  errors << "Cross-platform release must wait for both platform jobs" unless release_workflow.include?("needs: [macos-preflight, windows-release]")
  errors << "Cross-platform release must publish the Windows updater manifest" unless release_workflow.include?("latest.json") && release_workflow.include?("windows-x86_64")
  errors << "Tagged releases must run the synchronized version contract" unless release_workflow.include?("ruby scripts/check-cross-platform-contracts.rb .")
  errors << "Preview releases must use the isolated config and feed" unless release_workflow.include?("tauri.preview.release.conf.json") && release_workflow.include?("windows-preview-feed") && release_workflow.include?("latest-preview.json")
  errors << "Cross-platform release must keep GitHub assets in a draft until verification passes" unless release_workflow.include?('gh release edit "v${VERSION}" --draft=false')
  errors << "Windows updater asset must be verified with the embedded Tauri public key" unless release_workflow.include?("--example verify_update_signature")
  unless release_workflow.include?("group: cross-platform-release") && release_workflow.include?("Back up public update feeds before publication")
    errors << "Cross-platform publications must serialize and back up both public update feeds"
  end
  unless release_workflow.include?("Rollback public update feeds and release visibility") && release_workflow.include?("prior-appcast.xml") && release_workflow.include?("prior-preview-feed.json") && release_workflow.include?("release_may_return_to_draft") && release_workflow.include?("publication-attempted") && release_workflow.include?('--draft=true')
    errors << "Failed publication must restore both feeds and return the target release to draft"
  end
  unless release_workflow.include?("probe_github_release") && release_workflow.include?("exact public recovery assets")
    errors << "Recovery must distinguish an explicit 404 and reuse the exact public assets"
  end
else
  errors << "Cross-platform release workflow is missing"
end

cross_platform_release_path = root + "scripts/package-cross-platform-release.sh"
if cross_platform_release_path.file?
  release_script = cross_platform_release_path.read
  errors << "Cross-platform release entry must preserve local Sparkle signing" unless release_script.include?("package-update-release.sh")
  errors << "Cross-platform release entry must create a draft" unless release_script.include?("gh release create") && release_script.include?("--draft")
  errors << "Cross-platform release entry must dispatch the tagged workflow" unless release_script.include?("gh workflow run release.yml") && release_script.include?('--ref "v$VERSION"')
  errors << "Preview packaging must not modify the stable appcast" unless release_script.include?("AI_METER_RELEASE_CHANNEL") && release_script.include?("preview-appcast.xml")
  unless release_script.include?("SmartScreen") && release_script.include?("unknown publisher")
    errors << "Windows Preview release notes must explain the SmartScreen unknown-publisher warning"
  end
  errors << "Stable appcast must not be pushed before the GitHub assets are public" if release_script.include?("git add appcast.xml")
  errors << "Cross-platform release entry must be executable" unless git_tracks_executable?(root, cross_platform_release_path)
else
  errors << "Cross-platform release script is missing"
end

settings_source = root + "windows/src/settings/SettingsWindow.tsx"
shell_source = root + "windows/src/Shell.tsx"
rust_entry = root + "windows/src-tauri/src/lib.rs"
credential_prompt = root + "windows/src-tauri/src/platform/windows/credential_prompt.rs"
if [settings_source, shell_source, rust_entry, credential_prompt].all?(&:file?)
  settings_text = settings_source.read
  shell_text = shell_source.read
  rust_text = rust_entry.read
  prompt_text = credential_prompt.read
  if settings_text.include?('type="password"') || settings_text.include?("pendingDeepSeekKey") || shell_text.include?('{ candidate }') || rust_text.match?(/replace_deepseek_api_key[\s\S]{0,250}candidate:\s*String/)
    errors << "DeepSeek API Key must never enter the WebView or string IPC"
  end
  unless prompt_text.include?("CredUIPromptForCredentialsW") && prompt_text.include?("CREDUI_FLAGS_DO_NOT_PERSIST")
    errors << "DeepSeek replacement must use the non-persistent native Windows credential prompt"
  end
else
  errors << "DeepSeek protected native credential flow is incomplete"
end

schema = read_json(root + "contracts/schemas/usage-snapshot.schema.json", errors)
presentation = read_json(root + "contracts/presentation/providers.json", errors)
expected_providers = %w[claude codex deepseek]
expected_names = ["Claude Code", "OpenAI Codex", "DeepSeek"]
expected_identity = expected_providers.zip(expected_names).to_h
allowed_statuses = %w[
  fresh cached refreshing notInstalled authenticationRequired setupRequired unavailable unrecognizedOutput
]

if schema
  schema_statuses = schema.dig("properties", "status", "enum")
  errors << "Snapshot schema status enum is incomplete" unless schema_statuses == allowed_statuses
  schema_providers = schema.dig("properties", "providerId", "enum")
  errors << "Snapshot schema provider enum is incomplete" unless schema_providers == expected_providers
end

if presentation
  providers = presentation["providers"]
  if presentation["schemaVersion"] != 1 || !providers.is_a?(Array)
    errors << "Provider presentation contract must use schemaVersion 1 and a providers array"
  else
    errors << "Provider order or IDs changed" unless providers.map { |item| item["id"] } == expected_providers
    errors << "Provider display names changed" unless providers.map { |item| item["displayName"] } == expected_names
    errors << "Provider logo keys must be unique" unless providers.map { |item| item["logoKey"] }.uniq.length == 3
    deepseek = providers.find { |item| item["id"] == "deepseek" }
    unless deepseek && deepseek["progressSemantics"] == "consumedFromBalanceBaseline"
      errors << "DeepSeek must use consumed-from-balance progress semantics"
    end
  end
end

fixture_paths = (root + "contracts/fixtures").glob("*.json").sort
errors << "Expected exactly four shared snapshot fixtures" unless fixture_paths.length == 4
fixture_paths.each do |path|
  fixture = read_json(path, errors)
  next unless fixture

  errors << "#{path.basename}: schemaVersion must be 1" unless fixture["schemaVersion"] == 1
  errors << "#{path.basename}: invalid providerId" unless expected_providers.include?(fixture["providerId"])
  unless expected_identity[fixture["providerId"]] == fixture["displayName"]
    errors << "#{path.basename}: displayName does not match providerId"
  end
  errors << "#{path.basename}: invalid status" unless allowed_statuses.include?(fixture["status"])
  ratio = fixture["usedRatio"]
  unless ratio.nil? || (ratio.is_a?(Numeric) && ratio.finite? && ratio.between?(0, 1))
    errors << "#{path.basename}: usedRatio must be null or between 0 and 1"
  end
  begin
    Time.iso8601(fixture.fetch("fetchedAt"))
  rescue KeyError, ArgumentError
    errors << "#{path.basename}: fetchedAt must be RFC 3339"
  end
end

parity_path = root + "contracts/parity/features.yml"
if parity_path.file?
  parity = parity_path.read
  errors << "Parity matrix must declare schema_version 1" unless parity.match?(/^schema_version: 1$/)
  errors << "Parity matrix must include REQ identifiers" unless parity.include?("requirement: REQ-")
  errors << "Parity matrix must include macOS evidence" unless parity.include?("macos:") && parity.include?("evidence:")
  errors << "Parity matrix must include Windows state" unless parity.include?("windows:")
  allowed_parity_statuses = %w[complete planned deferred not-applicable blocked in-progress]
  parity.scan(/^\s+status:\s+(\S+)$/).flatten.each do |status|
    errors << "Parity matrix contains invalid status #{status}" unless allowed_parity_statuses.include?(status)
  end
  parity.scan(/^\s+evidence:\s+(\S+)$/).flatten.each do |evidence|
    errors << "Parity evidence does not exist: #{evidence}" unless (root + evidence).exist?
  end
else
  errors << "Missing cross-platform parity matrix: #{parity_path}"
end

secret_patterns = {
  "bearer token" => /Bearer\s+[A-Za-z0-9._~+\/-]{16,}/i,
  "Telegram bot token" => /\b\d{8,10}:[A-Za-z0-9_-]{30,}\b/,
  "OpenAI-style API key" => /\bsk-[A-Za-z0-9_-]{16,}\b/,
  "cookie header" => /\b(?:set-)?cookie\s*:/i,
  "phone number" => /(?<!\d)1[3-9]\d{9}(?!\d)/,
}
(root + "contracts").glob("**/*").select(&:file?).each do |path|
  content = path.read
  secret_patterns.each do |label, pattern|
    errors << "#{path.relative_path_from(root)} contains a possible #{label}" if content.match?(pattern)
  end
end

if errors.empty?
  puts "Cross-platform contract checks passed (#{fixture_paths.length} fixtures)."
  exit 0
end

warn "Cross-platform contract checks failed:"
errors.uniq.each { |error| warn "- #{error}" }
exit 1
