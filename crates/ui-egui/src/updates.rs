//! Update check for this personal build ("Lightroom by Tony"): the About dialog and
//! Help ▸ Check for Updates ask GitHub for the newest release of the fork's repo
//! (`tonynehihi/lightroom-by-tony`, public — no sign-in needed). On macOS the dialog can
//! also download the release's source archive and hand it to a small shell updater
//! (embedded below) that waits for the app to quit, rebuilds from that source with
//! `packaging/macos/package.sh`, replaces the app in /Applications and relaunches it.
//! The network work runs on a worker thread; the UI only polls messages, so the UI
//! thread never waits on the network. Failures land in [`Phase::Failed`] as readable
//! text — nothing here panics.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, TryRecvError};
use std::sync::Arc;
use std::time::Duration;

use crate::theme::Tokens;
use crate::LightcraftApp;

/// GitHub API: the newest published release of the fork.
pub const LATEST_RELEASE_URL: &str = "https://api.github.com/repos/tonynehihi/lightroom-by-tony/releases/latest";

/// Largest release JSON accepted (release notes included).
const MAX_JSON_BYTES: u64 = 1 << 20;
/// Largest source archive accepted (the repo snapshot is ~15 MB today).
const MAX_ZIP_BYTES: u64 = 300 << 20;

/// One published release, as the dialog shows it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReleaseInfo {
    /// Version without the tag's leading "v" (e.g. "1.1.6").
    pub version: String,
    pub name: String,
    pub notes: String,
    /// Release page (opened in the browser on platforms without auto-install).
    pub page_url: String,
    /// Source archive of the release tag (zipball).
    pub zip_url: String,
}

/// Where the update flow stands.
#[derive(Clone, Debug, PartialEq)]
pub enum Phase {
    Idle,
    Checking,
    UpToDate,
    Available(ReleaseInfo),
    Downloading { info: ReleaseInfo, received: u64, total: Option<u64> },
    Downloaded { info: ReleaseInfo, zip: PathBuf },
    Failed(String),
}

enum JobMsg {
    Check(Result<Option<ReleaseInfo>, String>),
    Progress(u64, Option<u64>),
    Download(Result<PathBuf, String>),
}

/// Update state owned by the app; jobs report back over a channel drained by [`UpdateState::poll`].
pub struct UpdateState {
    pub phase: Phase,
    rx: Option<Receiver<JobMsg>>,
    cancel: Arc<AtomicBool>,
}

impl Default for UpdateState {
    fn default() -> Self {
        UpdateState { phase: Phase::Idle, rx: None, cancel: Arc::new(AtomicBool::new(false)) }
    }
}

impl UpdateState {
    fn busy(&self) -> bool {
        self.rx.is_some()
    }

    /// Ask GitHub for the newest release (no-op while another job runs).
    pub fn start_check(&mut self) {
        if self.busy() {
            return;
        }
        self.cancel.store(false, Ordering::SeqCst);
        let (tx, rx) = std::sync::mpsc::channel();
        self.rx = Some(rx);
        self.phase = Phase::Checking;
        let cancel = self.cancel.clone();
        std::thread::spawn(move || {
            let _ = tx.send(JobMsg::Check(check_latest(&cancel)));
        });
    }

    /// Download the source archive of the release in [`Phase::Available`] (macOS auto-install path).
    pub fn start_download(&mut self) {
        if self.busy() {
            return;
        }
        let Phase::Available(info) = self.phase.clone() else { return };
        let Some(dir) = update_dir() else {
            self.phase = Phase::Failed("Could not find your home folder to hold the download.".to_string());
            return;
        };
        if let Err(e) = std::fs::create_dir_all(&dir) {
            self.phase = Phase::Failed(format!("Could not create the update folder: {e}"));
            return;
        }
        let dest = dir.join(format!("lightroom-by-tony-{}-source.zip", info.version));
        self.cancel.store(false, Ordering::SeqCst);
        let (tx, rx) = std::sync::mpsc::channel();
        self.rx = Some(rx);
        self.phase = Phase::Downloading { info: info.clone(), received: 0, total: None };
        let cancel = self.cancel.clone();
        let url = info.zip_url.clone();
        std::thread::spawn(move || {
            let _ = tx.send(JobMsg::Download(download_zip(&url, &dest, &cancel, &tx).map(|()| dest)));
        });
    }

    /// Drain job messages into the phase. Returns true while a job is still running.
    pub fn poll(&mut self) -> bool {
        let mut finished = false;
        if let Some(rx) = &self.rx {
            loop {
                match rx.try_recv() {
                    Ok(JobMsg::Check(res)) => {
                        self.phase = match res {
                            Ok(Some(info)) => Phase::Available(info),
                            Ok(None) => Phase::UpToDate,
                            Err(e) => Phase::Failed(e),
                        };
                        finished = true;
                    }
                    Ok(JobMsg::Progress(received, total)) => {
                        if let Phase::Downloading { received: r, total: t, .. } = &mut self.phase {
                            *r = received;
                            *t = total;
                        }
                    }
                    Ok(JobMsg::Download(res)) => {
                        let info = match &self.phase {
                            Phase::Downloading { info, .. } => Some(info.clone()),
                            _ => None,
                        };
                        self.phase = match (res, info) {
                            (Ok(zip), Some(info)) => Phase::Downloaded { info, zip },
                            (Err(e), _) => Phase::Failed(e),
                            (Ok(_), None) => Phase::Failed("The download finished in an unexpected state.".to_string()),
                        };
                        finished = true;
                    }
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => {
                        self.phase = match &self.phase {
                            Phase::Checking => Phase::Failed("The update check stopped unexpectedly.".to_string()),
                            Phase::Downloading { .. } => Phase::Failed("The download stopped unexpectedly.".to_string()),
                            other => other.clone(),
                        };
                        finished = true;
                        break;
                    }
                }
            }
        }
        if finished {
            self.rx = None;
        }
        self.busy()
    }

    /// Write the shell updater next to the downloaded archive and launch it detached;
    /// it waits for this app to exit, then builds and installs. macOS only.
    pub fn install(&self) -> Result<(), String> {
        let Phase::Downloaded { info, zip } = &self.phase else {
            return Err("There is no downloaded update to install.".to_string());
        };
        if !cfg!(target_os = "macos") {
            return Err("Automatic install is only available on macOS.".to_string());
        }
        let dir = update_dir().ok_or_else(|| "Could not find your home folder.".to_string())?;
        std::fs::create_dir_all(&dir).map_err(|e| format!("Could not create the update folder: {e}"))?;
        let script = dir.join("install-update.sh");
        std::fs::write(&script, UPDATER_SCRIPT).map_err(|e| format!("Could not write the updater script: {e}"))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755))
                .map_err(|e| format!("Could not make the updater script runnable: {e}"))?;
        }
        spawn_updater(&script, zip, &info.version)
    }
}

/// Launch the updater detached from this app (it must survive the app quitting).
#[cfg(unix)]
fn spawn_updater(script: &std::path::Path, zip: &std::path::Path, version: &str) -> Result<(), String> {
    use std::os::unix::process::CommandExt;
    std::process::Command::new("/bin/sh")
        .arg(script)
        .arg(zip)
        .arg(version)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .process_group(0)
        .spawn()
        .map_err(|e| format!("Could not start the updater: {e}"))?;
    Ok(())
}

/// Non-Unix builds never reach here (install stops earlier); this only satisfies the compiler.
#[cfg(not(unix))]
fn spawn_updater(_script: &std::path::Path, _zip: &std::path::Path, _version: &str) -> Result<(), String> {
    Err("Automatic install is only available on macOS.".to_string())
}

/// Folder holding downloaded update archives and the updater script:
/// `~/Library/Caches/Lightroom by Tony/updates` on macOS, `~/.cache/...` elsewhere.
fn update_dir() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    let mut p = PathBuf::from(home);
    if cfg!(target_os = "macos") {
        p.push("Library");
        p.push("Caches");
    } else {
        p.push(".cache");
    }
    p.push("Lightroom by Tony");
    p.push("updates");
    Some(p)
}

/// Split a version into numeric parts ("1.10.2" → [1, 10, 2]); non-numeric tails count as 0.
pub fn version_parts(version: &str) -> Vec<u64> {
    version
        .split('.')
        .map(|part| {
            let digits: String = part.chars().take_while(|c| c.is_ascii_digit()).collect();
            digits.parse().unwrap_or(0)
        })
        .collect()
}

/// True when `remote` is a strictly newer version than `current`.
pub fn is_newer(remote: &str, current: &str) -> bool {
    let (r, c) = (version_parts(remote), version_parts(current));
    for i in 0..r.len().max(c.len()) {
        let (a, b) = (r.get(i).copied().unwrap_or(0), c.get(i).copied().unwrap_or(0));
        if a != b {
            return a > b;
        }
    }
    false
}

/// Parse the GitHub "latest release" JSON into a [`ReleaseInfo`].
pub fn parse_release(json_text: &str) -> Result<ReleaseInfo, String> {
    let v: serde_json::Value = serde_json::from_str(json_text).map_err(|e| format!("GitHub sent release data that could not be read: {e}"))?;
    let field = |key: &str| v.get(key).and_then(serde_json::Value::as_str);
    let tag = field("tag_name").ok_or_else(|| "GitHub's release data has no version tag.".to_string())?;
    let zip_url = field("zipball_url").ok_or_else(|| "GitHub's release data has no source archive.".to_string())?;
    Ok(ReleaseInfo {
        version: tag.trim_start_matches('v').to_string(),
        name: field("name").unwrap_or(tag).to_string(),
        notes: field("body").unwrap_or_default().to_string(),
        page_url: field("html_url").unwrap_or_default().to_string(),
        zip_url: zip_url.to_string(),
    })
}

#[cfg(not(target_arch = "wasm32"))]
fn limits(cancel: &AtomicBool) -> lightcraft_fetch::http::Limits<'_> {
    lightcraft_fetch::http::Limits { connect: Duration::from_secs(10), stall: Duration::from_secs(20), cancel }
}

#[cfg(not(target_arch = "wasm32"))]
fn read_body(resp: &mut lightcraft_fetch::http::Response, cancel: &AtomicBool, cap: u64) -> Result<Vec<u8>, String> {
    let limits = limits(cancel);
    let mut out = Vec::new();
    let mut buf = [0u8; 16 * 1024];
    loop {
        let n = resp.read(&mut buf, &limits).map_err(|e| format!("Could not read GitHub's reply: {e}"))?;
        if n == 0 {
            return Ok(out);
        }
        out.extend_from_slice(buf.get(..n).unwrap_or_default());
        if out.len() as u64 > cap {
            return Err("GitHub's reply was larger than expected.".to_string());
        }
    }
}

/// Fetch and parse the newest release; `Ok(None)` when it is not newer than this build.
#[cfg(not(target_arch = "wasm32"))]
fn check_latest(cancel: &AtomicBool) -> Result<Option<ReleaseInfo>, String> {
    let url = lightcraft_fetch::http::Url::parse(LATEST_RELEASE_URL).map_err(|e| format!("Bad update URL: {e}"))?;
    let limits = limits(cancel);
    let headers = [("Accept", "application/vnd.github+json".to_string())];
    let mut resp = lightcraft_fetch::http::get(&url, &headers, &limits).map_err(|e| format!("Could not reach GitHub: {e}"))?;
    if resp.status == 404 {
        return Err("No published release was found on GitHub yet.".to_string());
    }
    if resp.status != 200 {
        return Err(format!("GitHub replied with status {}.", resp.status));
    }
    let body = read_body(&mut resp, cancel, MAX_JSON_BYTES)?;
    let text = String::from_utf8_lossy(&body);
    let info = parse_release(&text)?;
    Ok(if is_newer(&info.version, env!("CARGO_PKG_VERSION")) { Some(info) } else { None })
}

#[cfg(target_arch = "wasm32")]
fn check_latest(_cancel: &AtomicBool) -> Result<Option<ReleaseInfo>, String> {
    Err("Update checks are not available in the web build.".to_string())
}

/// Download `url` (a GitHub zipball: redirects to the file host) into `dest`,
/// reporting progress on `tx`. Written to a `.part` file and renamed when complete.
#[cfg(not(target_arch = "wasm32"))]
fn download_zip(url_str: &str, dest: &std::path::Path, cancel: &AtomicBool, tx: &Sender<JobMsg>) -> Result<(), String> {
    use std::io::Write;
    let limits = limits(cancel);
    let mut url = lightcraft_fetch::http::Url::parse(url_str).map_err(|e| format!("Bad download URL: {e}"))?;
    let mut part = dest.as_os_str().to_owned();
    part.push(".part");
    let part = PathBuf::from(part);
    let mut hops = 0u32;
    loop {
        let mut resp = lightcraft_fetch::http::get(&url, &[], &limits).map_err(|e| format!("Could not reach the download host: {e}"))?;
        match resp.status {
            200 => {
                let total = resp.content_length();
                let mut file = std::fs::File::create(&part).map_err(|e| format!("Could not create the download file: {e}"))?;
                let mut buf = [0u8; 64 * 1024];
                let mut received: u64 = 0;
                let mut last_sent: u64 = 0;
                let outcome = loop {
                    match resp.read(&mut buf, &limits) {
                        Ok(0) => break Ok(()),
                        Ok(n) => {
                            if let Err(e) = file.write_all(buf.get(..n).unwrap_or_default()) {
                                break Err(format!("Could not write the download: {e}"));
                            }
                            received = received.saturating_add(n as u64);
                            if received > MAX_ZIP_BYTES {
                                break Err("The update archive was larger than expected.".to_string());
                            }
                            if received - last_sent >= 256 * 1024 {
                                last_sent = received;
                                let _ = tx.send(JobMsg::Progress(received, total));
                            }
                        }
                        Err(e) => break Err(format!("The download was interrupted: {e}")),
                    }
                };
                if let Err(e) = outcome {
                    let _ = std::fs::remove_file(&part);
                    return Err(e);
                }
                let _ = tx.send(JobMsg::Progress(received, total));
                return std::fs::rename(&part, dest).map_err(|e| format!("Could not finish the download file: {e}"));
            }
            301 | 302 | 303 | 307 | 308 => {
                hops += 1;
                if hops > 5 {
                    return Err("The download bounced through too many redirects.".to_string());
                }
                let location = resp.header("location").ok_or_else(|| "The download redirect had no destination.".to_string())?.to_string();
                url = url.join(&location).map_err(|e| format!("Bad download redirect: {e}"))?;
            }
            status => return Err(format!("The download host replied with status {status}.")),
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn download_zip(_url: &str, _dest: &std::path::Path, _cancel: &AtomicBool, _tx: &Sender<JobMsg>) -> Result<(), String> {
    Err("Update downloads are not available in the web build.".to_string())
}
/// Shell updater launched detached by [`UpdateState::install`]: waits for the app to
/// quit, rebuilds from the downloaded source, replaces /Applications and relaunches.
/// Every step logs to ~/Library/Logs/LightroomByTony-update.log; a failure before the
/// replace step leaves the installed app untouched and reopens it.
const UPDATER_SCRIPT: &str = r#"#!/bin/sh
ZIP="$1"
VER="${2:-unknown}"
DEST="/Applications/Lightroom by Tony.app"
LOG_DIR="$HOME/Library/Logs"
LOG="$LOG_DIR/LightroomByTony-update.log"
mkdir -p "$LOG_DIR"
exec >>"$LOG" 2>&1
echo ""
echo "=== Installing Lightroom by Tony $VER - $(date) ==="
fail() {
  echo "FAILED: $1"
  open "$DEST" >/dev/null 2>&1 || true
  exit 1
}
i=0
while pgrep -f "Lightroom by Tony.app/Contents/MacOS" >/dev/null 2>&1; do
  i=$((i+1))
  if [ "$i" -gt 120 ]; then
    echo "The app was still running after 120s - update aborted, nothing changed."
    exit 0
  fi
  sleep 1
done
WORK="$(mktemp -d /tmp/lbt-update.XXXXXX)" || fail "mktemp"
mkdir -p "$WORK/src"
ditto -x -k "$ZIP" "$WORK/src" || fail "extract the archive"
SRC=""
for d in "$WORK/src"/*/; do
  if [ -f "${d}Cargo.toml" ]; then SRC="${d%/}"; break; fi
done
[ -n "$SRC" ] || fail "source folder not found in the archive"
export PATH="$HOME/.cargo/bin:/opt/homebrew/bin:/usr/local/bin:$PATH"
cd "$SRC" || fail "enter the source folder"
packaging/macos/package.sh --arch aarch64 || fail "build and package"
BUILT="$SRC/target/macos-package/Lightroom by Tony.app"
[ -d "$BUILT" ] || fail "the packaged app was not produced"
rm -rf "$DEST" || fail "remove the old app"
ditto "$BUILT" "$DEST" || fail "install the new app"
echo "=== Update to $VER installed ==="
open "$DEST" || fail "relaunch the app"
"#;

/// The Updates section at the bottom of the About dialog.
pub fn about_section(app: &mut LightcraftApp, ui: &mut egui::Ui) {
    let busy = app.updates.poll();
    if busy {
        ui.ctx().request_repaint_after(Duration::from_millis(200));
    }
    let t = Tokens::get(ui.ctx());
    ui.label(egui::RichText::new("Updates").font(t.semibold(15.0)).color(t.text));
    ui.add_space(2.0);
    let phase = app.updates.phase.clone();
    match &phase {
        Phase::Idle => {
            if crate::widgets::text_button(ui, "updateCheck", "Check for Updates", false).clicked() {
                app.updates.start_check();
            }
        }
        Phase::Checking => {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label("Checking for updates...");
            });
        }
        Phase::UpToDate => {
            ui.label(egui::RichText::new("You have the latest version.").color(t.text_dim));
            if crate::widgets::text_button(ui, "updateCheckAgain", "Check Again", false).clicked() {
                app.updates.start_check();
            }
        }
        Phase::Available(info) => {
            ui.label(format!("Version {} is available - you have {}.", info.version, env!("CARGO_PKG_VERSION")));
            if !info.notes.trim().is_empty() {
                egui::ScrollArea::vertical().id_salt("update-notes").max_height(90.0).show(ui, |ui| {
                    ui.label(egui::RichText::new(info.notes.trim()).color(t.text_dim));
                });
            }
            ui.horizontal(|ui| {
                if cfg!(target_os = "macos") {
                    if crate::widgets::text_button(ui, "updateDownload", "Download Update", false).clicked() {
                        app.updates.start_download();
                    }
                } else if crate::widgets::text_button(ui, "updateOpenPage", "Open Release Page", false).clicked() {
                    if !info.page_url.is_empty() {
                        let url = info.page_url.clone();
                        let _ = crate::links::open(app, &url);
                    }
                }
                if crate::widgets::text_button(ui, "updateCheckAgain", "Check Again", false).clicked() {
                    app.updates.start_check();
                }
            });
        }
        Phase::Downloading { received, total, .. } => {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(format!("Downloading update... {:.1} MB", *received as f64 / 1_000_000.0));
            });
            if let Some(total) = total {
                if *total > 0 {
                    let frac = (*received as f64 / *total as f64).clamp(0.0, 1.0) as f32;
                    ui.add(egui::ProgressBar::new(frac).desired_width(220.0));
                }
            }
        }
        Phase::Downloaded { info, .. } => {
            ui.label(format!("Update {} is downloaded.", info.version));
            ui.label(egui::RichText::new("Installing quits the app, rebuilds (a few minutes) and reopens it.").color(t.text_dim));
            ui.horizontal(|ui| {
                if crate::widgets::text_button(ui, "updateInstall", "Quit & Install", false).clicked() {
                    match app.updates.install() {
                        Ok(()) => ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close),
                        Err(e) => app.updates.phase = Phase::Failed(e),
                    }
                }
                if crate::widgets::text_button(ui, "updateLater", "Later", false).clicked() {
                    app.updates.phase = Phase::Available(info.clone());
                }
            });
        }
        Phase::Failed(msg) => {
            ui.label(egui::RichText::new(msg.as_str()).color(t.text_dim));
            if crate::widgets::text_button(ui, "updateRetry", "Try Again", false).clicked() {
                app.updates.start_check();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn newer_versions_compare_by_numbers() {
        assert!(is_newer("1.1.6", "1.1.5"));
        assert!(is_newer("1.2.0", "1.1.9"));
        assert!(is_newer("2.0", "1.9.9"));
        assert!(is_newer("1.1.5.1", "1.1.5"));
        assert!(!is_newer("1.1.5", "1.1.5"));
        assert!(!is_newer("1.1.4", "1.1.5"));
        assert!(!is_newer("1.1.5", "1.1.5.1"));
        assert!(!is_newer("1.9.9", "2.0"));
    }

    #[test]
    fn release_json_parses_and_strips_the_v() {
        let json = r#"{
            "tag_name": "v1.2.3",
            "name": "Lightroom by Tony 1.2.3",
            "body": "What changed.",
            "html_url": "https://github.com/tonynehihi/lightroom-by-tony/releases/tag/v1.2.3",
            "zipball_url": "https://api.github.com/repos/tonynehihi/lightroom-by-tony/zipball/v1.2.3"
        }"#;
        let info = parse_release(json).expect("sample release parses");
        assert_eq!(info.version, "1.2.3");
        assert_eq!(info.name, "Lightroom by Tony 1.2.3");
        assert_eq!(info.notes, "What changed.");
        assert!(info.zip_url.ends_with("/zipball/v1.2.3"));
    }

    #[test]
    fn release_json_without_a_source_archive_is_an_error() {
        assert!(parse_release(r#"{"tag_name": "v1.0.0"}"#).is_err());
        assert!(parse_release("not json").is_err());
    }

    #[test]
    fn updater_script_waits_builds_replaces_and_reopens() {
        for needle in ["pgrep -f", "package.sh --arch aarch64", "/Applications/Lightroom by Tony.app", "ditto", "open \"$DEST\""] {
            assert!(UPDATER_SCRIPT.contains(needle), "updater script must contain: {needle}");
        }
    }
}
