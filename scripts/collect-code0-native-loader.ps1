# Metadata only: never starts Cargo, the fixture, a GUI, or a debugger.
# Run with pwsh after the existing runtime step in a fresh ephemeral Windows job.
[CmdletBinding()]
param()

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$script:FileLimit = 1MB
$script:PacketLimit = 8MB
$script:ReportReserve = 256KB
$script:PacketBytes = 0L
$script:OutputDirectory = $null
$script:CreatedOutputDirectory = $false
$script:ToolTerminationUnknown = $false
$script:Problems = [System.Collections.Generic.List[string]]::new()
$script:Files = [System.Collections.Generic.List[object]]::new()
$utf8 = [System.Text.UTF8Encoding]::new($false, $true)

function Assert-PlainPath([string] $Path) {
    $full = [IO.Path]::GetFullPath($Path)
    $cursor = $full
    while ($cursor) {
        $item = Get-Item -LiteralPath $cursor -Force -ErrorAction SilentlyContinue
        if ($null -ne $item -and ($item.Attributes -band [IO.FileAttributes]::ReparsePoint)) {
            throw 'Reparse point in a selected path; collection refused.'
        }
        $parent = [IO.Path]::GetDirectoryName($cursor)
        if ($parent -eq $cursor) { break }
        $cursor = $parent
    }
    return $full
}

function Assert-Within([string] $Path, [string] $Root) {
    $full = Assert-PlainPath $Path
    $prefix = (Assert-PlainPath $Root).TrimEnd('\', '/') + [IO.Path]::DirectorySeparatorChar
    if (-not $full.StartsWith($prefix, [StringComparison]::OrdinalIgnoreCase)) {
        throw 'Selected path is outside its fixed evidence root.'
    }
    return $full
}

function Save-Text([string] $Name, [string] $Text) {
    if ($Name -notmatch '^[a-z0-9][a-z0-9.-]{0,100}$') { throw 'Invalid packet filename.' }
    $bytes = $utf8.GetBytes($Text)
    $limit = if ($Name -eq 'metadata.json') { $script:PacketLimit } else { $script:PacketLimit - $script:ReportReserve }
    if ($bytes.Length -gt $script:FileLimit -or
        ($Name -eq 'metadata.json' -and $bytes.Length -gt $script:ReportReserve) -or
        $script:PacketBytes + $bytes.Length -gt $limit) {
        throw 'Metadata packet size limit exceeded; no partial file accepted.'
    }
    $path = Assert-Within (Join-Path $script:OutputDirectory $Name) $script:OutputDirectory
    $stream = [IO.File]::Open($path, [IO.FileMode]::CreateNew, [IO.FileAccess]::Write, [IO.FileShare]::None)
    try { $stream.Write($bytes, 0, $bytes.Length) } finally { $stream.Dispose() }
    $script:PacketBytes += $bytes.Length
    $script:Files.Add([ordered]@{ name = $Name; bytes = $bytes.Length })
}

function Get-FileIdentity([string] $Path, [long] $Maximum = 512MB) {
    $path = Assert-PlainPath $Path
    $item = Get-Item -LiteralPath $path -Force
    if ($item.PSIsContainer -or $item.Length -gt $Maximum) { throw 'Selected file exceeds its read limit.' }
    $stream = [IO.File]::OpenRead($path)
    $sha = [Security.Cryptography.SHA256]::Create()
    try {
        $buffer = [byte[]]::new(64KB); $readBytes = 0L
        while (($length = $stream.Read($buffer, 0, $buffer.Length)) -gt 0) {
            $readBytes += $length
            if ($readBytes -gt $Maximum) { throw 'Selected file grew beyond its hash read limit.' }
            [void]$sha.TransformBlock($buffer, 0, $length, $buffer, 0)
        }
        [void]$sha.TransformFinalBlock([byte[]]::new(0), 0, 0)
        if ($readBytes -ne $item.Length) { throw 'Selected file changed size during hashing.' }
        $hash = [Convert]::ToHexString($sha.Hash).ToLowerInvariant()
    }
    finally { $sha.Dispose(); $stream.Dispose() }
    return [ordered]@{ path = $path; bytes = $item.Length; sha256 = $hash }
}

function Read-SmallText([string] $Path, [int] $Maximum = 256KB) {
    $path = Assert-PlainPath $Path
    $stream = [IO.File]::OpenRead($path)
    try {
        if ($stream.Length -gt $Maximum) { throw 'Own text input exceeds its byte limit.' }
        $memory = [IO.MemoryStream]::new()
        $buffer = [byte[]]::new(4096)
        try {
            while (($length = $stream.Read($buffer, 0, $buffer.Length)) -gt 0) {
                if ($memory.Length + $length -gt $Maximum) { throw 'Own text input grew beyond its byte limit.' }
                $memory.Write($buffer, 0, $length)
            }
            $memory.Position = 0
            $reader = [IO.StreamReader]::new($memory, $utf8, $true, 4096, $true)
            try { return $reader.ReadToEnd() } finally { $reader.Dispose() }
        } finally { $memory.Dispose() }
    } finally { $stream.Dispose() }
}

function Get-ToolInfo([string] $Path) {
    if (-not $Path) { return [ordered]@{ status = 'unavailable' } }
    $path = Assert-PlainPath $Path
    $version = [Diagnostics.FileVersionInfo]::GetVersionInfo($path)
    return [ordered]@{
        status = 'selected'; path = $path
        fileVersion = $version.FileVersion; productVersion = $version.ProductVersion
    }
}

function Find-InstalledTools {
    # Fixed installed locations only. No PATH, vswhere, registry, downloads, or recursive scan.
    $msvc = @()
    foreach ($root in @($env:ProgramFiles, ${env:ProgramFiles(x86)})) {
        if (-not $root) { continue }
        foreach ($year in @('18', '2026', '2022')) {
            foreach ($edition in @('Enterprise', 'Professional', 'Community', 'BuildTools')) {
                $base = Join-Path $root "Microsoft Visual Studio\$year\$edition\VC\Tools\MSVC"
                if (-not (Test-Path -LiteralPath $base -PathType Container)) { continue }
                $base = Assert-PlainPath $base
                $count = 0
                foreach ($directory in [IO.Directory]::EnumerateDirectories($base)) {
                    if (++$count -gt 32) { throw 'MSVC version directory limit exceeded.' }
                    $name = [IO.Path]::GetFileName($directory)
                    if ($name -notmatch '^\d+\.\d+\.\d+$') { continue }
                    $bin = Assert-Within (Join-Path $directory 'bin\Hostx64\x64') $base
                    if (Test-Path -LiteralPath (Join-Path $bin 'dumpbin.exe') -PathType Leaf) {
                        $msvc += [pscustomobject]@{ version = [version]$name; bin = $bin }
                    }
                }
            }
        }
    }
    $selected = $msvc | Sort-Object -Property @{ Expression = 'version'; Descending = $true }, bin | Select-Object -First 1
    $dumpbin = $null; $link = $null
    if ($null -ne $selected) {
        $dumpbin = Join-Path $selected.bin 'dumpbin.exe'
        $candidate = Join-Path $selected.bin 'link.exe'
        if (Test-Path -LiteralPath $candidate -PathType Leaf) { $link = $candidate }
    }
    $sdks = @()
    if (${env:ProgramFiles(x86)}) {
        $base = Join-Path ${env:ProgramFiles(x86)} 'Windows Kits\10\bin'
        if (Test-Path -LiteralPath $base -PathType Container) {
            $base = Assert-PlainPath $base
            $count = 0
            foreach ($directory in [IO.Directory]::EnumerateDirectories($base)) {
                if (++$count -gt 32) { throw 'SDK version directory limit exceeded.' }
                $name = [IO.Path]::GetFileName($directory)
                if ($name -notmatch '^\d+\.\d+\.\d+\.\d+$') { continue }
                $bin = Assert-Within (Join-Path $directory 'x64') $base
                if (Test-Path -LiteralPath (Join-Path $bin 'mt.exe') -PathType Leaf) {
                    $sdks += [pscustomobject]@{ version = [version]$name; bin = $bin }
                }
            }
        }
    }
    $sdk = $sdks | Sort-Object -Property @{ Expression = 'version'; Descending = $true }, bin | Select-Object -First 1
    $mt = $null; $rc = $null; $sdkVersion = $null
    if ($null -ne $sdk) {
        $mt = Join-Path $sdk.bin 'mt.exe'; $sdkVersion = $sdk.version.ToString()
        $candidate = Join-Path $sdk.bin 'rc.exe'
        if (Test-Path -LiteralPath $candidate -PathType Leaf) { $rc = $candidate }
    }
    return [ordered]@{
        selection = 'highest installed numeric version, then path; fixed x64 locations'
        sdkDirectoryVersion = $sdkVersion
        dumpbin = (Get-ToolInfo $dumpbin); link = (Get-ToolInfo $link)
        mt = (Get-ToolInfo $mt); rc = (Get-ToolInfo $rc)
    }
}

function Invoke-MetadataTool([string] $Tool, [string[]] $Arguments, [string] $OutputFile = '') {
    # Retain at most 768 KiB combined output; drain both pipes in fixed chunks.
    # Only the selected dumpbin.exe / mt.exe are ever passed by the caller.
    if ($script:ToolTerminationUnknown) { throw 'Further metadata tools blocked after unknown owned-tool termination.' }
    $result = [ordered]@{
        status = 'unavailable'; exitCode = $null; truncated = $false; stdout = ''; stderr = ''
        termination = 'not_started'; killAttempted = $false; waitConfirmed = $null; cleanupError = $null
    }
    if (-not $Tool) { return $result }
    $process = [Diagnostics.Process]::new()
    $process.StartInfo.FileName = Assert-PlainPath $Tool
    $process.StartInfo.UseShellExecute = $false
    $process.StartInfo.CreateNoWindow = $true
    $process.StartInfo.RedirectStandardOutput = $true
    $process.StartInfo.RedirectStandardError = $true
    foreach ($argument in $Arguments) { $process.StartInfo.ArgumentList.Add($argument) }
    $streams = @(); $clock = [Diagnostics.Stopwatch]::StartNew(); $started = $false
    try {
        if (-not $process.Start()) { throw 'Metadata tool did not start.' }
        $started = $true
        foreach ($pipe in @($process.StandardOutput.BaseStream, $process.StandardError.BaseStream)) {
            $buffer = [byte[]]::new(4096)
            $streams += @{ pipe = $pipe; buffer = $buffer; task = $pipe.ReadAsync($buffer, 0, $buffer.Length); done = $false; kept = [IO.MemoryStream]::new() }
        }
        $keptBytes = 0
        while (-not ($streams[0].done -and $streams[1].done -and $process.HasExited)) {
            foreach ($entry in $streams) {
                if ($entry.done -or -not $entry.task.IsCompleted) { continue }
                $length = $entry.task.GetAwaiter().GetResult()
                if ($length -eq 0) { $entry.done = $true; continue }
                if ($keptBytes + $length -le 768KB) {
                    $entry.kept.Write($entry.buffer, 0, $length); $keptBytes += $length
                } else { $result.truncated = $true }
                $entry.task = $entry.pipe.ReadAsync($entry.buffer, 0, $entry.buffer.Length)
            }
            $oversizeFile = $OutputFile -and (Test-Path -LiteralPath $OutputFile) -and
                ((Get-Item -LiteralPath $OutputFile).Length -gt 64KB)
            if ($result.truncated -or $oversizeFile -or $clock.Elapsed.TotalSeconds -gt 30) {
                $result.status = if ($oversizeFile -or $result.truncated) { 'limit_exceeded' } else { 'timeout' }
                break
            }
            [Threading.Thread]::Sleep(10)
        }
        # MSVC diagnostics are not guaranteed UTF-8. Preserve replacement-decoded text, never binary.
        if ($streams.Count -eq 2) {
            $result.stdout = [Text.Encoding]::UTF8.GetString($streams[0].kept.ToArray())
            $result.stderr = [Text.Encoding]::UTF8.GetString($streams[1].kept.ToArray())
        }
    } catch { $result.status = 'tool_error'; $result.stderr = $_.Exception.GetType().Name }
    finally {
        # Every successful Start has this single bounded cleanup/confirmation path,
        # including pipe/read/decoding exceptions. Dispose is never exit evidence.
        if ($started) {
            $confirmed = $false
            try { $confirmed = $process.HasExited } catch { $result.cleanupError = $_.Exception.GetType().Name }
            if (-not $confirmed) {
                $result.killAttempted = $true
                try { $process.Kill() } catch { $result.cleanupError = $_.Exception.GetType().Name }
                try {
                    $result.waitConfirmed = $process.WaitForExit(5000)
                    $confirmed = $result.waitConfirmed -eq $true
                } catch { $result.cleanupError = $_.Exception.GetType().Name }
            }
            if ($confirmed) {
                $result.termination = 'exit_confirmed'
                try { $result.exitCode = $process.ExitCode } catch { $result.cleanupError = $_.Exception.GetType().Name }
            }
            if (-not $confirmed) {
                $result.termination = 'termination_unknown'
                $script:ToolTerminationUnknown = $true
            }
            if ($result.status -eq 'unavailable') {
                $result.status = if ($result.termination -eq 'exit_confirmed' -and $result.exitCode -eq 0) { 'captured' } else { 'tool_failed' }
            }
            if ($result.termination -eq 'termination_unknown' -and $result.status -eq 'captured') { $result.status = 'tool_error' }
        }
        foreach ($entry in $streams) { try { $entry.kept.Dispose() } catch { } }
        try { $process.Dispose() } catch { }
        $clock.Stop()
    }
    return $result
}

function Save-ToolResult([string] $Name, $Result) {
    Save-Text $Name ("status=$($Result.status)`nexitCode=$($Result.exitCode)`ntruncated=$($Result.truncated)`ntermination=$($Result.termination)`nkillAttempted=$($Result.killAttempted)`nwaitConfirmed=$($Result.waitConfirmed)`ncleanupError=$($Result.cleanupError)`n[stdout]`n$($Result.stdout)`n[stderr]`n$($Result.stderr)")
    if ($Result.status -ne 'captured') { $script:Problems.Add("$Name : $($Result.status)") }
    if ($Result.termination -eq 'termination_unknown') {
        $script:Problems.Add("$Name : owned tool termination_unknown; collection stopped.")
        throw 'Owned metadata tool termination_unknown; no later tool or staging publication is allowed.'
    }
}

function Read-At([IO.BinaryReader] $Reader, [long] $Offset, [int] $Count) {
    if ($Count -lt 0 -or $Count -gt 64KB -or $Offset -lt 0 -or $Offset -gt $Reader.BaseStream.Length - $Count) {
        throw 'PE read outside bounded file range.'
    }
    [void]$Reader.BaseStream.Seek($Offset, [IO.SeekOrigin]::Begin)
    $bytes = $Reader.ReadBytes($Count)
    if ($bytes.Length -ne $Count) { throw 'Short PE metadata read.' }
    return ,$bytes
}

function Get-PeMetadata([string] $Path, [bool] $ReadManifests = $true) {
    # Seek-only PE/resource reader. No image load, API execution, or full-binary allocation.
    $reader = [IO.BinaryReader]::new([IO.File]::OpenRead((Assert-PlainPath $Path)))
    try {
        $dos = Read-At $reader 0 64
        if ($dos[0] -ne 0x4d -or $dos[1] -ne 0x5a) { throw 'Invalid DOS signature.' }
        $pe = [BitConverter]::ToUInt32($dos, 60)
        $coff = Read-At $reader $pe 24
        if ([BitConverter]::ToUInt32($coff, 0) -ne 0x4550) { throw 'Invalid PE signature.' }
        $sectionCount = [BitConverter]::ToUInt16($coff, 6)
        $optionalSize = [BitConverter]::ToUInt16($coff, 20)
        if ($sectionCount -gt 96 -or $optionalSize -gt 4096 -or $optionalSize -lt 96) { throw 'PE header limits exceeded.' }
        $optional = Read-At $reader ($pe + 24) $optionalSize
        $magic = [BitConverter]::ToUInt16($optional, 0)
        $directoryOffset = switch ($magic) { 0x10b { 96 } 0x20b { 112 } default { throw 'Unsupported PE optional header.' } }
        $metadata = [ordered]@{
            machine = ('0x{0:x4}' -f [BitConverter]::ToUInt16($coff, 4))
            subsystem = [BitConverter]::ToUInt16($optional, 68)
            manifests = @(); manifestStatus = 'not_requested'
        }
        if (-not $ReadManifests) { return $metadata }
        $numberOffset = $directoryOffset - 4
        if ($optional.Length -lt $directoryOffset + 24 -or [BitConverter]::ToUInt32($optional, $numberOffset) -lt 3) {
            $metadata.manifestStatus = 'no_resource_directory'; return $metadata
        }
        $resourceRva = [BitConverter]::ToUInt32($optional, $directoryOffset + 16)
        $resourceSize = [BitConverter]::ToUInt32($optional, $directoryOffset + 20)
        if ($resourceRva -eq 0 -or $resourceSize -eq 0) { $metadata.manifestStatus = 'no_resource_directory'; return $metadata }
        $sections = @()
        for ($index = 0; $index -lt $sectionCount; $index++) {
            $section = Read-At $reader ($pe + 24 + $optionalSize + 40 * $index) 40
            $sections += @{
                rva = [long][BitConverter]::ToUInt32($section, 12)
                rawSize = [long][BitConverter]::ToUInt32($section, 16)
                raw = [long][BitConverter]::ToUInt32($section, 20)
            }
        }
        function Convert-Rva([long] $Rva, [int] $Size) {
            $matches = @($sections | Where-Object { $Rva -ge $_.rva -and $Rva - $_.rva + $Size -le $_.rawSize })
            if ($matches.Count -ne 1) { throw 'Unmapped or ambiguous PE resource RVA.' }
            return $matches[0].raw + $Rva - $matches[0].rva
        }
        [void](Convert-Rva $resourceRva 16)
        function Read-Resource([long] $Offset, [int] $Size) {
            if ($Offset -lt 0 -or $Offset + $Size -gt $resourceSize) { throw 'Resource directory range exceeded.' }
            # Resolve every read separately: do not assume resources stay in one section.
            return ,(Read-At $reader (Convert-Rva ($resourceRva + $Offset) $Size) $Size)
        }
        function Get-ResourceEntries([long] $Offset) {
            $header = Read-Resource $Offset 16
            $count = [int][BitConverter]::ToUInt16($header, 12) + [BitConverter]::ToUInt16($header, 14)
            if ($count -gt 128) { throw 'Resource entry limit exceeded.' }
            $entries = @()
            for ($i = 0; $i -lt $count; $i++) {
                $row = Read-Resource ($Offset + 16 + 8 * $i) 8
                $name = [BitConverter]::ToUInt32($row, 0)
                $child = [BitConverter]::ToUInt32($row, 4)
                $label = $name.ToString()
                if ($name -band 0x80000000L) {
                    $nameOffset = $name -band 0x7fffffffL
                    $size = [BitConverter]::ToUInt16((Read-Resource $nameOffset 2), 0)
                    if ($size -gt 128) { throw 'Resource name limit exceeded.' }
                    $label = [Text.Encoding]::Unicode.GetString((Read-Resource ($nameOffset + 2) ($size * 2)))
                }
                $entries += @{ numeric = -not [bool]($name -band 0x80000000L); id = $name; label = $label; directory = [bool]($child -band 0x80000000L); offset = $child -band 0x7fffffffL }
            }
            return $entries
        }
        $types = @(Get-ResourceEntries 0 | Where-Object { $_.numeric -and $_.id -eq 24 })
        if ($types.Count -eq 0) { $metadata.manifestStatus = 'rt_manifest_absent'; return $metadata }
        if ($types.Count -ne 1 -or -not $types[0].directory) { throw 'Invalid RT_MANIFEST directory.' }
        $total = 0
        foreach ($name in (Get-ResourceEntries $types[0].offset)) {
            if (-not $name.directory) { throw 'Invalid manifest name directory.' }
            foreach ($language in (Get-ResourceEntries $name.offset)) {
                if ($language.directory -or $metadata.manifests.Count -ge 16) { throw 'Manifest resource depth/count limit exceeded.' }
                $data = Read-Resource $language.offset 16
                $rva = [BitConverter]::ToUInt32($data, 0); $size = [BitConverter]::ToUInt32($data, 4)
                $total += $size
                if ($size -gt 64KB -or $total -gt 512KB) { throw 'Manifest resource byte limit exceeded.' }
                $raw = Read-At $reader (Convert-Rva $rva $size) $size
                $text = if ($raw.Length -ge 2 -and (($raw[0] -eq 255 -and $raw[1] -eq 254) -or $raw[1] -eq 0)) {
                    [Text.Encoding]::Unicode.GetString($raw)
                } else { [Text.Encoding]::UTF8.GetString($raw) }
                $metadata.manifests += [ordered]@{ id = $name.label; numericId = $name.numeric; language = $language.label; bytes = $size; text = $text }
            }
        }
        $metadata.manifestStatus = 'enumerated'
        return $metadata
    } finally { $reader.Dispose() }
}

function Collect-OwnBuild([string] $PackageRoot) {
    $base = Assert-Within (Join-Path $PackageRoot 'target\debug\build') $PackageRoot
    $records = @()
    if (-not (Test-Path -LiteralPath $base -PathType Container)) {
        $script:Problems.Add('Own package build directory unavailable.')
        return @(@{ status = 'build_directory_unavailable' })
    }
    $count = 0
    foreach ($directory in [IO.Directory]::EnumerateDirectories($base, 'ai-token-meter-windows-*', [IO.SearchOption]::TopDirectoryOnly)) {
        if (++$count -gt 16) { throw 'Own package build directory limit exceeded.' }
        $directory = Assert-Within $directory $base
        if ([IO.Path]::GetFileName($directory) -notmatch '^ai-token-meter-windows-[a-f0-9]+$') { continue }
        $record = [ordered]@{ relativeDirectory = [IO.Path]::GetRelativePath($PackageRoot, $directory); association = 'own package candidate; not uniquely tied to fixture'; files = @() }
        foreach ($relative in @('output', 'out\resource.rc', 'out\manifest.xml', 'out\app.manifest', 'out\windows-app-manifest.xml')) {
            $path = Assert-Within (Join-Path $directory $relative) $directory
            if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
                $record.files += @{ source = $relative; status = 'absent' }
                continue
            }
            try {
                $text = Read-SmallText $path
                $identity = Get-FileIdentity $path 256KB
                if ($relative -eq 'output') {
                    $text = (@($text -split '\r?\n' | Where-Object { $_ -match '^cargo::?rustc-link-arg(?:-[a-z-]+)?=' }) -join "`n")
                }
                $name = "build-$count-" + ($relative -replace '[\\/]', '-').ToLowerInvariant() + '.txt'
                Save-Text $name $text
                $record.files += @{ source = $relative; sha256 = $identity.sha256; sourceBytes = $identity.bytes; packetFile = $name }
            } catch { $record.files += @{ source = $relative; status = 'unavailable_or_limit'; errorType = $_.Exception.GetType().Name }; $script:Problems.Add('Own package text could not be collected.') }
        }
        $records += $record
    }
    if ($records.Count -eq 0) {
        $script:Problems.Add('Own package build outputs unavailable.')
        return @(@{ status = 'own_package_build_outputs_unavailable' })
    }
    return $records
}

$report = [ordered]@{
    schema = 1; requirement = 'REQ-20261026-004'; kind = 'metadata_only'
    sourceSha = $env:GITHUB_SHA; runId = $env:GITHUB_RUN_ID; runAttempt = $env:GITHUB_RUN_ATTEMPT
    architecture = $env:RUNNER_ARCH; fixture = @{ status = 'unavailable' }
    tools = [ordered]@{ dumpbin = @{ status = 'unavailable' }; link = @{ status = 'unavailable' }; mt = @{ status = 'unavailable' }; rc = @{ status = 'unavailable' }; sdkDirectoryVersion = $null }
    ownBuild = @(); operations = @(); problems = @(); packetFiles = @()
    limitations = @('No fixture/product/test launch added.', 'No resolved DLL identity or missing-symbol proof.', 'No native behavioral RED/GREEN or COM acceptance.', 'Fresh same-job artifact; not the lost executable from run 40553916957.', 'Own build candidates are not assumed to be the fixture link invocation.')
}

try {
    if (-not $IsWindows -or $env:GITHUB_ACTIONS -ne 'true' -or
        $env:GITHUB_RUN_ID -notmatch '^\d{1,20}$' -or $env:GITHUB_RUN_ATTEMPT -notmatch '^\d{1,6}$' -or
        $env:GITHUB_SHA -notmatch '^[a-fA-F0-9]{40}$' -or $env:RUNNER_ARCH -ne 'X64') {
        throw 'Requires an identified fresh x64 Windows GitHub Actions job.'
    }
    if (-not $env:RUNNER_TEMP) { throw 'RUNNER_TEMP is unavailable.' }
    $tempRoot = Assert-PlainPath $env:RUNNER_TEMP
    if (-not (Test-Path -LiteralPath $tempRoot -PathType Container)) { throw 'RUNNER_TEMP is not a directory.' }
    $script:OutputDirectory = Assert-Within (Join-Path $tempRoot "code0-native-loader-$env:GITHUB_RUN_ID-$env:GITHUB_RUN_ATTEMPT") $tempRoot
    if ($null -ne (Get-Item -LiteralPath $script:OutputDirectory -Force -ErrorAction SilentlyContinue)) { throw 'Diagnostic directory already exists; refusing reuse.' }
    [void][IO.Directory]::CreateDirectory($script:OutputDirectory)
    $script:CreatedOutputDirectory = $true
    $repo = Assert-PlainPath (Split-Path -Parent $PSScriptRoot)
    if (-not $env:GITHUB_WORKSPACE -or
        -not $repo.Equals((Assert-PlainPath $env:GITHUB_WORKSPACE), [StringComparison]::OrdinalIgnoreCase) -or
        $env:CARGO_TARGET_DIR -or $env:CARGO_BUILD_TARGET) {
        throw 'Requires the checked-out workspace and conventional same-job Cargo target directory.'
    }
    $package = Assert-Within (Join-Path $repo 'windows\src-tauri') $repo
    $deps = Assert-Within (Join-Path $package 'target\debug\deps') $package
    $candidates = @()
    $candidateOverflow = $false
    if (Test-Path -LiteralPath $deps -PathType Container) {
        foreach ($path in [IO.Directory]::EnumerateFiles($deps, 'code0_native_fixture-*.exe', [IO.SearchOption]::TopDirectoryOnly)) {
            if ([IO.Path]::GetFileName($path) -notmatch '^code0_native_fixture-[a-f0-9]+\.exe$') { continue }
            if ($candidates.Count -ge 16) { $candidateOverflow = $true; break }
            $candidates += Assert-Within $path $deps
        }
    }
    $report.fixture = [ordered]@{
        status = if ($candidates.Count -eq 0) { 'unavailable' } elseif ($candidates.Count -eq 1) { 'unique_same_job_candidate' } else { 'ambiguous' }
        candidateRelativePaths = @($candidates | ForEach-Object { [IO.Path]::GetRelativePath($repo, $_) })
        candidateListLimitExceeded = $candidateOverflow
        binding = 'Unique exact-name match in fresh job target/debug/deps; no timestamp selection and no claim of Cargo executable-message binding.'
    }
    try { $report.tools = Find-InstalledTools } catch { $script:Problems.Add("Tool discovery unavailable: $($_.Exception.GetType().Name)") }
    if ($candidates.Count -eq 1) {
        $fixture = $candidates[0]
        $identity = $null
        try {
            $identity = Get-FileIdentity $fixture
            $report.fixture.identity = @{ relativePath = [IO.Path]::GetRelativePath($repo, $fixture); bytes = $identity.bytes; sha256 = $identity.sha256 }
        } catch { $script:Problems.Add('Unique fixture identity unavailable or exceeds 512 MiB.'); $report.fixture.status = 'identity_unavailable' }
        if ($null -eq $identity) { throw 'Cannot inspect an unbound fixture identity.' }
        $dumpbin = if ($report.tools.Contains('dumpbin') -and $report.tools.dumpbin.status -eq 'selected') { $report.tools.dumpbin.path } else { $null }
        $imports = $null
        foreach ($mode in @('headers', 'imports', 'dependents')) {
            $result = Invoke-MetadataTool $dumpbin @('/nologo', "/$mode", $fixture)
            $report.operations += @{ tool = 'dumpbin'; mode = $mode; status = $result.status; exitCode = $result.exitCode; truncated = $result.truncated; termination = $result.termination; killAttempted = $result.killAttempted; waitConfirmed = $result.waitConfirmed; cleanupError = $result.cleanupError }
            Save-ToolResult "fixture-$mode.txt" $result
            if ($mode -eq 'imports' -and $result.status -eq 'captured') { $imports = $result.stdout }
        }
        try {
            $pe = Get-PeMetadata $fixture
            $report.fixture.pe = $pe
            $index = 0
            foreach ($manifest in $pe.manifests) {
                $name = "fixture-resource-manifest-$index.xml"
                Save-Text $name $manifest.text
                $manifest.Remove('text'); $manifest.packetFile = $name; $index++
            }
            $mt = if ($report.tools.Contains('mt') -and $report.tools.mt.status -eq 'selected') { $report.tools.mt.path } else { $null }
            # Sibling staging is outside the upload directory and has no upload extension.
            # Only this collector can publish the immutable final XML after confirmed exit.
            $stage = Assert-Within (Join-Path $tempRoot "code0-native-loader-staging-$env:GITHUB_RUN_ID-$env:GITHUB_RUN_ATTEMPT") $tempRoot
            if ($null -ne (Get-Item -LiteralPath $stage -Force -ErrorAction SilentlyContinue)) { throw 'Manifest staging directory already exists; refusing reuse.' }
            [void][IO.Directory]::CreateDirectory($stage)
            $out = Assert-Within (Join-Path $stage 'fixture-manifest.bin') $stage
            $report.fixture.mtPublication = 'not_published'
            # One literal inputresource argument. Never -outputresource / resource injection.
            $result = Invoke-MetadataTool $mt @('-nologo', "-inputresource:$fixture;#1", "-out:$out") $out
            $report.operations += @{ tool = 'mt'; resource = '#1'; status = $result.status; exitCode = $result.exitCode; truncated = $result.truncated; termination = $result.termination; killAttempted = $result.killAttempted; waitConfirmed = $result.waitConfirmed; cleanupError = $result.cleanupError }
            Save-ToolResult 'fixture-mt.txt' $result
            $id1 = @($pe.manifests | Where-Object { $_.numericId -and $_.id -eq '1' })
            $report.fixture.manifestId1 = if ($id1.Count -eq 0) { 'absent_in_enumerated_resources' } else { 'present_in_enumerated_resources' }
            if ($result.status -eq 'captured' -and $result.termination -eq 'exit_confirmed') {
                if (-not (Test-Path -LiteralPath $out -PathType Leaf)) { throw 'Successful mt did not produce manifest text.' }
                $text = Read-SmallText (Assert-Within $out $stage) 64KB
                $settings = [Xml.XmlReaderSettings]::new()
                $settings.DtdProcessing = [Xml.DtdProcessing]::Prohibit
                $settings.XmlResolver = $null
                $settings.MaxCharactersInDocument = 64KB
                $stringReader = [IO.StringReader]::new($text)
                $xmlReader = $null
                try {
                    $xmlReader = [Xml.XmlReader]::Create($stringReader, $settings)
                    while ($xmlReader.Read()) { }
                } finally {
                    if ($null -ne $xmlReader) { $xmlReader.Dispose() }
                    $stringReader.Dispose()
                }
                if ($utf8.GetByteCount($text) -gt 64KB) { throw 'Decoded mt manifest exceeds publication byte limit.' }
                Save-Text 'fixture-mt-manifest.xml' $text
                $report.fixture.mtPublication = 'published_after_confirmed_exit_and_bounded_xml_validation'
            }
        } catch { $script:Problems.Add("PE/resource collection unavailable: $($_.Exception.GetType().Name)"); $report.fixture.resourceStatus = 'unavailable_or_limit' }
        if ($script:ToolTerminationUnknown) { throw 'Owned metadata tool termination_unknown; collection stopped after resource step.' }
        $adjacent = Assert-Within ($fixture + '.manifest') $deps
        $report.fixture.adjacentManifest = @{ present = (Test-Path -LiteralPath $adjacent -PathType Leaf) }
        if ($report.fixture.adjacentManifest.present) {
            try {
                $adjacentIdentity = Get-FileIdentity $adjacent 256KB
                $report.fixture.adjacentManifest.sha256 = $adjacentIdentity.sha256
                $report.fixture.adjacentManifest.bytes = $adjacentIdentity.bytes
                Save-Text 'fixture-adjacent.manifest.xml' (Read-SmallText $adjacent)
            }
            catch { $script:Problems.Add('Adjacent manifest unavailable or exceeds limit.') }
        }
        $symbols = @('DefSubclassProc', 'RemoveWindowSubclass', 'SetWindowSubclass')
        $retained = @($symbols | Where-Object { $imports -and $imports -cmatch ('\b' + $_ + '\b') })
        if ($retained.Count -gt 0 -and $imports -match '(?i)comctl32\.dll') {
            try {
                $systemDll = Assert-Within (Join-Path $env:SystemRoot 'System32\comctl32.dll') $env:SystemRoot
                $systemIdentity = Get-FileIdentity $systemDll 32MB
                $version = [Diagnostics.FileVersionInfo]::GetVersionInfo($systemDll)
                $result = Invoke-MetadataTool $dumpbin @('/nologo', '/exports', $systemDll)
                $report.operations += @{ tool = 'dumpbin'; mode = 'system32_comctl32_exports'; status = $result.status; exitCode = $result.exitCode; truncated = $result.truncated; termination = $result.termination; killAttempted = $result.killAttempted; waitConfirmed = $result.waitConfirmed; cleanupError = $result.cleanupError }
                $matching = @($result.stdout -split '\r?\n' | Where-Object { $_ -cmatch '\b(DefSubclassProc|RemoveWindowSubclass|SetWindowSubclass)\b' }) -join "`n"
                $result.stdout = $matching
                $report.system32Comparison = @{ identity = $systemIdentity; fileVersion = $version.FileVersion; productVersion = $version.ProductVersion; pe = (Get-PeMetadata $systemDll $false); candidateNames = $retained; status = $result.status; exitCode = $result.exitCode; termination = $result.termination; killAttempted = $result.killAttempted; waitConfirmed = $result.waitConfirmed; cleanupError = $result.cleanupError; warning = 'Base System32 image only. NOT actual fixture SxS resolution, loaded DLL identity, or missing-entrypoint proof; forwarded/API-set targets unresolved.' }
                Save-ToolResult 'system32-comctl32-subclass-exports.txt' $result
            } catch { $script:Problems.Add('System32 comparison unavailable or exceeds limit.') }
            if ($script:ToolTerminationUnknown) { throw 'Owned metadata tool termination_unknown; collection stopped after export step.' }
        } else { $report.system32Comparison = @{ status = 'not_requested'; reason = 'No complete captured imports retaining subclass candidate and comctl32 DLL name.' } }
    } else { $script:Problems.Add("Fixture $($report.fixture.status); no PE selected or inspected.") }
    try { $report.ownBuild = @(Collect-OwnBuild $package) } catch { $script:Problems.Add("Own build collection unavailable: $($_.Exception.GetType().Name)") }
    $report.problems = @($script:Problems.ToArray())
    $report.toolTerminationUnknown = $script:ToolTerminationUnknown
    $report.status = if ($script:Problems.Count -eq 0) { 'metadata_captured_no_runtime_acceptance' } else { 'incomplete_metadata_no_runtime_acceptance' }
    $report.packetFiles = @($script:Files.ToArray())
    Save-Text 'metadata.json' ($report | ConvertTo-Json -Depth 14)
    Write-Host "Code0 metadata-only packet: $script:OutputDirectory; $($report.status)"
    if ($script:Problems.Count -gt 0) { exit 1 }
} catch {
    # Do not touch a pre-existing/rejected output directory or any unrelated path.
    if ($script:CreatedOutputDirectory) {
        try {
            $report.status = 'incomplete_metadata_no_runtime_acceptance'
            $report.toolTerminationUnknown = $script:ToolTerminationUnknown
            $script:Problems.Add("Collection stopped: $($_.Exception.Message)")
            $report.problems = @($script:Problems.ToArray())
            $report.packetFiles = @($script:Files.ToArray())
            Save-Text 'metadata.json' ($report | ConvertTo-Json -Depth 14)
        } catch { Write-Host 'Metadata failure report could not be written within the packet limits.' }
    }
    Write-Error ("Code0 metadata collection refused/failed: " + $_.Exception.Message)
    exit 1
}
