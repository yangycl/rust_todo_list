#![windows_subsystem = "windows"]

use eframe::egui;
use rfd::{FileDialog, MessageButtons, MessageDialog};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use image;
use chrono::{DateTime, Local};

#[derive(Serialize, Deserialize, Clone)]
struct TodoItemOld {
    id: usize,
    title: String,
}

#[derive(Serialize, Deserialize, Clone)]
struct TodoItem {
    id: usize,
    title: String,
    created_at: std::time::SystemTime,
}

struct TodoApp {
    todos: Vec<TodoItem>,
    new_todo_input: String,
    next_id: usize,
    pending_delete: Option<usize>,
    export_status: Option<String>,
    search_query: String,
    sort_newest_first: bool,
    editing: Option<usize>,
    editing_input: String,
    confirm_exit: bool,
    exit_dialog_opened: bool,
}

impl Default for TodoApp {
    fn default() -> Self {
        Self {
            todos: vec![],
            new_todo_input: String::new(),
            next_id: 1,
            pending_delete: None,
            export_status: None,
            search_query: String::new(),
            sort_newest_first: true,
            editing: None,
            editing_input: String::new(),
            confirm_exit: false,
            exit_dialog_opened: false,
        }
    }
}

impl eframe::App for TodoApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let close_requested = ctx.input(|i| i.viewport().close_requested());
        if close_requested && !self.confirm_exit {
            self.confirm_exit = true;
        }


        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("📝 Todo List");

            // Input a new todo
            ui.horizontal(|ui| {
                ui.label("New Todo:");
                ui.text_edit_singleline(&mut self.new_todo_input);
                if ui.button("Add").clicked() && !self.new_todo_input.is_empty() {
                    self.todos.push(TodoItem {
                        id: self.next_id,
                        title: self.new_todo_input.clone(),
                        created_at: std::time::SystemTime::now(),
                    });
                    self.next_id += 1;
                    self.new_todo_input.clear();
                }
            });

            ui.horizontal(|ui| {
                ui.label("Search:");
                ui.text_edit_singleline(&mut self.search_query);
                if ui.button("Clear Search").clicked() {
                    self.search_query.clear();
                    self.export_status = None;
                }
                if ui.button(if self.sort_newest_first { "Sort: Newest first" } else { "Sort: Oldest first" }).clicked() {
                    self.sort_newest_first = !self.sort_newest_first;
                    if self.sort_newest_first {
                        self.todos.sort_by(|a, b| b.created_at.cmp(&a.created_at));
                    } else {
                        self.todos.sort_by(|a, b| a.created_at.cmp(&b.created_at));
                    }
                }
            });

            ui.separator();

            // Todo list
            let query = self.search_query.to_lowercase();
            for (index, todo) in self.todos.iter().enumerate().filter(|(_, todo)| {
                self.search_query.is_empty() || todo.title.to_lowercase().contains(&query)
            }) {
                ui.horizontal(|ui| {
                    ui.label(&todo.title);
                    
                    // Show creation date (date only)
                    let dt: DateTime<Local> = todo.created_at.into();
                    ui.label(format!("Created on: {}", dt.format("%Y-%m-%d")));
                    
                    if ui.button("Delete").clicked() {
                        self.pending_delete = Some(index);
                    }
                    if ui.button("Edit").clicked() {
                        self.editing = Some(index);
                        self.editing_input = todo.title.clone();
                    }
                });
            }

            if let Some(index) = self.editing {
                egui::Window::new("Edit Todo")
                    .collapsible(false)
                    .resizable(false)
                    .show(ctx, |ui| {
                        ui.label("Edit Todo:");
                        ui.text_edit_singleline(&mut self.editing_input);
                        ui.horizontal(|ui| {
                            if ui.button("Save").clicked() {
                                if let Some(todo) = self.todos.get_mut(index) {
                                    todo.title = self.editing_input.clone();
                                }
                                self.editing = None;
                                self.editing_input.clear();
                            }
                            if ui.button("Cancel").clicked() {
                                self.editing = None;
                                self.editing_input.clear();
                            }
                        });
                    });
            }

            if let Some(index) = self.pending_delete {
                egui::Window::new("Confirm delete")
                    .collapsible(false)
                    .resizable(false)
                    .show(ctx, |ui| {
                        ui.label("Are you sure you want to delete this todo?");
                        ui.horizontal(|ui| {
                            if ui.button("Yes").clicked() {
                                if index < self.todos.len() {
                                    self.todos.remove(index);
                                }
                                self.pending_delete = None;
                            }
                            if ui.button("Cancel").clicked() {
                                self.pending_delete = None;
                            }
                        });
                    });
            }

            // Statistics
            ui.separator();
            let total = self.todos.len();
            ui.label(format!("Total: {}", total));
            if let Some(status) = &self.export_status {
                ui.label(status);
            }
            ui.horizontal(|ui:&mut egui::Ui| {
                if ui.button("Clear All").clicked() {
                    self.todos.clear();
                    self.export_status = None;
                }
                if ui.button("Export").clicked() {
                    if let Some(folder) = FileDialog::new().set_title("Select folder to export").pick_folder() {
                        let file_path = folder.join("todo_export.json");
                        match std::fs::write(&file_path, serde_json::to_string_pretty(&self.todos).unwrap()) {
                            Ok(_) => {
                                self.export_status = Some(format!("Exported to: {}", file_path.display()));
                            }
                            Err(err) => {
                                self.export_status = Some(format!("Export failed: {}", err));
                            }
                        }
                    } else {
                        self.export_status = Some("Export canceled".to_owned());
                    }
                }
                if ui.button("Import").clicked() {
                    if let Some(file_path) = FileDialog::new().set_title("Select JSON file to import").add_filter("JSON", &["json"]).pick_file() {
                        match std::fs::read_to_string(&file_path) {
                            Ok(data) => {
                                // Try new format first
                                match serde_json::from_str::<Vec<TodoItem>>(&data) {
                                    Ok(imported) => {
                                        self.todos = imported;
                                        self.next_id = self.todos.iter().map(|t| t.id).max().unwrap_or(0) + 1;
                                        self.export_status = Some(format!("Imported: {} ({} items)", file_path.display(), self.todos.len()));
                                    }
                                    Err(_) => {
                                        // Try old format
                                        match serde_json::from_str::<Vec<TodoItemOld>>(&data) {
                                            Ok(imported_old) => {
                                                self.todos = imported_old.into_iter().map(|old| TodoItem {
                                                    id: old.id,
                                                    title: old.title,
                                                    created_at: std::time::SystemTime::now(),
                                                }).collect();
                                                self.next_id = self.todos.iter().map(|t| t.id).max().unwrap_or(0) + 1;
                                                self.export_status = Some(format!("Imported old format: {} ({} items)", file_path.display(), self.todos.len()));
                                            }
                                            Err(err) => {
                                                self.export_status = Some(format!("Import failed (JSON parse): {}", err));
                                            }
                                        }
                                    }
                                }
                            }
                            Err(err) => {
                                self.export_status = Some(format!("Import failed (read file): {}", err));
                            }
                        }
                    } else {
                        self.export_status = Some("Import canceled".to_owned());
                    }
                }
            });
        });
        if self.confirm_exit {
            if !self.exit_dialog_opened {
                self.exit_dialog_opened = true;    
                let should_export = MessageDialog::new()
                    .set_title("Export on Exit")
                    .set_description("Do you want to export your todos before closing?")
                    .set_buttons(MessageButtons::YesNo)
                    .show();
                if should_export {
                    if let Some(folder) = FileDialog::new().set_title("Select folder to export").pick_folder() {
                        let file_path = folder.join("todo_export.json");
                        match std::fs::write(&file_path, serde_json::to_string_pretty(&self.todos).unwrap()) {
                            Ok(_) => {
                                MessageDialog::new()
                                    .set_title("Export Successful")
                                    .set_description(&format!("Exported to: {}", file_path.display()))
                                    .set_buttons(MessageButtons::Ok)
                                    .show();
                            }
                            Err(err) => {
                                MessageDialog::new()
                                    .set_title("Export Failed")
                                    .set_description(&format!("Failed to export: {}", err))
                                    .set_buttons(MessageButtons::Ok)
                                    .show();
                            }
                        }
                    }
                }
                ctx.send_viewport_cmd(eframe::egui::ViewportCommand::Close);
            };
        };
    }

}

fn load_icon() -> Option<Arc<egui::IconData>> {
    let icon_bytes = include_bytes!("../icon.png");
    let image = image::load_from_memory(icon_bytes).ok()?;
    let rgba = image.to_rgba8();
    let width = rgba.width();
    let height = rgba.height();
    let bytes = rgba.into_raw();
    Some(Arc::new(egui::IconData {
        rgba: bytes,
        width,
        height,
    }))
}

fn main() -> Result<(), eframe::Error> {
    let mut options = eframe::NativeOptions::default();
    options.viewport.icon = load_icon();
    eframe::run_native(
        "Todo List App",
        options,
        Box::new(|_cc| Box::new(TodoApp::default())),
    )?;
    Ok(())
}
