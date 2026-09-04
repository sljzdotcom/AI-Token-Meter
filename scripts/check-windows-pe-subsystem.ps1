param(
    [Parameter(Mandatory = $true)]
    [string] $Path
)

$ErrorActionPreference = "Stop"
$resolvedPath = (Resolve-Path -LiteralPath $Path).Path
$bytes = [System.IO.File]::ReadAllBytes($resolvedPath)

if ($bytes.Length -lt 64 -or $bytes[0] -ne 0x4D -or $bytes[1] -ne 0x5A) {
    throw "Windows executable is missing a valid DOS header: $resolvedPath"
}

$peOffset = [System.BitConverter]::ToInt32($bytes, 0x3C)
if ($peOffset -lt 0 -or $peOffset -gt ($bytes.Length - 94)) {
    throw "Windows executable contains an invalid PE header offset: $resolvedPath"
}

if ($bytes[$peOffset] -ne 0x50 -or
    $bytes[$peOffset + 1] -ne 0x45 -or
    $bytes[$peOffset + 2] -ne 0x00 -or
    $bytes[$peOffset + 3] -ne 0x00) {
    throw "Windows executable is missing a valid PE signature: $resolvedPath"
}

$optionalHeaderSize = [System.BitConverter]::ToUInt16($bytes, $peOffset + 20)
$optionalHeaderOffset = $peOffset + 24
if ($optionalHeaderSize -lt 70 -or
    $optionalHeaderOffset -gt ($bytes.Length - $optionalHeaderSize)) {
    throw "Windows executable contains a truncated PE optional header: $resolvedPath"
}

$magic = [System.BitConverter]::ToUInt16($bytes, $optionalHeaderOffset)
if ($magic -ne 0x010B -and $magic -ne 0x020B) {
    throw "Windows executable uses an unsupported PE optional header: $resolvedPath"
}

$subsystem = [System.BitConverter]::ToUInt16($bytes, $optionalHeaderOffset + 68)
if ($subsystem -ne 2) {
    throw "AI Token Meter must use Windows GUI subsystem 2; actual subsystem is $subsystem in $resolvedPath"
}

Write-Host "Windows PE subsystem check passed (GUI = 2): $resolvedPath"
