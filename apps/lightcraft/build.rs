//! Windows only: embed the app icon and version info (VERSIONINFO) into the exe, so
//! Explorer, the taskbar, the Start menu and Alt-Tab show the LrT icon.
//!
//! Tony's local rebrand ("Lightroom by Tony"): embeds assets/app-icon/lrt.ico and
//! "Lightroom by Tony" VERSIONINFO strings instead of the upstream LightCraft ones.
//!
//! On every other target this does nothing. A missing resource compiler is a warning, so a
//! cross-compile from macOS or Linux still links, unless `LIGHTCRAFT_REQUIRE_WINRES=1` turns it
//! into an error (for release builds).

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=../../assets/app-icon/lrt.ico");
    println!("cargo:rerun-if-env-changed=LIGHTCRAFT_REQUIRE_WINRES");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let mut res = winresource::WindowsResource::new();
    res.set_icon("../../assets/app-icon/lrt.ico")
        .set("ProductName", "Lightroom by Tony")
        .set("FileDescription", "Lightroom by Tony photo library and raw developer")
        .set("LegalCopyright", "Copyright (c) the LightCraft authors. MIT OR Apache-2.0.")
        .set("OriginalFilename", "lightroom-by-tony.exe")
        .set("InternalName", "lightroom-by-tony");
    if let Err(e) = res.compile() {
        if std::env::var_os("LIGHTCRAFT_REQUIRE_WINRES").is_some() {
            panic!("embedding Windows resources failed: {e}");
        }
        println!("cargo:warning=lightcraft.exe built without icon/version resources: {e}");
    }
}
