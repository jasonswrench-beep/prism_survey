# Prism Hub listing (draft)

Prism Survey is listed in Prism Hub like its siblings. The listing lives in the hub's repository (`C:\Dev\OpenStage-Hub`, brand folder `brands/prism`), not here, and a hub installs only signed releases whose SHA-256 matches the signed catalog. This page holds the entry to add there once the first signed release exists; nothing in the hub has been changed.

## Entry for `brands/prism/apps.json`

```json
{
  "id": "prismsurvey",
  "name": "Prism Survey",
  "mark": {
    "letters": "ps",
    "color": "#285BFC",
    "icon": "icons/prismsurvey.svg"
  },
  "description": "Scales from published measures, ready for Word questionnaires",
  "repository": "jasonswrench-beep/prism_survey",
  "installedAs": "Prism Survey",
  "programFile": "prismsurvey-app.exe",
  "quietSwitch": ["/P"],
  "expectedSigner": "Jason Wrench",
  "setupPattern": "^Prism Survey_.*_x64-setup\\.exe$"
}
```

The icon is `assets/PrismSurvey-Icon.svg` from this repository, copied to `brands/prism/icons/prismsurvey.svg` in the hub.

## Before it can be listed

1. A signed Windows installer: the build script and signing follow PrismQDA's (`app\BUILD-INSTALLER.ps1 -Sign`, Azure Artifact Signing, signer Jason Wrench). The installer's name comes from `productName` in `app/src-tauri/tauri.conf.json` ("Prism Survey"), which gives `Prism Survey_0.1.0_x64-setup.exe`.
2. A GitHub release of `jasonswrench-beep/prism_survey` carrying the installer and its SHA-256 file.
3. The hub's catalog rebuilt and signed with the new entry (the hub's own process, in its repository).

Version 1 (0.1.0) ships after phase 7 under the revised plan, so this waits until then; the entry can be added earlier for a beta.
