use super::{INFO, ModRow, VISIBLE};
use crate::config::{self, Entrypoint, Kind, LoaderConfig, Setting};
use crate::log::{self, Level};
use egui::{Color32, RichText};
use std::collections::HashMap;
use std::sync::atomic::Ordering;
use toml_edit::{Array, Value};

#[derive(Default, PartialEq, Eq, Clone, Copy)]
enum Tab {
    #[default]
    Mods,
    Console,
    Loader,
}

#[derive(Default)]
pub struct UiState {
    tab: Tab,
    filter: String,
    edits: HashMap<(String, String), String>,
    input: String,
    loader: Option<LoaderConfig>,
    saved: bool,
    log_filter: String,
}

const ACCENT: Color32 = Color32::from_rgb(232, 120, 64);

pub fn style(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.selection.bg_fill = ACCENT.gamma_multiply(0.6);
    visuals.hyperlink_color = ACCENT;
    visuals.window_corner_radius = egui::CornerRadius::same(8);
    visuals.menu_corner_radius = egui::CornerRadius::same(8);
    for w in [
        &mut visuals.widgets.noninteractive,
        &mut visuals.widgets.inactive,
        &mut visuals.widgets.hovered,
        &mut visuals.widgets.active,
        &mut visuals.widgets.open,
    ] {
        w.corner_radius = egui::CornerRadius::same(w.corner_radius.nw.saturating_add(2));
    }
    ctx.set_visuals(visuals);
}

fn loader_path() -> Option<std::path::PathBuf> {
    INFO.get().map(|i| i.root.join("ferrite.toml"))
}

static SHARED: std::sync::Mutex<Option<UiState>> = std::sync::Mutex::new(None);

pub fn draw_shared(ui: &mut egui::Ui) {
    let mut guard = SHARED
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    draw(ui.ctx(), guard.get_or_insert_with(UiState::default));
}

fn draw(ctx: &egui::Context, st: &mut UiState) {
    let mut open = true;
    let title = INFO.get().map_or_else(
        || format!("Ferrite {}", env!("CARGO_PKG_VERSION")),
        |i| {
            format!(
                "Ferrite {} — {} ({})",
                env!("CARGO_PKG_VERSION"),
                i.game,
                i.runtime
            )
        },
    );
    egui::Window::new(title)
        .id(egui::Id::new("ferrite_menu"))
        .open(&mut open)
        .default_pos(egui::pos2(40.0, 60.0))
        .default_size(egui::vec2(640.0, 480.0))
        .min_size(egui::vec2(460.0, 300.0))
        .resizable(true)
        .scroll(false)
        .collapsible(false)
        .constrain(true)
        .show(ctx, |ui| {
            ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
                ui.small(format!(
                    "{} or Esc closes the menu",
                    super::input::menu_key_name()
                ));
                ui.separator();
                ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                    ui.horizontal(|ui| {
                        let count = super::mods().len();
                        ui.selectable_value(&mut st.tab, Tab::Mods, format!("Mods ({count})"));
                        ui.selectable_value(&mut st.tab, Tab::Console, "Console");
                        ui.selectable_value(&mut st.tab, Tab::Loader, "Loader");
                    });
                    ui.separator();
                    match st.tab {
                        Tab::Mods => mods_tab(ui, st),
                        Tab::Console => console_tab(ui, st),
                        Tab::Loader => loader_tab(ui, st),
                    }
                });
            });
        });
    if !open {
        VISIBLE.store(false, Ordering::SeqCst);
    }
}

fn mods_tab(ui: &mut egui::Ui, st: &mut UiState) {
    ui.horizontal(|ui| {
        ui.label("Search");
        ui.add(egui::TextEdit::singleline(&mut st.filter).desired_width(f32::INFINITY));
    });
    let disabled = loader_path()
        .map(|p| config::read_loader(&p).mods.disabled)
        .unwrap_or_default();
    let filter = st.filter.to_lowercase();
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let mods = super::mods();
            if mods.is_empty() {
                ui.label("No mods loaded. Put mods into Ferrite\\mods.");
            }
            for m in mods {
                let settings = config::with_mod_configs(|c| c.settings(&m.id)).unwrap_or_default();
                let matches = filter.is_empty()
                    || m.name.to_lowercase().contains(&filter)
                    || m.id.to_lowercase().contains(&filter)
                    || settings
                        .iter()
                        .any(|s| s.key.to_lowercase().contains(&filter));
                if matches {
                    mod_entry(ui, st, &m, &settings, &disabled, !filter.is_empty());
                }
            }
            for id in disabled
                .iter()
                .filter(|d| !super::mods().iter().any(|m| m.id.eq_ignore_ascii_case(d)))
            {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("{id} (disabled)")).weak());
                    if ui.small_button("enable").clicked() {
                        set_disabled(id, false);
                    }
                });
            }
        });
}

fn mod_entry(
    ui: &mut egui::Ui,
    st: &mut UiState,
    m: &ModRow,
    settings: &[Setting],
    disabled: &[String],
    force_open: bool,
) {
    let mut header = RichText::new(format!("{}  {}", m.name, m.version)).strong();
    if m.faulted {
        header = header.color(Color32::from_rgb(230, 90, 90));
    }
    let id = egui::Id::new(("ferrite_mod", &m.id));
    let mut state =
        egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, false);
    if force_open {
        state.set_open(true);
    }
    state
        .show_header(ui, |ui| {
            ui.label(header);
            ui.label(RichText::new(m.kind).small().color(ACCENT));
            if !m.author.is_empty() {
                ui.label(RichText::new(format!("by {}", m.author)).weak());
            }
            if m.faulted {
                ui.label(RichText::new("faulted").color(Color32::from_rgb(230, 90, 90)));
            }
        })
        .body(|ui| {
            ui.label(RichText::new(&m.id).weak().small());
            if settings.is_empty() {
                ui.label(RichText::new("no settings").weak());
            } else {
                egui::Grid::new(("ferrite_settings", &m.id))
                    .num_columns(2)
                    .spacing([16.0, 6.0])
                    .striped(true)
                    .show(ui, |ui| {
                        for s in settings {
                            setting_row(ui, st, &m.id, s);
                            ui.end_row();
                        }
                    });
            }
            ui.horizontal(|ui| {
                if m.kind != "BepInEx"
                    && ui
                        .button("Reload")
                        .on_hover_text("Reload the mod from its dll")
                        .clicked()
                {
                    log::push_command(format!("reload {}", m.id));
                }
                let off = disabled.iter().any(|d| d.eq_ignore_ascii_case(&m.id));
                let label = if off {
                    "Enable on next start"
                } else {
                    "Disable on next start"
                };
                if ui.button(label).clicked() {
                    set_disabled(&m.id, !off);
                }
            });
        });
}

fn setting_row(ui: &mut egui::Ui, st: &mut UiState, id: &str, s: &Setting) {
    let label = ui.label(&s.key);
    if !s.description.is_empty() {
        label.on_hover_text(&s.description);
    }
    if s.value == "true" || s.value == "false" {
        let mut on = s.value == "true";
        if ui.checkbox(&mut on, "").changed() {
            apply(id, &s.key, if on { "true" } else { "false" });
        }
        return;
    }
    let key = (id.to_owned(), s.key.clone());
    let text = st
        .edits
        .entry(key.clone())
        .or_insert_with(|| s.value.clone());
    let response = ui.add(egui::TextEdit::singleline(text).desired_width(260.0));
    if response.lost_focus() {
        if *text != s.value {
            apply(id, &s.key, text);
        }
        st.edits.remove(&key);
    } else if !response.has_focus() && *text != s.value {
        text.clone_from(&s.value);
    }
}

fn apply(id: &str, key: &str, value: &str) {
    config::with_mod_configs(|c| {
        if c.set(id, key, value, Kind::Infer).is_ok() {
            c.notify(id);
        }
    });
}

fn set_disabled(id: &str, disable: bool) {
    let Some(path) = loader_path() else { return };
    let _ = config::edit_loader(&path, |doc| {
        let mods = doc
            .entry("mods")
            .or_insert_with(|| toml_edit::Item::Table(toml_edit::Table::new()));
        let Some(table) = mods.as_table_mut() else {
            return;
        };
        let mut list: Vec<String> = table
            .get("disabled")
            .and_then(|i| i.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(str::to_owned))
                    .collect()
            })
            .unwrap_or_default();
        list.retain(|d| !d.eq_ignore_ascii_case(id));
        if disable {
            list.push(id.to_owned());
        }
        let array: Array = list.into_iter().collect();
        table.insert("disabled", toml_edit::Item::Value(Value::Array(array)));
    });
}

fn level_color(line: &str) -> Color32 {
    if line.starts_with("[Error]") {
        Color32::from_rgb(240, 100, 100)
    } else if line.starts_with("[Warning]") {
        Color32::from_rgb(235, 200, 90)
    } else if line.starts_with("[Debug]") {
        Color32::GRAY
    } else {
        Color32::from_rgb(215, 215, 215)
    }
}

fn console_tab(ui: &mut egui::Ui, st: &mut UiState) {
    ui.horizontal(|ui| {
        ui.label("Filter");
        ui.add(egui::TextEdit::singleline(&mut st.log_filter).desired_width(f32::INFINITY));
    });
    let filter = st.log_filter.to_lowercase();
    let text = log::recent();
    ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
        ui.horizontal(|ui| {
            let edit = ui.add(
                egui::TextEdit::singleline(&mut st.input)
                    .desired_width(ui.available_width() - 60.0)
                    .hint_text("command, try help"),
            );
            let enter = edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            if (ui.button("Run").clicked() || enter) && !st.input.trim().is_empty() {
                log::push_command(st.input.trim().to_owned());
                st.input.clear();
                edit.request_focus();
            }
        });
        ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    for line in text
                        .lines()
                        .filter(|l| filter.is_empty() || l.to_lowercase().contains(&filter))
                    {
                        ui.label(RichText::new(line).monospace().color(level_color(line)));
                    }
                });
        });
    });
}

fn loader_tab(ui: &mut egui::Ui, st: &mut UiState) {
    let Some(path) = loader_path() else { return };
    let cfg = st.loader.get_or_insert_with(|| config::read_loader(&path));
    let before = format!("{cfg:?}");
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Grid::new("ferrite_loader")
                .num_columns(2)
                .spacing([16.0, 8.0])
                .show(ui, |ui| {
                    ui.label("Console window");
                    ui.checkbox(&mut cfg.loader.console, "");
                    ui.end_row();
                    ui.label("Capture Unity log");
                    ui.checkbox(&mut cfg.loader.unity_log, "");
                    ui.end_row();
                    ui.label("Start mods");
                    ui.horizontal(|ui| {
                        ui.selectable_value(
                            &mut cfg.loader.entrypoint,
                            Entrypoint::Early,
                            "before the first scene",
                        );
                        ui.selectable_value(
                            &mut cfg.loader.entrypoint,
                            Entrypoint::Scene,
                            "on the first scene",
                        );
                    });
                    ui.end_row();
                    ui.label("Menu key");
                    egui::ComboBox::from_id_salt("ferrite_menu_key")
                        .selected_text(cfg.loader.menu_key.clone())
                        .show_ui(ui, |ui| {
                            for (name, _) in super::input::NAMES {
                                ui.selectable_value(
                                    &mut cfg.loader.menu_key,
                                    name.to_owned(),
                                    name,
                                );
                            }
                        });
                    ui.end_row();
                    ui.label("Hot reload mods");
                    ui.checkbox(&mut cfg.mods.hot_reload, "");
                    ui.end_row();
                    ui.label("Console log level");
                    level_combo(ui, "ferrite_console_level", &mut cfg.log.console_level);
                    ui.end_row();
                    ui.label("File log level");
                    level_combo(ui, "ferrite_file_level", &mut cfg.log.file_level);
                    ui.end_row();
                    ui.label("Mono debugger");
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut cfg.debug.enabled, "");
                        ui.add(
                            egui::TextEdit::singleline(&mut cfg.debug.address).desired_width(150.0),
                        );
                        ui.checkbox(&mut cfg.debug.suspend, "wait for debugger");
                    });
                    ui.end_row();
                    ui.label("Dump patched assemblies");
                    ui.checkbox(&mut cfg.patchers.dump_assemblies, "");
                    ui.end_row();
                });
            if format!("{cfg:?}") != before {
                let cfg = cfg.clone();
                let _ = config::edit_loader(&path, |doc| write_loader(doc, &cfg));
                log::set_console_level(cfg.log.console_level);
                super::input::set_menu_key(&cfg.loader.menu_key);
                st.saved = true;
            }
            ui.add_space(8.0);
            if st.saved {
                ui.label(
                    RichText::new("Saved to ferrite.toml. Most options apply on the next start.")
                        .color(ACCENT),
                );
            }
            ui.label(RichText::new(path.display().to_string()).weak().small());
        });
}

fn level_combo(ui: &mut egui::Ui, id: &str, level: &mut Level) {
    egui::ComboBox::from_id_salt(id)
        .selected_text(level.to_string())
        .show_ui(ui, |ui| {
            for l in [Level::Debug, Level::Info, Level::Warning, Level::Error] {
                ui.selectable_value(level, l, l.to_string());
            }
        });
}

fn write_loader(doc: &mut toml_edit::DocumentMut, cfg: &LoaderConfig) {
    let Ok(full) = toml::to_string_pretty(cfg) else {
        return;
    };
    let Ok(full) = full.parse::<toml_edit::DocumentMut>() else {
        return;
    };
    for (section, item) in full.iter() {
        let Some(values) = item.as_table() else {
            continue;
        };
        let Some(target) = doc
            .entry(section)
            .or_insert_with(|| toml_edit::Item::Table(toml_edit::Table::new()))
            .as_table_mut()
        else {
            continue;
        };
        for (key, value) in values {
            if key == "disabled" {
                continue;
            }
            match target.get_mut(key) {
                Some(existing) => *existing = value.clone(),
                None => {
                    target.insert(key, value.clone());
                }
            }
        }
    }
}
