mod db;

use db::*;
use eframe::egui;
use rusqlite::Connection;
use std::sync::Mutex;

// ── App State ────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
enum Screen {
    Dashboard,
    ProjectDetail(i64),
    Search,
}

struct App {
    conn: Mutex<Connection>,
    screen: Screen,

    // Cached data
    projects: Vec<Project>,
    tasks: Vec<Task>,
    logs: Vec<Log>,
    current_project: Option<Project>,
    latest_log: Option<Log>,

    // Form state — Dashboard
    show_create_project: bool,
    new_project_name: String,
    new_project_goal: String,

    // Form state — Project Detail
    new_task_desc: String,
    new_log_notes: String,
    new_log_next_action: String,
    edit_project_name: String,
    edit_project_goal: String,
    edit_project_progress: i32,
    edit_project_current_task: String,
    show_edit_project: bool,

    // Search state
    search_query: String,
    search_results: Vec<SearchResult>,
    search_performed: bool,

    // Flags
    needs_refresh: bool,
}

impl App {
    fn new(db_path: &str) -> Self {
        let conn = open_db(db_path).expect("Failed to open database");
        let projects = get_all_projects(&conn).unwrap_or_default();
        Self {
            conn: Mutex::new(conn),
            screen: Screen::Dashboard,
            projects,
            tasks: Vec::new(),
            logs: Vec::new(),
            current_project: None,
            latest_log: None,
            show_create_project: false,
            new_project_name: String::new(),
            new_project_goal: String::new(),
            new_task_desc: String::new(),
            new_log_notes: String::new(),
            new_log_next_action: String::new(),
            edit_project_name: String::new(),
            edit_project_goal: String::new(),
            edit_project_progress: 0,
            edit_project_current_task: String::new(),
            show_edit_project: false,
            search_query: String::new(),
            search_results: Vec::new(),
            search_performed: false,
            needs_refresh: false,
        }
    }

    fn refresh_projects(&mut self) {
        let conn = self.conn.lock().unwrap();
        self.projects = get_all_projects(&conn).unwrap_or_default();
    }

    fn refresh_project_detail(&mut self, project_id: i64) {
        let conn = self.conn.lock().unwrap();
        self.current_project = get_project(&conn, project_id).ok();
        self.tasks = get_tasks_for_project(&conn, project_id).unwrap_or_default();
        self.logs = get_logs_for_project(&conn, project_id).unwrap_or_default();
        self.latest_log = get_latest_log(&conn, project_id).unwrap_or(None);
    }

    fn navigate_to_project(&mut self, project_id: i64) {
        self.screen = Screen::ProjectDetail(project_id);
        self.refresh_project_detail(project_id);
        self.show_edit_project = false;
        self.new_task_desc.clear();
        self.new_log_notes.clear();
        self.new_log_next_action.clear();
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.needs_refresh {
            self.needs_refresh = false;
            self.refresh_projects();
            if let Screen::ProjectDetail(id) = self.screen {
                self.refresh_project_detail(id);
            }
        }

        // ── Global keyboard shortcuts ────────────────────────
        let mut shortcut_action: Option<&str> = None;
        ctx.input(|i| {
            if i.key_pressed(egui::Key::N) && i.modifiers.ctrl {
                shortcut_action = Some("new_task");
            }
            if i.key_pressed(egui::Key::F) && i.modifiers.ctrl {
                shortcut_action = Some("search");
            }
            if i.key_pressed(egui::Key::D) && i.modifiers.ctrl {
                shortcut_action = Some("dashboard");
            }
        });
        match shortcut_action {
            Some("search") => {
                self.screen = Screen::Search;
                self.search_query.clear();
                self.search_results.clear();
                self.search_performed = false;
            }
            Some("dashboard") => {
                self.screen = Screen::Dashboard;
                self.refresh_projects();
            }
            Some("new_task") => {
                if let Screen::Dashboard = self.screen {
                    self.show_create_project = true;
                }
            }
            _ => {}
        }

        // ── Top Navigation Bar ───────────────────────────────
        let nav_frame = egui::Frame::default()
            .inner_margin(egui::Margin::symmetric(24, 16))
            .fill(ctx.style().visuals.window_fill);
        egui::TopBottomPanel::top("nav_bar")
            .frame(nav_frame)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("❖ Tracker").size(20.0).strong());
                    ui.add_space(24.0);
                    if ui.selectable_label(matches!(self.screen, Screen::Dashboard), "Dashboard (Ctrl+D)").clicked() {
                        self.screen = Screen::Dashboard;
                        self.refresh_projects();
                    }
                    if ui.selectable_label(matches!(self.screen, Screen::Search), "Search (Ctrl+F)").clicked() {
                        self.screen = Screen::Search;
                        self.search_query.clear();
                        self.search_results.clear();
                        self.search_performed = false;
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            egui::RichText::new(format!("{} active", self.projects.len()))
                                .small()
                                .weak(),
                        );
                    });
                });
            });

        // ── Main Content ─────────────────────────────────────
        let central_frame = egui::Frame::default()
            .inner_margin(egui::Margin::same(24))
            .fill(ctx.style().visuals.window_fill);
        egui::CentralPanel::default().frame(central_frame).show(ctx, |ui| {
            match self.screen.clone() {
                Screen::Dashboard => self.ui_dashboard(ui),
                Screen::ProjectDetail(id) => self.ui_project_detail(ui, id),
                Screen::Search => self.ui_search(ui),
            }
        });
    }
}

// ── Dashboard Screen ─────────────────────────────────────────

impl App {
    fn ui_dashboard(&mut self, ui: &mut egui::Ui) {
        ui.add_space(12.0);
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Dashboard").size(32.0).strong());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("✨ New Project").clicked() {
                    self.show_create_project = true;
                }
                ui.add_space(8.0);
                if ui.button("📥 Import").clicked() {
                    if let Some(path) = rfd::FileDialog::new().add_filter("JSON", &["json"]).pick_file() {
                        if let Ok(json) = std::fs::read_to_string(path) {
                            let conn = self.conn.lock().unwrap();
                            let _ = import_all_from_json(&conn, &json);
                            drop(conn);
                            self.needs_refresh = true;
                        }
                    }
                }
                ui.add_space(8.0);
                if ui.button("📤 Export").clicked() {
                    if let Some(path) = rfd::FileDialog::new().add_filter("JSON", &["json"]).save_file() {
                        let conn = self.conn.lock().unwrap();
                        if let Ok(json) = export_all_to_json(&conn) {
                            let _ = std::fs::write(path, json);
                        }
                    }
                }
            });
        });
        ui.add_space(20.0);

        // Create project modal
        if self.show_create_project {
            egui::Window::new("Create Project")
                .collapsible(false)
                .resizable(false)
                .show(ui.ctx(), |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Name:");
                        ui.text_edit_singleline(&mut self.new_project_name);
                    });
                    ui.horizontal(|ui| {
                        ui.label("Goal:");
                        ui.text_edit_multiline(&mut self.new_project_goal);
                    });
                    ui.horizontal(|ui| {
                        if ui.button("Create").clicked() && !self.new_project_name.trim().is_empty()
                        {
                            let conn = self.conn.lock().unwrap();
                            let _ = create_project(
                                &conn,
                                self.new_project_name.trim(),
                                self.new_project_goal.trim(),
                            );
                            drop(conn);
                            self.new_project_name.clear();
                            self.new_project_goal.clear();
                            self.show_create_project = false;
                            self.refresh_projects();
                        }
                        if ui.button("Cancel").clicked() {
                            self.show_create_project = false;
                            self.new_project_name.clear();
                            self.new_project_goal.clear();
                        }
                    });
                });
        }

        if self.projects.is_empty() {
            ui.vertical_centered(|ui| {
                ui.add_space(40.0);
                ui.label(
                    egui::RichText::new("No projects yet. Create one to get started!")
                        .size(16.0)
                        .weak(),
                );
            });
            return;
        }

        // Project list
        egui::ScrollArea::vertical().auto_shrink([false; 2]).show(ui, |ui| {
            for project in self.projects.clone() {
                ui.push_id(project.id, |ui| {
                    egui::Frame::NONE
                        .fill(ui.visuals().panel_fill)
                        .corner_radius(egui::CornerRadius::same(16))
                        .stroke(ui.visuals().widgets.noninteractive.bg_stroke)
                        .inner_margin(egui::Margin::same(24))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.vertical(|ui| {
                                    if ui
                                        .add(
                                            egui::Label::new(
                                                egui::RichText::new(&project.name)
                                                    .size(24.0)
                                                    .strong(),
                                            )
                                            .sense(egui::Sense::click()),
                                        )
                                        .clicked()
                                    {
                                        self.navigate_to_project(project.id);
                                    }
                                    if !project.goal.is_empty() {
                                        ui.add_space(6.0);
                                        ui.label(
                                            egui::RichText::new(&project.goal).size(15.0).weak(),
                                        );
                                    }
                                });
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        ui.label(
                                            egui::RichText::new(&project.updated_at)
                                                .small()
                                                .weak(),
                                        );
                                        ui.add_space(20.0);
                                        if !project.current_task.is_empty() {
                                            ui.label(
                                                egui::RichText::new(format!("Current: {}", &project.current_task))
                                                    .color(ui.visuals().hyperlink_color),
                                            );
                                            ui.add_space(20.0);
                                        }
                                        // Progress bar
                                        let progress = project.progress as f32 / 100.0;
                                        ui.add(
                                            egui::ProgressBar::new(progress)
                                                .text(format!("{}%", project.progress))
                                                .desired_width(120.0)
                                        );
                                    },
                                );
                            });
                        });
                    ui.add_space(16.0);
                });
            }
        });
    }

    // ── Project Detail Screen ────────────────────────────────

    fn ui_project_detail(&mut self, ui: &mut egui::Ui, project_id: i64) {
        let project = match &self.current_project {
            Some(p) => p.clone(),
            None => {
                ui.label("Project not found.");
                return;
            }
        };

        // Header with back button
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if ui.button("⬅").on_hover_text("Back to Dashboard").clicked() {
                self.screen = Screen::Dashboard;
                self.refresh_projects();
            }
            ui.add_space(12.0);
            ui.label(egui::RichText::new(&project.name).size(30.0).strong());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("✏ Edit").clicked() {
                    self.edit_project_name = project.name.clone();
                    self.edit_project_goal = project.goal.clone();
                    self.edit_project_progress = project.progress;
                    self.edit_project_current_task = project.current_task.clone();
                    self.show_edit_project = !self.show_edit_project;
                }
                ui.add_space(8.0);
                if ui
                    .button(egui::RichText::new("Archive 🗑").color(egui::Color32::from_rgb(210, 130, 130)))
                    .clicked()
                {
                    let conn = self.conn.lock().unwrap();
                    let _ = archive_project(&conn, project_id);
                    drop(conn);
                    self.screen = Screen::Dashboard;
                    self.refresh_projects();
                    return;
                }
                ui.add_space(16.0);
                let progress = project.progress as f32 / 100.0;
                ui.add(
                    egui::ProgressBar::new(progress)
                        .text(format!("{}%", project.progress))
                        .desired_width(120.0),
                );
            });
        });
        ui.add_space(20.0);

        // Edit project modal
        if self.show_edit_project {
            egui::Window::new("Edit Project")
                .collapsible(false)
                .resizable(false)
                .show(ui.ctx(), |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Name:");
                        ui.text_edit_singleline(&mut self.edit_project_name);
                    });
                    ui.horizontal(|ui| {
                        ui.label("Goal:");
                        ui.text_edit_multiline(&mut self.edit_project_goal);
                    });
                    ui.horizontal(|ui| {
                        ui.label("Progress:");
                        ui.add(egui::Slider::new(&mut self.edit_project_progress, 0..=100).suffix("%"));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Current Task:");
                        ui.text_edit_singleline(&mut self.edit_project_current_task);
                    });
                    ui.horizontal(|ui| {
                        if ui.button("Save").clicked() {
                            let conn = self.conn.lock().unwrap();
                            let _ = update_project(
                                &conn,
                                project_id,
                                self.edit_project_name.trim(),
                                self.edit_project_goal.trim(),
                                &project.status,
                                self.edit_project_progress,
                                self.edit_project_current_task.trim(),
                            );
                            drop(conn);
                            self.show_edit_project = false;
                            self.refresh_project_detail(project_id);
                            self.refresh_projects();
                        }
                        if ui.button("Cancel").clicked() {
                            self.show_edit_project = false;
                        }
                    });
                });
        }

        // ── Context Recovery Section ─────────────────────────
        if let Some(log) = &self.latest_log {
            egui::Frame::NONE
                .fill(ui.visuals().panel_fill)
                .corner_radius(egui::CornerRadius::same(12))
                .stroke(ui.visuals().widgets.noninteractive.bg_stroke)
                .inner_margin(egui::Margin::same(16))
                .show(ui, |ui| {
                    ui.label(egui::RichText::new("Context Recovery").size(16.0).strong().color(ui.visuals().hyperlink_color));
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.label("Last log:");
                        ui.label(egui::RichText::new(&log.date).weak());
                    });
                    if !log.notes.is_empty() {
                        ui.label(format!("Notes: {}", &log.notes));
                    }
                    if !log.next_action.is_empty() {
                        ui.add_space(4.0);
                        ui.label(
                            egui::RichText::new(format!("Next Action: {}", &log.next_action))
                                .strong()
                        );
                    }
                });
            ui.add_space(16.0);
        }

        egui::ScrollArea::vertical().show(ui, |ui| {
            // ── Tasks Section ────────────────────────────────
            ui.collapsing(
                egui::RichText::new(format!(
                    "Tasks ({}/{})",
                    self.tasks.iter().filter(|t| t.status == "done").count(),
                    self.tasks.len()
                ))
                .heading(),
                |ui| {
                    // New task input
                    ui.horizontal(|ui| {
                        ui.label("New task:");
                        let response = ui.text_edit_singleline(&mut self.new_task_desc);
                        if (ui.button("Add").clicked()
                            || (response.lost_focus()
                                && ui.input(|i| i.key_pressed(egui::Key::Enter))))
                            && !self.new_task_desc.trim().is_empty()
                        {
                            let conn = self.conn.lock().unwrap();
                            let _ = create_task(&conn, project_id, self.new_task_desc.trim());
                            drop(conn);
                            self.new_task_desc.clear();
                            self.refresh_project_detail(project_id);
                        }
                    });
                    ui.separator();

                    let tasks_clone = self.tasks.clone();
                    for task in &tasks_clone {
                        ui.horizontal(|ui| {
                            let checked = task.status == "done";
                            if ui.checkbox(&mut checked.clone(), "").changed() {
                                let conn = self.conn.lock().unwrap();
                                let _ = toggle_task_status(&conn, task.id);
                                drop(conn);
                                self.needs_refresh = true;
                            }
                            let text = if checked {
                                egui::RichText::new(&task.description).strikethrough().weak()
                            } else {
                                egui::RichText::new(&task.description)
                            };
                            ui.label(text);
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if ui
                                        .button(egui::RichText::new("x").small().weak())
                                        .clicked()
                                    {
                                        let conn = self.conn.lock().unwrap();
                                        let _ = delete_task(&conn, task.id);
                                        drop(conn);
                                        self.needs_refresh = true;
                                    }
                                },
                            );
                        });
                    }
                },
            );

            ui.add_space(8.0);

            // ── Daily Log Section ────────────────────────────
            ui.collapsing(
                egui::RichText::new(format!("Daily Logs ({})", self.logs.len())).heading(),
                |ui| {
                    // New log form
                    egui::Frame::group(ui.style())
                        .inner_margin(egui::Margin::same(8))
                        .show(ui, |ui| {
                            ui.label(egui::RichText::new("New Log Entry").strong());
                            ui.horizontal(|ui| {
                                ui.label("Notes:");
                                ui.text_edit_multiline(&mut self.new_log_notes);
                            });
                            ui.horizontal(|ui| {
                                ui.label("Next:");
                                ui.text_edit_singleline(&mut self.new_log_next_action);
                            });
                            if ui.button("Add Log").clicked()
                                && !self.new_log_notes.trim().is_empty()
                            {
                                let conn = self.conn.lock().unwrap();
                                let _ = create_log(
                                    &conn,
                                    project_id,
                                    self.new_log_notes.trim(),
                                    self.new_log_next_action.trim(),
                                );
                                drop(conn);
                                self.new_log_notes.clear();
                                self.new_log_next_action.clear();
                                self.refresh_project_detail(project_id);
                            }
                        });
                    ui.add_space(4.0);

                    let logs_clone = self.logs.clone();
                    for log in &logs_clone {
                        egui::Frame::group(ui.style())
                            .inner_margin(egui::Margin::same(6))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new(&log.date).strong());
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            if ui.button("x").clicked() {
                                                let conn = self.conn.lock().unwrap();
                                                let _ = delete_log(&conn, log.id);
                                                drop(conn);
                                                self.needs_refresh = true;
                                            }
                                        },
                                    );
                                });
                                if !log.notes.is_empty() {
                                    ui.label(&log.notes);
                                }
                                if !log.next_action.is_empty() {
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "Next: {}",
                                            &log.next_action
                                        ))
                                        .italics()
                                        .weak(),
                                    );
                                }
                            });
                        ui.add_space(2.0);
                    }
                },
            );
        });
    }

    // ── Search Screen ────────────────────────────────────────

    fn ui_search(&mut self, ui: &mut egui::Ui) {
        ui.heading("Search");
        ui.separator();
        ui.horizontal(|ui| {
            let response = ui.text_edit_singleline(&mut self.search_query);
            if (ui.button("Search").clicked()
                || (response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))))
                && !self.search_query.trim().is_empty() {
                    let conn = self.conn.lock().unwrap();
                    self.search_results =
                        search_all(&conn, self.search_query.trim()).unwrap_or_default();
                    self.search_performed = true;
                }
        });

        if self.search_performed {
            ui.separator();
            if self.search_results.is_empty() {
                ui.label("No results found.");
            } else {
                ui.label(format!("{} result(s) found", self.search_results.len()));
                ui.add_space(4.0);

                egui::ScrollArea::vertical().show(ui, |ui| {
                    let results_clone = self.search_results.clone();
                    for result in &results_clone {
                        match result {
                            SearchResult::ProjectResult(p) => {
                                egui::Frame::group(ui.style())
                                    .inner_margin(egui::Margin::same(6))
                                    .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            ui.label(
                                                egui::RichText::new("[Project]")
                                                    .small()
                                                    .color(egui::Color32::from_rgb(100, 150, 255)),
                                            );
                                            if ui
                                                .add(
                                                    egui::Label::new(
                                                        egui::RichText::new(&p.name).strong(),
                                                    )
                                                    .sense(egui::Sense::click()),
                                                )
                                                .clicked()
                                            {
                                                self.navigate_to_project(p.id);
                                            }
                                        });
                                        if !p.goal.is_empty() {
                                            ui.label(
                                                egui::RichText::new(&p.goal).weak().italics(),
                                            );
                                        }
                                    });
                            }
                            SearchResult::TaskResult(t) => {
                                egui::Frame::group(ui.style())
                                    .inner_margin(egui::Margin::same(6))
                                    .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            ui.label(
                                                egui::RichText::new("[Task]")
                                                    .small()
                                                    .color(egui::Color32::from_rgb(100, 255, 150)),
                                            );
                                            if ui
                                                .add(
                                                    egui::Label::new(
                                                        egui::RichText::new(&t.description)
                                                            .strong(),
                                                    )
                                                    .sense(egui::Sense::click()),
                                                )
                                                .clicked()
                                            {
                                                self.navigate_to_project(t.project_id);
                                            }
                                            ui.label(
                                                egui::RichText::new(format!("({})", t.status))
                                                    .weak(),
                                            );
                                        });
                                    });
                            }
                            SearchResult::LogResult(l) => {
                                egui::Frame::group(ui.style())
                                    .inner_margin(egui::Margin::same(6))
                                    .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            ui.label(
                                                egui::RichText::new("[Log]")
                                                    .small()
                                                    .color(egui::Color32::from_rgb(255, 200, 100)),
                                            );
                                            if ui
                                                .add(
                                                    egui::Label::new(
                                                        egui::RichText::new(&l.date).strong(),
                                                    )
                                                    .sense(egui::Sense::click()),
                                                )
                                                .clicked()
                                            {
                                                self.navigate_to_project(l.project_id);
                                            }
                                        });
                                        if !l.notes.is_empty() {
                                            ui.label(&l.notes);
                                        }
                                    });
                            }
                        }
                        ui.add_space(2.0);
                    }
                });
            }
        }
    }
}

// ── Main ─────────────────────────────────────────────────────

fn main() -> eframe::Result {
    let db_path = "project_tracker.sqlite";

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([900.0, 620.0])
            .with_min_inner_size([600.0, 400.0])
            .with_title("Project Tracker"),
        ..Default::default()
    };

    eframe::run_native(
        "Project Tracker",
        options,
        Box::new(move |cc| {
            // Apply Cozy Chic Pastel Dark Theme
            let mut style = (*cc.egui_ctx.style()).clone();
            
            // Spacing System
            style.spacing.item_spacing = egui::Vec2::new(16.0, 16.0);
            style.spacing.button_padding = egui::Vec2::new(20.0, 10.0);
            style.spacing.window_margin = egui::Margin::same(32);

            // Color Palette - Cozy Modern Minimalist Pastels
            let app_bg = egui::Color32::from_rgb(25, 24, 28);       // Warm dark, slightly mauvey 
            let panel_bg = egui::Color32::from_rgb(33, 31, 37);     // Elevated element
            let editor_bg = egui::Color32::from_rgb(19, 18, 22);    // Inputs
            let text_primary = egui::Color32::from_rgb(240, 235, 230); // Soft milk text
            let text_secondary = egui::Color32::from_rgb(165, 160, 168); // Muted texts
            
            // Accent Color: Sophisticated Pastel Dusty Rose
            let accent_pastel = egui::Color32::from_rgb(198, 165, 188); 
            let accent_hover = egui::Color32::from_rgb(218, 185, 208); 
            let divider = egui::Color32::from_rgb(48, 45, 53);      // Subtle separation

            let mut visuals = egui::Visuals::dark();
            visuals.window_fill = app_bg;
            visuals.panel_fill = panel_bg;
            visuals.faint_bg_color = panel_bg;
            visuals.extreme_bg_color = editor_bg;
            visuals.code_bg_color = editor_bg;
            visuals.hyperlink_color = accent_pastel;
            visuals.override_text_color = Some(text_primary);

            // Widgets
            visuals.widgets.noninteractive.bg_fill = app_bg;
            visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, divider);
            visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, text_secondary);

            visuals.widgets.inactive.bg_fill = panel_bg;
            visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, divider);
            visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, text_primary);
            visuals.widgets.inactive.corner_radius = egui::CornerRadius::same(12);

            visuals.widgets.active.bg_fill = accent_pastel; 
            visuals.widgets.active.bg_stroke = egui::Stroke::new(1.0, accent_pastel);
            visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0, egui::Color32::BLACK);
            visuals.widgets.active.corner_radius = egui::CornerRadius::same(12);

            visuals.widgets.hovered.bg_fill = accent_hover; 
            visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, accent_hover);
            visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, egui::Color32::BLACK);
            visuals.widgets.hovered.corner_radius = egui::CornerRadius::same(14);

            // Selection
            visuals.selection.bg_fill = accent_pastel.linear_multiply(0.2);
            visuals.selection.stroke = egui::Stroke::new(1.5, accent_pastel);
            
            // Typography 
            use egui::{FontId, FontFamily};
            style.text_styles.insert(egui::TextStyle::Heading, FontId::new(28.0, FontFamily::Proportional));
            style.text_styles.insert(egui::TextStyle::Body, FontId::new(16.0, FontFamily::Proportional));
            style.text_styles.insert(egui::TextStyle::Small, FontId::new(14.0, FontFamily::Proportional));
            style.text_styles.insert(egui::TextStyle::Monospace, FontId::new(14.0, FontFamily::Monospace));
            style.text_styles.insert(egui::TextStyle::Button, FontId::new(16.0, FontFamily::Proportional));

            cc.egui_ctx.set_visuals(visuals);
            cc.egui_ctx.set_style(style);

            Ok(Box::new(App::new(db_path)))
        }),
    )
}
