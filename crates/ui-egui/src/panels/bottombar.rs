//! The bottom bar: view modes, sort, rating/flags pill, copy/paste settings, zoom and view toggles.

use egui::{Align2, Rect, Sense, pos2, vec2};
use lightcraft_catalog::Flag;
use serde_json::json;

use crate::LightcraftApp;
use crate::icons::{Icon, paint};
use crate::state::{BeforeAfter, ViewMode, Zoom};
use crate::theme::Tokens;
use crate::widgets::{icon_button, register, stars};

pub fn show(app: &mut LightcraftApp, ui: &mut egui::Ui) {
    let t = Tokens::get(ui.ctx());
    egui::Panel::bottom("bottom_bar")
        .exact_size(t.bottom_bar_h)
        .frame(egui::Frame::NONE.fill(t.chrome).inner_margin(egui::Margin { left: 12, right: 12, top: 0, bottom: 0 }))
        .show(ui, |ui| {
            let full = ui.max_rect();
            let left_end = ui
                .horizontal_centered(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    for (id, icon, mode, tip) in [
                        ("photoGrid", Icon::GridPhoto, ViewMode::PhotoGrid, "Photo Grid (G)"),
                        ("squareGrid", Icon::GridSquare, ViewMode::SquareGrid, "Square Grid (G toggles)"),
                        ("detail", Icon::Single, ViewMode::Detail, "Detail (D)"),
                        ("compare", Icon::Compare, ViewMode::Compare, "Compare (Shift+C)"),
                        ("survey", Icon::Survey, ViewMode::Survey, "Survey (N)"),
                        ("people", Icon::Subject, ViewMode::People, "People"),
                    ] {
                        if icon_button(ui, id, icon, vec2(32.0, 32.0), app.ui.view == mode, true, tip).clicked() {
                            let _ = app.run(&format!("view.{id}"), json!({}));
                        }
                    }
                    ui.add_space(10.0);
                    let (sep, _) = ui.allocate_exact_size(vec2(1.0, 22.0), Sense::hover());
                    ui.painter().rect_filled(sep, 0.0, t.button_border);
                    ui.add_space(10.0);
                    let sort = icon_button(ui, "sort", Icon::Sort, vec2(30.0, 30.0), false, true, "Sort");
                    egui::Popup::menu(&sort).show(|ui| sort_menu(app, ui));
                })
                .response
                .rect
                .right();
            let right_start = right_side(app, ui, full);
            centre(app, ui, full, left_end + 12.0, right_start - 12.0);
        });
}

/// Width of the centre group: the rating/flag pill, then separate Copy and Paste Settings
/// buttons, the copy-options gear, and a Select All/None toggle.
const PILL_W: f32 = 196.0;
const COPY_W: f32 = 112.0;
const PASTE_W: f32 = 112.0;
const ALL_W: f32 = 46.0;
const CENTRE_W: f32 = PILL_W + 10.0 + COPY_W + 6.0 + PASTE_W + 4.0 + 30.0 + 6.0 + ALL_W;

/// The rating/flag pill and copy/paste settings, between `from` and `to` (the side groups): the
/// copy buttons go first when there is no room (they are in the Edit menu too), then the pill.
fn centre(app: &mut LightcraftApp, ui: &mut egui::Ui, full: Rect, from: f32, to: f32) {
    let t = Tokens::get(ui.ctx());
    let room = to - from;
    if room < PILL_W {
        return;
    }
    let with_copy = room >= CENTRE_W;
    let w = if with_copy { CENTRE_W } else { PILL_W };
    // where it sits with room to spare (slightly left of centre), kept between the side groups
    let left = (full.center().x - 60.0 - PILL_W / 2.0).clamp(from, to - w);
    let active = app.session.active().and_then(|id| app.session.catalog.photo(id).cloned());
    let pill = Rect::from_min_size(pos2(left, full.center().y - 15.0), vec2(PILL_W, 30.0));
    ui.painter().rect_filled(pill, 15.0, t.canvas);
    let mut pill_ui =
        ui.new_child(egui::UiBuilder::new().max_rect(pill.shrink2(vec2(10.0, 4.0))).layout(egui::Layout::left_to_right(egui::Align::Center)));
    pill_ui.spacing_mut().item_spacing.x = 3.0;
    let rating = active.as_ref().map(|p| p.rating).unwrap_or(0);
    if let Some(r) = stars(&mut pill_ui, "bottom", rating, 19.0) {
        let mut params = json!({"rating": r});
        crate::panels::compare::target_active(app, &mut params);
        let _ = app.run("photo.rate", params);
    }
    pill_ui.add_space(6.0);
    let flag = active.as_ref().map(|p| p.flag).unwrap_or_default();
    for (id, icon, f) in [("unflag", Icon::Circle, Flag::None), ("pick", Icon::FlagPick, Flag::Pick), ("reject", Icon::FlagReject, Flag::Reject)] {
        let (r, resp) = pill_ui.allocate_exact_size(vec2(22.0, 22.0), Sense::click());
        register(pill_ui.ctx(), format!("flag:{id}"), r);
        let on = flag == f && f != Flag::None;
        let c = if on {
            if f == Flag::Reject { t.reject } else { t.pick }
        } else if resp.hovered() {
            t.text
        } else {
            t.icon
        };
        paint(pill_ui.painter(), r.shrink(3.0), icon, c);
        if resp.clicked() {
            let mut params = json!({"flag": match f { Flag::Pick => "pick", Flag::Reject => "reject", Flag::None => "none" }});
            crate::panels::compare::target_active(app, &mut params);
            let _ = app.run("photo.flag", params);
        }
    }
    // copy / paste settings: two separate buttons side by side, so new settings can be
    // copied at any time (the old single button flipped to Paste once something was copied)
    if !with_copy {
        return;
    }
    let has_clip = app.session.clipboard.is_some();
    // when the selected photos' settings differ there is no single settings set to
    // copy: the button greys out and stops responding instead of failing on click
    // (the engine still refuses the command itself, for the other callers)
    let copy_blocked = app.session.copy_is_ambiguous(&json!({}));
    let copy_enabled = active.is_some() && !copy_blocked;
    let copy_r = Rect::from_min_size(pos2(pill.right() + 10.0, pill.top()), vec2(COPY_W, 30.0));
    let cresp = ui.interact(copy_r, egui::Id::new("copy-settings"), if copy_enabled { Sense::click() } else { Sense::hover() });
    register(ui.ctx(), "button:copySettings", copy_r);
    ui.painter().rect_filled(copy_r, 15.0, if cresp.hovered() && copy_enabled { t.hover } else { t.canvas });
    ui.painter().text(
        copy_r.center(),
        Align2::CENTER_CENTER,
        crate::i18n::tr("Copy Settings"),
        t.font(13.0),
        if copy_enabled { t.text_label } else { t.text_disabled },
    );
    let cresp = if copy_blocked {
        cresp.on_hover_text(crate::i18n::tr("The selected photos have different edit settings — select a single photo to copy its settings"))
    } else {
        cresp
    };
    if cresp.clicked() && copy_enabled {
        match app.run("develop.copy", json!({})) {
            Ok(_) => app.toast(ui.ctx(), "Edit settings copied"),
            Err(e) => app.toast(ui.ctx(), e),
        }
    }
    let paste_r = Rect::from_min_size(pos2(copy_r.right() + 6.0, pill.top()), vec2(PASTE_W, 30.0));
    let presp = ui.interact(paste_r, egui::Id::new("paste-settings"), Sense::click());
    register(ui.ctx(), "button:pasteSettings", paste_r);
    ui.painter().rect_filled(paste_r, 15.0, if presp.hovered() && has_clip { t.hover } else { t.canvas });
    ui.painter().text(
        paste_r.center(),
        Align2::CENTER_CENTER,
        crate::i18n::tr("Paste Settings"),
        t.font(13.0),
        if has_clip { t.text_label } else { t.text_disabled },
    );
    if presp.clicked() && has_clip {
        // paste applies to every selected photo (the engine's targets), not just the active one
        match app.run("develop.paste", json!({})) {
            Ok(r) => {
                let n = r["changed"].as_u64().unwrap_or(0);
                app.toast(ui.ctx(), format!("Settings pasted to {n} photo{}", if n == 1 { "" } else { "s" }));
            }
            Err(e) => app.toast(ui.ctx(), e),
        }
    }
    let gear_r = Rect::from_min_size(pos2(paste_r.right() + 4.0, pill.top()), vec2(30.0, 30.0));
    let gresp = ui.interact(gear_r, egui::Id::new("copy-gear"), Sense::click());
    register(ui.ctx(), "icon:copyGear", gear_r);
    ui.painter().rect_filled(gear_r, 15.0, if gresp.hovered() { t.hover } else { t.canvas });
    paint(ui.painter(), gear_r.shrink(7.0), Icon::Gear, t.icon);
    if gresp.clicked() {
        let _ = app.run("dialog.copySettings", json!({}));
    }
    // select all / none toggle for the photos in view
    let vis_n = app.session.visible_cloned().len();
    let all_sel = vis_n > 0 && app.session.selection.ids.len() >= vis_n;
    let all_r = Rect::from_min_size(pos2(gear_r.right() + 6.0, pill.top()), vec2(ALL_W, 30.0));
    let aresp = ui.interact(all_r, egui::Id::new("select-all"), Sense::click());
    register(ui.ctx(), "button:selectAll", all_r);
    ui.painter().rect_filled(all_r, 15.0, if aresp.hovered() && vis_n > 0 { t.hover } else { t.canvas });
    ui.painter().text(
        all_r.center(),
        Align2::CENTER_CENTER,
        crate::i18n::tr(if all_sel { "None" } else { "All" }),
        t.font(12.5),
        if vis_n > 0 { t.text_label } else { t.text_disabled },
    );
    if aresp.clicked() && vis_n > 0 {
        let _ = app.run(if all_sel { "library.selectNone" } else { "library.selectAll" }, json!({}));
    }
}

/// The right-hand group (zoom, view toggles or thumbnail size); returns its left edge.
fn right_side(app: &mut LightcraftApp, ui: &mut egui::Ui, full: Rect) -> f32 {
    let t = Tokens::get(ui.ctx());
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(Rect::from_min_max(pos2(full.right() - 360.0, full.top()), full.right_bottom()))
            .layout(egui::Layout::right_to_left(egui::Align::Center)),
    );
    child.spacing_mut().item_spacing.x = 6.0;
    if matches!(app.ui.view, ViewMode::Compare | ViewMode::Survey)
        && icon_button(
            &mut child,
            "autoAdvance",
            Icon::ChevronRight,
            vec2(30.0, 30.0),
            app.ui.auto_advance,
            true,
            "Auto Advance after rating or flagging",
        )
        .clicked()
    {
        let _ = app.run("view.autoAdvance", json!({}));
    }
    if app.ui.view == ViewMode::Survey {
        // no zoom or thumbnail size in Survey
    } else if matches!(app.ui.view, ViewMode::Detail | ViewMode::Compare) {
        if icon_button(
            &mut child,
            "beforeAfter",
            Icon::BeforeAfter,
            vec2(30.0, 30.0),
            app.ui.before_after != BeforeAfter::Off,
            true,
            "Before/After (Y)",
        )
        .clicked()
        {
            let _ = app.run("view.beforeAfter", json!({}));
        }
        if app.ui.view == ViewMode::Detail
            && icon_button(&mut child, "faceBoxes", Icon::FaceBox, vec2(30.0, 30.0), app.ui.face_boxes, true, "Face boxes").clicked()
        {
            let _ = app.run("view.faceBoxes", json!({}));
        }
        if icon_button(&mut child, "filmstrip", Icon::Filmstrip, vec2(30.0, 30.0), app.ui.filmstrip, true, "Filmstrip (/)").clicked() {
            let _ = app.run("view.filmstrip", json!({}));
        }
        child.add_space(10.0);
        let zoom_label = match app.ui.zoom {
            Zoom::Fit => "Fit".to_string(),
            Zoom::Fill => "Fill".to_string(),
            Zoom::Percent(p) => format!("{p}%"),
        };
        let zr = crate::widgets::dropdown(&mut child, "zoom", crate::i18n::tr(&zoom_label), t.font(13.0), t.text_label);
        egui::Popup::menu(&zr).show(|ui| {
            for (label, z) in [
                ("Fit", Zoom::Fit),
                ("Fill", Zoom::Fill),
                ("50%", Zoom::Percent(50)),
                ("100%", Zoom::Percent(100)),
                ("200%", Zoom::Percent(200)),
                ("400%", Zoom::Percent(400)),
            ] {
                if ui.selectable_label(app.ui.zoom == z, label).clicked() {
                    app.ui.zoom = z;
                    app.ui.pan = (0.5, 0.5);
                }
            }
        });
        child.add_space(6.0);
        let cz = crate::widgets::dropdown(
            &mut child,
            "clickZoom",
            &crate::i18n::tr_format!("Click {}:1", app.ui.click_zoom / 100),
            t.font(13.0),
            t.text_label,
        );
        egui::Popup::menu(&cz).show(|ui| {
            for pct in crate::state::CLICK_ZOOMS {
                if ui.selectable_label(app.ui.click_zoom == pct, format!("{}:1", pct / 100)).clicked() {
                    let _ = app.run("view.clickZoom", json!({"ratio": pct / 100}));
                }
            }
        });
    } else {
        // thumbnail size slider
        let (r, resp) = child.allocate_exact_size(vec2(110.0, 20.0), Sense::click_and_drag());
        register(child.ctx(), "slider:thumbSize", r);
        let p = child.painter();
        let y = r.center().y;
        p.line_segment([pos2(r.left(), y), pos2(r.right(), y)], egui::Stroke::new(2.0, t.track));
        let f = (app.ui.thumb_size - 90.0) / (480.0 - 90.0);
        let x = r.left() + f * r.width();
        p.rect_filled(Rect::from_center_size(pos2(x, y), vec2(3.0, 14.0)), 1.0, t.thumb_hover);
        if let Some(pp) = resp.interact_pointer_pos()
            && (resp.dragged() || resp.clicked())
        {
            app.ui.thumb_size = 90.0 + ((pp.x - r.left()) / r.width()).clamp(0.0, 1.0) * (480.0 - 90.0);
        }
    }
    if child.min_rect().width() > 0.0 { child.min_rect().left() } else { full.right() }
}

fn sort_menu(app: &mut LightcraftApp, ui: &mut egui::Ui) {
    use lightcraft_catalog::GroupBy;
    use lightcraft_catalog::SortKey::*;
    let cur = app.session.sort;
    ui.label(egui::RichText::new(crate::i18n::tr("Sort by")).weak());
    for (label, key, k) in [
        ("Capture Date", CaptureDate, "captureDate"),
        ("Import Date", ImportDate, "importDate"),
        ("Modified Date", EditDate, "editDate"),
        ("File Name", FileName, "fileName"),
        ("Rating", Rating, "rating"),
        ("File Size", FileSize, "fileSize"),
        ("Random", Random, "random"),
    ] {
        if ui.selectable_label(cur.key == key, crate::i18n::tr(label)).clicked() {
            let _ = app.run("library.sort", json!({"key": k}));
        }
    }
    if cur.key == Random && ui.button(crate::i18n::tr("Reshuffle")).clicked() {
        let _ = app.run("library.shuffle", json!({}));
    }
    ui.separator();
    // a shuffle has no direction worth choosing
    ui.add_enabled_ui(cur.key != Random, |ui| {
        if ui.selectable_label(cur.key != Random && cur.ascending, crate::i18n::tr("Ascending")).clicked() {
            let _ = app.run("library.sort", json!({"ascending": true}));
        }
        if ui.selectable_label(cur.key != Random && !cur.ascending, crate::i18n::tr("Descending")).clicked() {
            let _ = app.run("library.sort", json!({"ascending": false}));
        }
    });
    ui.separator();
    ui.label(egui::RichText::new(crate::i18n::tr("Group by date")).weak());
    for (label, g, k) in [
        ("Automatic", GroupBy::Auto, "auto"),
        ("Day", GroupBy::Day, "day"),
        ("Month", GroupBy::Month, "month"),
        ("Year", GroupBy::Year, "year"),
        ("None", GroupBy::None, "none"),
    ] {
        if ui.selectable_label(cur.group == g, crate::i18n::tr(label)).clicked() {
            let _ = app.run("library.sort", json!({"group": k}));
        }
    }
}
