//! The missing-photo prompt (this fork): when the photo open for editing has lost
//! its original file — moved or deleted outside the app — a small window says so
//! and offers the ways out: locate the exact file again (verified by the engine,
//! so a same-name look-alike is refused), or point at a folder and let the app
//! search it — the search matches by content, so a renamed file is found too —
//! or remove the photo from the library, or wave the prompt away for this run.
//! The Missing Photos view remains the place for bulk repairs; this is the
//! in-the-moment version, so editing never just silently stops working.

use egui::Context;
use serde_json::json;

use crate::state::ViewMode;
use crate::theme::Tokens;
use crate::LightcraftApp;

/// Label of the background folder search (one at a time).
const SEARCH_LABEL: &str = "Locate in Folder";

/// Show the prompt while the active photo (in the edit view) is missing its file.
pub fn window(app: &mut LightcraftApp, ctx: &Context) {
    if !matches!(app.ui.view, ViewMode::Detail) || app.ui.dialog.is_some() {
        return;
    }
    let Some(id) = app.session.active() else { return };
    if app.missing_dismissed.contains(&id) {
        return;
    }
    let Some(ph) = app.session.catalog.photo(id) else { return };
    if ph.deleted || !ph.in_library() {
        return;
    }
    let lightcraft_catalog::Source::File { path } = &ph.source else { return };
    if !app.session.media.availability.is_offline(path) {
        return;
    }
    let path = path.clone();
    let t = Tokens::get(ctx);
    let searching = app.tasks.is_running(SEARCH_LABEL);
    let mut locate = false;
    let mut locate_folder = false;
    let mut remove = false;
    let mut dismiss = false;
    egui::Window::new(crate::i18n::tr("Photo file is missing"))
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            ui.set_width(430.0);
            ui.label(crate::i18n::tr("The original file for this photo was moved or deleted outside the app:"));
            ui.label(egui::RichText::new(&path).color(t.text_dim));
            ui.add_space(2.0);
            ui.label(crate::i18n::tr("Its preview is still here, but the photo can't be edited until it is linked to its file again."));
            if let Some(e) = &app.missing_error {
                ui.add_space(4.0);
                ui.label(egui::RichText::new(e.as_str()).color(t.text_dim));
            }
            if searching {
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(crate::i18n::tr("Searching the folder for this photo..."));
                });
            }
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if crate::widgets::text_button(ui, "missingLocate", "Locate File...", false).clicked() {
                    locate = true;
                }
                if crate::widgets::text_button(ui, "missingLocateFolder", "Locate in Folder...", false).clicked() {
                    locate_folder = true;
                }
            });
            ui.add_space(2.0);
            ui.horizontal(|ui| {
                if crate::widgets::text_button(ui, "missingRemove", "Remove from Library", false).clicked() {
                    remove = true;
                }
                if crate::widgets::text_button(ui, "missingLater", "Not Now", false).clicked() {
                    dismiss = true;
                }
            });
        });
    if locate {
        match app.run("photo.locate", json!({"verify": true})) {
            Ok(_) => {
                app.missing_error = None;
                app.toast(ctx, crate::i18n::tr("Photo linked to its file again"));
            }
            Err(e) => app.missing_error = Some(e),
        }
    }
    if locate_folder && !searching {
        // point at a folder and the engine's search looks for this photo in it,
        // recursively: same name and size, or — renamed — the same content.
        // The walk and the hashing run on a worker thread (see file.findMissing).
        if let Some(folder) = app.services.pick_folder.as_mut().and_then(|f| f()) {
            let candidates = lightcraft_engine::cmd::missing::find_candidates(&app.session.catalog);
            let work = move || lightcraft_engine::cmd::missing::plan_find_missing(&candidates, &folder);
            let done = move |app: &mut LightcraftApp, ctx: &Context, plan: Result<lightcraft_engine::cmd::missing::FindPlan, String>| match plan {
                Ok(plan) => {
                    if let Some(f) = plan.found.iter().find(|f| f.id == id) {
                        // the plan only names a file whose size and content match the
                        // photo the library knows, so it can be linked straight away
                        match app.run("photo.relink", json!({"id": id.0, "path": f.to})) {
                            Ok(_) => {
                                app.missing_error = None;
                                app.toast(ctx, crate::i18n::tr("Photo linked to its file again"));
                            }
                            Err(e) => app.missing_error = Some(e),
                        }
                    } else if plan.ambiguous.iter().any(|a| a.id == id) {
                        app.missing_error = Some(crate::i18n::tr(
                            "Several files in that folder look like this photo (same name and size) — use Locate File... to pick the exact one.",
                        ).to_string());
                    } else {
                        app.missing_error = Some(crate::i18n::tr(
                            "No file in that folder matches this photo. The search compares the photo's content, so a renamed file would have been found — the file may have been edited or deleted.",
                        ).to_string());
                    }
                }
                Err(e) => app.missing_error = Some(e),
            };
            if let Err(e) = crate::tasks::spawn(app, SEARCH_LABEL, work, done) {
                app.missing_error = Some(e);
            }
        }
    }
    if remove {
        // permanent, not Recently Deleted: restoring a record whose file is gone
        // would only bring back a photo that still can't be edited
        match app.run("photo.deletePermanently", json!({"ids": [id.0]})) {
            Ok(_) => {
                app.missing_error = None;
                app.toast(ctx, crate::i18n::tr("Photo removed from the library"));
            }
            Err(e) => app.missing_error = Some(e),
        }
    }
    if dismiss {
        app.missing_dismissed.insert(id);
        app.missing_error = None;
    }
}
