# Changelog — Lightroom by Tony

All notable changes to this personal fork. The same notes are published on GitHub Releases and shown in the app when an update is available.

## 1.3.0

- **SYNCED WITH UPSTREAM LIGHTCRAFT v0.11.0**: the whole app moves up to upstream 0.11.0 — **LOSSY DNG SUPPORT** (lossy DNG files open natively, including JPEG XL-compressed raws), **SAM 3 WITH CPU FALLBACK** (AI masking works even without a GPU), **DETAIL FIT** rendering, two new interface languages (**Czech** and **Simplified Chinese**), and upstream's fixes and polish since 0.10.
- **TRACKPAD PINCH IS NOW UPSTREAM'S**: pinch-to-zoom uses upstream's new native implementation — the fork's own pinch handler is retired, one less patch to carry across future syncs.
- **HELP MENU STAYS THE FORK'S FOUR**: About, Check for Updates, System Info, Keyboard Shortcuts — the merge briefly brought back extra upstream items; they are gone again.

— **tonynehihi**

## 1.2.0

- **NEW NAME — "LrT"**: the app is now called **LrT** in Spotlight, Finder and the Dock — the same short-code style as the rest of the suite (PsT, AiT, PrT, AeT, IdT, AcT) — and the icon keeps its **LrT** artwork. Inside the app the full name is **Lightroom by Tonynehihi**. The GitHub repo moved to **tonynehihi/lr-by-tonynehihi** — older versions still find updates through GitHub's redirect. Your library, settings and photos are untouched (same bundle id, same library folder). **One manual install**: the old updater looks for the old bundle name, so this version must be installed by hand once (zip + build, like the early releases); automatic updates work as usual from the next version on.
- **CHANGELOG RENDERS PROPERLY IN THE APP**: release notes in the update prompt and About now show **bold text as bold**, like on GitHub — no more literal `**` markers.

— **tonynehihi**

## 1.1.9

- **COPY SETTINGS NOW GREYS OUT**: when several selected photos have **different edit settings**, the Copy Settings button is **greyed out and unclickable** — hover it for the reason — instead of failing with a message after the click. Select photos that share the same settings, or a single photo, and it lights up again.
- **LOCATE IN FOLDER**: the missing-photo prompt now offers **two separate ways** to relink. **Locate File...** picks the exact file (still verified by content, so a same-name look-alike is refused). **Locate in Folder...** takes just a **folder** and **searches it recursively**, matching by **content** — the photo is found and relinked automatically **even if the file was renamed**.

— **tonynehihi**

## 1.1.8

- **UPDATE PROGRESS, LIVE IN THE APP**: installing an update no longer goes silent in the background. The new version now **BUILDS INSIDE THE APP** with live progress — current step, crates compiled, elapsed time, plus a **Cancel Build** button — while your current version stays open and untouched. The app closes **only for a few seconds** at the end to swap versions, then reopens by itself; during the swap the old version is moved out of Applications first, so it **can't be launched mid-update** from Spotlight. If the build fails, nothing was changed and you keep using the current version. (Installing this version itself still uses the previous flow — the new flow applies from the next update onward.)

— **tonynehihi**

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
