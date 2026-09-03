#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="${1:-$(cd "$SCRIPT_DIR/.." && pwd)}"

ruby - "$PROJECT_DIR" <<'RUBY'
require "pathname"
require "uri"

root = Pathname.new(ARGV.fetch(0)).expand_path
errors = []

required_files = %w[
  README.md
  LICENSE
  SECURITY.md
  CONTRIBUTING.md
  CODE_OF_CONDUCT.md
  SUPPORT.md
  .github/ISSUE_TEMPLATE/bug_report.yml
  .github/ISSUE_TEMPLATE/feature_request.yml
  .github/ISSUE_TEMPLATE/config.yml
  .github/pull_request_template.md
  .github/workflows/ci.yml
  docs/assets/screenshots/floating-strip.png
  docs/assets/screenshots/provider-detail.png
  docs/assets/screenshots/settings.png
  Sources/AIMeterApp/Resources/Info.plist
  docs/README.md
  docs/project-status.md
  docs/requirements-backlog.md
  docs/architecture/decisions.md
  docs/development/maintenance-playbook.md
  docs/development/2026-09-02-project-retrospective.md
  docs/development/testing.md
]
required_directories = %w[
  .github/ISSUE_TEMPLATE
  .github/workflows
  docs/design/specifications
  docs/design/implementation-plans
]
forbidden_paths = %w[
  docs/superpowers
  docs/next-phase-requirements.md
]

required_files.each do |path|
  errors << "Required documentation file is missing: #{path}" unless (root + path).file?
end
required_directories.each do |path|
  errors << "Required documentation directory is missing: #{path}" unless (root + path).directory?
end
forbidden_paths.each do |path|
  errors << "Obsolete documentation path still exists: #{path}" if (root + path).exist?
end

skip_components = %w[.git .build .worktrees .superpowers dist node_modules]
markdown_files = root.glob("**/*.md").reject do |path|
  path.relative_path_from(root).each_filename.any? { |component| skip_components.include?(component) }
end

markdown_files.each do |source|
  content = source.read
  if content.match?(/^\s*(?:bash\s+)?scripts\/check-public-release\.sh\s+\.\s*$/)
    relative_source = source.relative_path_from(root)
    errors << "Ambiguous public release safety invocation in #{relative_source}; use --repository for a repository scan"
  end
  content.scan(/!?\[[^\]]*\]\(([^)]+)\)/).flatten.each do |raw_target|
    target = raw_target.strip
    target = target[1...target.index(">")].to_s if target.start_with?("<") && target.include?(">")
    target = target.split(/\s+["']/, 2).first.to_s
    next if target.empty? || target.start_with?("#")
    next if target.match?(/\A(?:https?|mailto|tel|data):/i)

    path_part = target.split("#", 2).first.to_s.split("?", 2).first.to_s
    next if path_part.empty?

    begin
      decoded = URI.decode_www_form_component(path_part)
    rescue ArgumentError
      decoded = path_part
    end
    destination = Pathname.new(decoded)
    destination = source.dirname + destination unless destination.absolute?
    unless destination.cleanpath.exist?
      relative_source = source.relative_path_from(root)
      errors << "Broken local Markdown link in #{relative_source}: #{target}"
    end
  end
end

readme_path = root + "README.md"
plist_path = root + "Sources/AIMeterApp/Resources/Info.plist"
testing_path = root + "docs/development/testing.md"

if readme_path.file? && plist_path.file?
  readme = readme_path.read
  plist = plist_path.read
  readme_version = readme[/!\[Version\s+([^\]]+)\]/i, 1]
  plist_version = plist[/<key>CFBundleShortVersionString<\/key>\s*<string>([^<]+)<\/string>/m, 1]
  if readme_version.nil?
    errors << "README version badge is missing"
  elsif plist_version.nil?
    errors << "Info.plist CFBundleShortVersionString is missing"
  elsif readme_version != plist_version
    errors << "README version #{readme_version} does not match Info.plist #{plist_version}"
  end

  public_readme_requirements = {
    "English project summary" => "macOS and Windows usage meter",
    "Windows 11 x64 support boundary" => "Windows 11 x64",
    "Windows SmartScreen boundary" => "SmartScreen",
    "Windows Authenticode boundary" => "Authenticode",
    "public author credit" => "Author: Miller",
    "MIT license notice" => "MIT License",
    "v#{plist_version} download guidance" => "Download v#{plist_version}",
    "ad-hoc signing notice" => "ad-hoc signed",
    "notarization notice" => "not notarized",
    "floating strip screenshot" => "docs/assets/screenshots/floating-strip.png",
    "provider detail screenshot" => "docs/assets/screenshots/provider-detail.png",
    "settings screenshot" => "docs/assets/screenshots/settings.png",
  }
  public_readme_requirements.each do |label, text|
    errors << "README #{label} is missing" unless readme.include?(text)
  end
end

%w[floating-strip.png provider-detail.png settings.png].each do |filename|
  screenshot = root + "docs/assets/screenshots/#{filename}"
  next unless screenshot.file?
  errors << "Documentation screenshot is not a PNG: #{filename}" unless screenshot.binread(8) == "\x89PNG\r\n\x1A\n".b
end

if readme_path.file? && testing_path.file?
  readme = readme_path.read
  testing = testing_path.read
  readme_tests = readme[/!\[Tests\s+(\d+)\]/i, 1]
  documented_tests = testing[/\*\*(\d+)(?:\s+tests|\s*个测试)/i, 1]
  if readme_tests.nil?
    errors << "README test-count badge is missing"
  elsif documented_tests.nil?
    errors << "Testing guide baseline is missing"
  elsif readme_tests != documented_tests
    errors << "README test count #{readme_tests} does not match testing guide #{documented_tests}"
  end
end

if errors.empty?
  puts "Documentation checks passed (#{markdown_files.length} Markdown files)."
  exit 0
end

warn "Documentation checks failed:"
errors.each { |error| warn "- #{error}" }
exit 1
RUBY
