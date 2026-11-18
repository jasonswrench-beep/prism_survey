# Makes the key that signs Prism Survey's updates (the Help tab's Check for Updates; docs\UPDATES.md). Run it once,
# yourself: it asks for a password (twice) that you choose. MAKE PRISM SURVEY UPDATE KEY.bat runs it by double-click.
#
# What it makes: a private key file and its public half in your profile folder, outside the repository
# (.prismsurvey-update.key and .prismsurvey-update.key.pub, as the sibling Prism tools keep theirs). The public half is put
# into app\src-tauri\src\updates.rs (PUBKEY), so the program built from then on trusts only files signed with this key.
# Check the change with `git diff`, and commit it.
#
# THE PRIVATE KEY AND ITS PASSWORD ARE THE ONLY WAY TO SIGN AN UPDATE. Keep the file out of the repository and out of any
# sync folder others can read, and back it up (a second copy on another drive or in a password manager). If the key or its
# password is lost, installed copies can never be updated again (a new installer would have to be installed by hand once,
# carrying a new key). If someone else gets both, they could sign an update that people's copies would accept: make a new
# key at once and publish an installer built with it. This script never prints the private key.
#
# Parameters, for testing only: -KeyFile (where the key goes), -NoSource (leave updates.rs alone), -Password (skip the
# question; a password on a command line can be read by other programs, so never use it for the real key).

param([string]$KeyFile = (Join-Path $env:USERPROFILE '.prismsurvey-update.key'), [switch]$NoSource, [string]$Password = $null)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$tauriCli = Join-Path $root 'app\ui\node_modules\.bin\tauri.cmd'
if (-not (Test-Path $tauriCli)) { throw 'Tauri''s command line is not installed: run npm ci in app\ui first' }
$key = $KeyFile
$Folder = Split-Path -Parent $key
if (Test-Path $key) { throw "There is already a key at $key. It is not replaced (replacing it would stop every installed copy from updating). Move it away yourself if you mean to." }
if ($key -like "$root*") { throw 'The key must not be inside the repository' }
New-Item -ItemType Directory -Force $Folder | Out-Null

if ($Password) {
    & $tauriCli signer generate --ci -w $key -p $Password
} else {
    & $tauriCli signer generate -w $key
}
if ($LASTEXITCODE -or -not (Test-Path "$key.pub")) { throw 'the key was not made' }

$public = (Get-Content "$key.pub" -Raw).Trim()
"The public key (safe to publish):"
$public
""
if ($NoSource) {
    "Not put into updates.rs (-NoSource)."
} else {
    $path = Join-Path $root 'app\src-tauri\src\updates.rs'
    $text = [System.IO.File]::ReadAllText($path)
    $pattern = 'pub const PUBKEY: &str = "[^"]*";'
    if ($text -notmatch $pattern) { throw 'PUBKEY was not found in updates.rs' }
    $replaced = [regex]::Replace($text, $pattern, ('pub const PUBKEY: &str = "' + $public + '";'))
    [System.IO.File]::WriteAllText($path, $replaced, (New-Object System.Text.UTF8Encoding $false))
    "Put into app\src-tauri\src\updates.rs. Review it with git diff, then commit it."
}
""
"Now: BACK UP $key and $key.pub, offline, with the password."
"Release builds then sign updates with it: BUILD SIGNED PRISM SURVEY RELEASE.bat (docs\UPDATES.md)."
