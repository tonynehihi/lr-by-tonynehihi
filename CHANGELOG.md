# Changelog — Lightroom by Tony

All notable changes to this personal fork. The same notes are published on GitHub Releases and shown in the app when an update is available.

## 1.1.7

- **HELP MENU, TRIMMED**: every item that linked to the upstream LightCraft/ArtCraft project is **GONE** — the Help docs, ArtCraft Discord, Send Feedback, both Website links and the upstream GitHub link; the old **What's New** (upstream release notes) is gone too. The Help menu now holds only **About**, **Check for Updates**, **System Info** and **Keyboard Shortcuts**. This build's own release notes live in the update prompt and on GitHub Releases, as before.

— **tonynehihi**

## 1.1.6

- **MACOS-STYLE ICON**: the LrT icon now uses the standard macOS squircle shape with proper padding — no more full-bleed square on the Dock.
- **HEIC SUPPORT (MAC ONLY)**: import, view and edit .heic photos from iPhone like any other photo, using the built-in macOS HEIC decoder. Not available on Windows yet.
- **MISSING FILE POP-UP**: opening a photo whose original file was moved or deleted shows a pop-up — **Locate File...** to relink it (verified by content, so a different photo with the same name is rejected), **Remove from Library**, or **Not Now**.
- **NO MORE COPY SETTINGS MIX-UPS**: copying is refused when the selected photos have different edit settings. A single photo, or photos with identical settings, copy as before.
- **UPDATE PROMPT ON LAUNCH**: if a new version is available, the app asks on startup and shows the release notes before you decide. Nothing pops up when you're up to date; **Later** asks again next launch.
- From this version on, **EVERY RELEASE SHIPS WITH A CHANGELOG** on GitHub and in the app.

— **tonynehihi**

## 1.1.5

- **IN-APP UPDATES**: Help → Check for Updates (or About → Updates). Download Update → Quit & Install — the app builds and replaces itself on macOS.

## 1.1.4

- Multi-select in the filmstrip (tick circles, ⌘+click, Shift+click, All/None buttons); separate Copy Settings / Paste Settings buttons; Paste applies to all selected photos; export uses the selection.
