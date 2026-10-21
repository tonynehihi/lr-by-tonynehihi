//! Opening web links in the user's browser.
//!
//! This personal build carries no upstream project/community links: the only
//! URLs ever opened are the app's own update pages (see `updates`) and
//! contributor/license links inside dialogs.

/// Open `url` in the user's browser (through the host's `open_url` service).
pub fn open(app: &mut crate::LightcraftApp, url: &str) -> Result<serde_json::Value, String> {
    let open = app.services.open_url.as_mut().ok_or("can't open links here")?;
    open(url)?;
    Ok(serde_json::json!({ "url": url }))
}
