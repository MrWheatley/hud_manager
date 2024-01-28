#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::collections::HashSet;

use eframe::egui::*;
use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
use nucleo_matcher::Matcher;

use hud_manager::{Hud, Huds};

const FONT_NAME: &str = "Inter";
const FONT_DATA: &[u8] = include_bytes!("../Inter-Regular.ttf");

enum Msg {
    Favorited,
    SetActive(String),
    Error(anyhow::Error),
}

#[derive(Default)]
struct App {
    huds: Huds,

    search: String,
    search_results: HashSet<String>,
    matcher: Matcher,

    msg: Option<Msg>,
    error: String,
}

impl App {
    fn new() -> Self {
        let mut huds = Huds::default();

        let error = huds
            .update_favorites()
            .and_then(|_| huds.scan_for_huds())
            .map_or_else(|e| format!("{e:#}"), |_| String::new());

        Self {
            huds,
            error,
            ..Default::default()
        }
    }

    fn search(&mut self) {
        self.search_results.clear();
        self.error.clear();

        if self.search.is_empty() {
            return;
        }

        let hud_names = self.huds.huds.iter().map(|h| h.name.as_str());
        let search_results =
            Pattern::parse(&self.search, CaseMatching::Ignore, Normalization::Never)
                .match_list(hud_names, &mut self.matcher);

        let highest_score = if search_results.is_empty() {
            self.error(anyhow::anyhow!("no results"));
            return;
        } else {
            search_results[0].1
        };

        for (hud, score) in search_results {
            if (score as f32 / highest_score as f32) >= 0.8 {
                self.search_results.insert(hud.to_string());
            }
        }
    }

    fn error(&mut self, e: anyhow::Error) {
        self.error = format!("{e:#}");
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        if let Some(msg) = self.msg.take() {
            self.error.clear();

            match msg {
                Msg::Favorited => {
                    self.huds.huds.sort_unstable();

                    if let Err(e) = self.huds.save_favorites() {
                        self.error(e);
                    }
                }
                Msg::SetActive(hud) => {
                    if let Err(e) = self
                        .huds
                        .set_active_hud(&hud)
                        .and_then(|_| self.huds.scan_for_huds())
                    {
                        self.error(e);
                    }
                }
                Msg::Error(e) => self.error(e),
            }
        }

        TopBottomPanel::bottom("status_bar")
            .show_separator_line(false)
            .frame(
                Frame::default()
                    .fill(ctx.style().visuals.panel_fill)
                    .inner_margin(Margin::same(8.0)),
            )
            .show(ctx, |ui| {
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(&self.error);
                        ui.allocate_space(ui.available_size());
                    });
                });
            });

        CentralPanel::default().show(ctx, |ui| {
            ui.group(|ui| {
                ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                    ui.horizontal(|ui| {
                        if ui.button("Search").clicked() {
                            self.search();
                        }
                    });

                    let text_edit = ui.add_sized(
                        [ui.available_width(), 0.0],
                        TextEdit::singleline(&mut self.search).hint_text("hud name"),
                    );

                    if text_edit.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                        text_edit.request_focus();
                        self.search();
                    }
                });
            });
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.label("Current hud:");
                    if let Some(hud) = &self.huds.active_hud {
                        ui.colored_label(
                            ui.style().visuals.widgets.inactive.fg_stroke.color,
                            &hud.name,
                        );
                        if ui
                            .add(Button::new("↪").fill(Color32::TRANSPARENT))
                            .on_hover_text("open folder")
                            .clicked()
                        {
                            if let Err(e) = open::that(&hud.path) {
                                self.error(e.into());
                            }
                        }
                    }
                    ui.allocate_space(ui.available_size());
                });
            });
            ui.group(|ui| {
                ui.columns(2, |col| {
                    col[0].vertical(|ui| {
                        let total_rows = self
                            .huds
                            .huds
                            .iter()
                            .filter(|hud| {
                                !hud.favorite
                                    && (self.search_results.is_empty()
                                        || self.search_results.contains(&hud.name))
                            })
                            .count();

                        ScrollArea::vertical().show_rows(
                            ui,
                            ui.spacing().interact_size.y,
                            total_rows,
                            |ui, range| {
                                Grid::new("huds")
                                    .num_columns(1)
                                    .striped(true)
                                    .start_row(range.start)
                                    .show(ui, |ui| {
                                        for hud in self
                                            .huds
                                            .huds
                                            .iter_mut()
                                            .filter(|hud| {
                                                !hud.favorite
                                                    && (self.search_results.is_empty()
                                                        || self.search_results.contains(&hud.name))
                                            })
                                            .skip(range.start)
                                            .take(range.end)
                                        {
                                            let active_hud = self
                                                .huds
                                                .active_hud
                                                .as_ref()
                                                .map(|hud| hud.name.as_str());

                                            hud_list_button(ui, hud, &mut self.msg, active_hud);
                                            ui.end_row();
                                        }
                                    });
                            },
                        );
                    });
                    col[1].vertical(|ui| {
                        ui.push_id("fav_huds_scroll", |ui| {
                            let total_rows = self
                                .huds
                                .huds
                                .iter()
                                .filter(|hud| {
                                    self.search_results.is_empty()
                                        || self.search_results.contains(&hud.name)
                                })
                                .take_while(|hud| hud.favorite)
                                .count();

                            ScrollArea::vertical().show_rows(
                                ui,
                                ui.spacing().interact_size.y,
                                total_rows,
                                |ui, range| {
                                    Grid::new("fav_huds")
                                        .num_columns(1)
                                        .striped(true)
                                        .start_row(range.start)
                                        .show(ui, |ui| {
                                            for hud in self
                                                .huds
                                                .huds
                                                .iter_mut()
                                                .filter(|hud| {
                                                    self.search_results.is_empty()
                                                        || self.search_results.contains(&hud.name)
                                                })
                                                .take_while(|hud| hud.favorite)
                                                .skip(range.start)
                                                .take(range.end)
                                            {
                                                let active_hud = self
                                                    .huds
                                                    .active_hud
                                                    .as_ref()
                                                    .map(|hud| hud.name.as_str());

                                                hud_list_button(ui, hud, &mut self.msg, active_hud);
                                                ui.end_row();
                                            }
                                        });
                                },
                            );
                        });
                    });
                });
                ui.allocate_space(ui.available_size());
            });
        });
    }
}

fn hud_list_button(ui: &mut Ui, hud: &mut Hud, msg: &mut Option<Msg>, active_hud: Option<&str>) {
    let right_align = Layout {
        main_dir: Direction::LeftToRight,
        main_wrap: false,
        main_align: Align::Min,
        main_justify: true,
        cross_align: Align::Center,
        cross_justify: true,
    };

    let center_align = Layout {
        main_dir: Direction::LeftToRight,
        main_wrap: false,
        main_align: Align::Center,
        main_justify: true,
        cross_align: Align::Center,
        cross_justify: true,
    };

    ui.with_layout(Layout::left_to_right(Align::Min), |ui| {
        ui.scope(|ui| {
            ui.style_mut().spacing.item_spacing.x = 2.0;

            ui.allocate_ui_with_layout([0.0, 0.0].into(), center_align, |ui| {
                let icon = if hud.favorite { "★" } else { "☆" };
                if ui
                    .toggle_value(&mut hud.favorite, icon)
                    .on_hover_text("toggle favorite")
                    .clicked()
                {
                    *msg = Some(Msg::Favorited);
                }
            });
            ui.allocate_ui_with_layout(
                [ui.available_width() - 25.0, 0.0].into(),
                right_align,
                |ui| {
                    let fill = if Some(hud.name.as_str()) == active_hud {
                        ui.style().visuals.selection.bg_fill
                    } else {
                        Color32::TRANSPARENT
                    };

                    if ui
                        .add(Button::new(&hud.name).fill(fill))
                        .on_hover_text("set active")
                        .clicked()
                    {
                        *msg = Some(Msg::SetActive(hud.name.clone()));
                    }
                },
            );
            ui.allocate_ui_with_layout([ui.available_width(), 0.0].into(), center_align, |ui| {
                if ui
                    .add(Button::new("↪").fill(Color32::TRANSPARENT))
                    .on_hover_text("open folder")
                    .clicked()
                {
                    if let Err(e) = open::that(&hud.path) {
                        *msg = Some(Msg::Error(e.into()));
                    }
                }
            });
        });
    });
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: ViewportBuilder::default().with_inner_size([650.0, 700.0]),
        ..Default::default()
    };

    eframe::run_native(
        "hud manager",
        options,
        Box::new(|cc| {
            let mut font_def = FontDefinitions::empty();

            font_def
                .font_data
                .insert(FONT_NAME.to_string(), FontData::from_static(FONT_DATA));

            font_def
                .families
                .get_mut(&FontFamily::Proportional)
                .unwrap()
                .insert(0, FONT_NAME.to_string());

            cc.egui_ctx.set_fonts(font_def);

            cc.egui_ctx.style_mut(|style| {
                style.spacing.scroll = style::ScrollStyle::solid();
                style.animation_time = 0.0;
                style.interaction.tooltip_delay = 0.33;
            });

            Box::new(App::new())
        }),
    )
}
