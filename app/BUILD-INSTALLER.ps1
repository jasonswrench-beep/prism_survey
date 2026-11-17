# Builds the interface, the desktop app and the Windows installer, then writes the installer's SHA-256 beside it.
#
#   powershell -ExecutionPolicy Bypass -File app\BUILD-INSTALLER.ps1          an unsigned installer
#   powershell -ExecutionPolicy Bypass -File app\BUILD-INSTALLER.ps1 -Sign    a signed and timestamped installer
#
# BUILD PRISM SURVEY INSTALLER.bat in the project folder runs the first form.
#
# Needs Rust, Node.js and, on the first run, a network connection: npm packages, Rust crates and Tauri's installer
# tools (NSIS) are downloaded once. Like the sibling Prism tools, the build runs on CPUs 16 to 31, the efficiency cores,
# because the performance cores of the author's computer are unstable under long builds.
# The result is app\src-tauri\target\release\bundle\nsis\Prism Survey_<version>_x64-setup.exe with a .sha256.txt beside it,
# and a .source.txt naming the tag and commit it was built from. It builds only from a tagged commit with nothing
# uncommitted.
#
# -Sign uses the suite's signing scripts rather than a copy of them, so one set of signing settings signs every program.
# OPENSTAGE_SIGN_TOOLS names the folder that holds them (sign_windows.py and check_signatures.py, as in the PrismQDA and
# OpenStage repositories); their own header says which settings and sign-in they need. Tauri is given sign_windows.py as
# its signCommand through a temporary configuration beside the build, so no signing setting goes into tauri.conf.json or
# the repository. check_signatures.py then checks the setup file and every program inside it before the SHA-256 is
# written, so the hash is the hash of the signed file. The build stops if anything is unsigned or has no timestamp.

param([switch]$Sign)

$ErrorActionPreference = 'Stop'
if ([Environment]::ProcessorCount -ge 32) {
    [System.Diagnostics.Process]::GetCurrentProcess().ProcessorAffinity = [IntPtr]4294901760
}

$app = Split-Path -Parent $MyInvocation.MyCommand.Path
$root = Split-Path -Parent $app

# Every installer is built from a tagged commit with nothing left uncommitted, so a release tag points at the exact
# commit its build came from (CLAUDE.md). The build stops otherwise.
Push-Location $root
$pending = git status --porcelain
if ($LASTEXITCODE) { Pop-Location; throw 'git is needed to check what the build is made from' }
if ($pending) { Pop-Location; throw "There are changes not committed. Commit everything and tag the commit before building:`n$($pending -join "`n")" }
$tag = git describe --exact-match --tags HEAD 2>$null
if ($LASTEXITCODE -or -not $tag) { Pop-Location; throw 'This commit has no tag. Tag it (for example v0.1.0) before building, so the release tag points at this build.' }
$commit = git rev-parse HEAD
Pop-Location
"Building from tag $tag (commit $commit)"

# Leaves the builder's folders out of the program. Rust writes the place of every error message into the program as a
# full path, so a build would carry the Windows account's folder name and this project's folder. The two prefixes are
# replaced by short stand-ins. (The flags go in CARGO_ENCODED_RUSTFLAGS, separated by the character 31, in case a folder
# has spaces in its name.)
$remap = @("--remap-path-prefix=$env:USERPROFILE=/user", "--remap-path-prefix=$root=/prismsurvey")
$env:CARGO_ENCODED_RUSTFLAGS = (@($env:CARGO_ENCODED_RUSTFLAGS) + $remap | Where-Object { $_ }) -join [string][char]31

$signConfig = $null
$dryRun = ($env:OPENSTAGE_SIGN_DRY_RUN -eq '1')
if ($Sign) {
    $tools = $env:OPENSTAGE_SIGN_TOOLS
    if (-not $tools -or -not (Test-Path (Join-Path $tools 'sign_windows.py'))) {
        throw 'OPENSTAGE_SIGN_TOOLS must name the folder that holds the suite''s sign_windows.py and check_signatures.py'
    }
    $python = (Get-Command python -ErrorAction SilentlyContinue).Source
    if (-not $python) { throw 'python was not found, and signing needs it' }
    if (-not $env:OPENSTAGE_SIGN_PUBLISHER) { throw 'OPENSTAGE_SIGN_PUBLISHER is not set: it names the signer the build must see on every file' }
    if (-not $dryRun) {
        & $python (Join-Path $tools 'sign_windows.py') --ready
        if ($LASTEXITCODE) { throw 'signing cannot start: see what is missing above' }
    }
    $signDir = Join-Path $app 'src-tauri\target\signing'
    New-Item -ItemType Directory -Force $signDir | Out-Null
    $signConfig = Join-Path $signDir 'tauri.sign.conf.json'
    @{ bundle = @{ windows = @{ signCommand = @{ cmd = $python; args = @((Join-Path $tools 'sign_windows.py'), '--description', 'Prism Survey', '%1') } } } } |
        ConvertTo-Json -Depth 8 | Out-File -Encoding ascii $signConfig
}

# The Comm Add On's data ships inside the installer; it must be there and readable.
$directory = Join-Path $root 'addons\comm-directory\directory.json'
if (-not (Test-Path $directory)) { throw 'addons\comm-directory\directory.json is missing (prismsurvey addon-build makes it)' }
$null = Get-Content $directory -Raw -Encoding UTF8 | ConvertFrom-Json

Set-Location (Join-Path $app 'ui')
if (-not (Test-Path 'node_modules')) { npm ci --no-audit --no-fund; if ($LASTEXITCODE) { throw 'npm ci failed' } }
npm run build
if ($LASTEXITCODE) { throw 'the interface did not build' }

Set-Location (Join-Path $app 'src-tauri')
if ($Sign) {
    & '..\ui\node_modules\.bin\tauri.cmd' build --ci --config $signConfig
} else {
    & '..\ui\node_modules\.bin\tauri.cmd' build --ci
}
if ($LASTEXITCODE) { throw 'the app did not build' }

$installer = Get-ChildItem 'target\release\bundle\nsis' -Filter '*-setup.exe' | Sort-Object LastWriteTime -Descending | Select-Object -First 1
if ($Sign -and -not $dryRun) {
    & $python (Join-Path $env:OPENSTAGE_SIGN_TOOLS 'check_signatures.py') $installer.FullName
    if ($LASTEXITCODE) { throw 'the setup was built, but it is not signed as it should be (see above)' }
}
$hash = (Get-FileHash $installer.FullName -Algorithm SHA256).Hash.ToLower()
"$hash  $($installer.Name)" | Out-File -Encoding ascii "$($installer.FullName).sha256.txt"
"Built from tag $tag, commit $commit" | Out-File -Encoding ascii "$($installer.FullName).source.txt"
"Built $($installer.FullName) from tag $tag"
"SHA-256 $hash"
