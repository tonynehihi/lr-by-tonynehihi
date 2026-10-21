# Changelog — Lightroom by Tony

All notable changes to this personal fork. The same notes are published on GitHub Releases and shown in the app when an update is available.

## 1.1.6

- **macOS-style icon**: the LrT icon now uses the standard macOS squircle shape with proper padding — no more full-bleed square on the Dock.
- **HEIC support (macOS)**: import, view and edit .heic photos from iPhone like any other photo, using the built-in macOS HEIC decoder. (Not on Windows yet.)
- **Missing file pop-up**: opening a photo whose original file was moved or deleted shows a pop-up — **Locate File...** to relink it (the file is verified by content, so a different photo with the same name is rejected), **Remove from Library**, or **Not Now**.
- **No more Copy Settings mix-ups**: copying is refused when the selected photos have different edit settings. A single photo, or photos with identical settings, copy as before.
- **Update prompt on launch**: if a new version is available, the app asks on startup and shows the release notes before you decide. Nothing pops up when you're up to date; **Later** asks again next launch.
- From this version on, every release ships with a changelog on GitHub and in the app.

## 1.1.5

- **In-app updates**: Help → Check for Updates (or About → Updates). Download Update → Quit & Install — the app builds and replaces itself on macOS.

## 1.1.4

- Multi-select in the filmstrip (tick circles, ⌘+click, Shift+click, All/None buttons); separate Copy Settings / Paste Settings buttons; Paste applies to all selected photos; export uses the selection.
