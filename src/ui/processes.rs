use egui::{
    pos2, vec2, Align, Color32, CornerRadius, Frame, Layout, Rect, RichText, ScrollArea, Sense,
    Stroke, Ui, UiBuilder,
};

use crate::models::{ProcessFilterCategory, ProcessInfo, ProcessSortColumn, SortDirection};
use crate::ui::components::ConfirmAction;
use crate::ui::helpers::{
    process_status_color, render_search_icon, render_status_dot, render_subtle_pill,
    render_tinted_pill,
};
use crate::ui::theme::AppTheme;
use crate::utils::format::format_bytes;

fn cell_ui<R>(
    ui: &mut Ui,
    rect: Rect,
    layout: Layout,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> R {
    let mut child = ui.new_child(UiBuilder::new().max_rect(rect).layout(layout));
    add_contents(&mut child)
}

pub struct ProcessViewState {
    pub search_query: String,
    pub filter_category: ProcessFilterCategory,
    pub sort_column: ProcessSortColumn,
    pub sort_direction: SortDirection,
    pub selected_pid: Option<i32>,
}

impl Default for ProcessViewState {
    fn default() -> Self {
        Self {
            search_query: String::new(),
            filter_category: ProcessFilterCategory::All,
            sort_column: ProcessSortColumn::Memory,
            sort_direction: SortDirection::Descending,
            selected_pid: None,
        }
    }
}

pub enum ProcessActionRequest {
    RequestQuit(ProcessInfo, ConfirmAction),
    Inspect(ProcessInfo),
}

pub fn render_processes(
    ui: &mut Ui,
    processes: &[ProcessInfo],
    state: &mut ProcessViewState,
    theme: &AppTheme,
) -> Option<ProcessActionRequest> {
    let mut action_request = None;
    let avail_width = (ui.available_width() - 4.0).max(280.0);

    ui.add_space(4.0);

    // 1. Top Toolbar: Header Title, Search Bar, and Process Count (macOS HIG Toolbar)
    ui.horizontal(|ui| {
        ui.label(
            RichText::new("Processes")
                .size(20.0)
                .strong()
                .color(theme.text_primary),
        );

        ui.add_space(8.0);

        render_subtle_pill(
            ui,
            &format!("{} total", processes.len()),
            theme.text_secondary,
            theme,
            11.0,
        );

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            // Apple-style Search Box
            let search_w = (avail_width * 0.32).clamp(180.0, 320.0);
            egui::Frame::new()
                .fill(theme.subtle_bg)
                .corner_radius(CornerRadius::same(7))
                .stroke(Stroke::new(1.0, theme.border_subtle))
                .inner_margin(egui::Margin::symmetric(8, 4))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        render_search_icon(ui, theme.text_secondary, 12.0);
                        ui.add_space(4.0);
                        let search_edit = egui::TextEdit::singleline(&mut state.search_query)
                            .id_salt("process_search_input")
                            .hint_text("Search name, PID, path... (⌘F)")
                            .desired_width(search_w - 48.0)
                            .frame(Frame::NONE);
                        ui.add(search_edit);

                        if !state.search_query.is_empty()
                            && ui
                                .add(
                                    egui::Button::new(
                                        RichText::new("✕").size(10.5).color(theme.text_secondary),
                                    )
                                    .frame(false),
                                )
                                .clicked()
                        {
                            state.search_query.clear();
                        }
                    });
                });
        });
    });

    ui.add_space(8.0);

    // 2. Category Filter Chips (macOS Segmented Control Capsule - Horizontal)
    let categories = [
        (ProcessFilterCategory::All, "All"),
        (ProcessFilterCategory::Applications, "Applications"),
        (ProcessFilterCategory::Background, "Background"),
        (ProcessFilterCategory::System, "System"),
        (ProcessFilterCategory::HighMemory, "High Memory (>500MB)"),
        (ProcessFilterCategory::HighEnergy, "Energy Hogs (>10% CPU)"),
    ];

    egui::Frame::new()
        .fill(theme.subtle_bg)
        .corner_radius(CornerRadius::same(8))
        .stroke(Stroke::new(1.0, theme.border_subtle))
        .inner_margin(egui::Margin::symmetric(3, 3))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing = vec2(2.0, 0.0);
                for (cat, label) in categories {
                    let is_selected = state.filter_category == cat;
                    let (bg_color, text_color) = if is_selected {
                        (theme.accent_color, Color32::WHITE)
                    } else {
                        (Color32::TRANSPARENT, theme.text_secondary)
                    };

                    let chip_btn = egui::Button::new(
                        RichText::new(label).size(11.5).strong().color(text_color),
                    )
                    .fill(bg_color)
                    .corner_radius(CornerRadius::same(6));

                    if ui.add(chip_btn).clicked() {
                        state.filter_category = cat;
                    }
                }
            });
        });

    ui.add_space(8.0);

    // 3. Selected Process Action Banner (macOS Inspector Strip)
    if let Some(selected_pid) = state.selected_pid {
        if let Some(proc) = processes.iter().find(|p| p.pid == selected_pid) {
            egui::Frame::new()
                .fill(theme.card_bg)
                .corner_radius(CornerRadius::same(8))
                .stroke(Stroke::new(1.0, theme.accent_color.linear_multiply(0.6)))
                .inner_margin(egui::Margin::symmetric(14, 10))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.horizontal(|ui| {
                                render_status_dot(ui, process_status_color(proc, theme), 4.0);
                                ui.add_space(4.0);
                                ui.label(
                                    RichText::new(&proc.name)
                                        .size(14.5)
                                        .strong()
                                        .color(theme.text_primary),
                                );
                                render_subtle_pill(
                                    ui,
                                    &format!("PID {}", proc.pid),
                                    theme.text_secondary,
                                    theme,
                                    11.0,
                                );
                                render_tinted_pill(
                                    ui,
                                    &format_bytes(proc.memory_bytes),
                                    theme.accent_color,
                                    11.0,
                                );
                                if let Some(cpu) = proc.cpu_usage {
                                    render_subtle_pill(
                                        ui,
                                        &format!("{:.1}% CPU", cpu),
                                        theme.text_primary,
                                        theme,
                                        11.0,
                                    );
                                }
                                render_subtle_pill(
                                    ui,
                                    proc.display_type(),
                                    theme.text_secondary,
                                    theme,
                                    10.5,
                                );
                                render_subtle_pill(
                                    ui,
                                    proc.architecture.display_name(),
                                    theme.text_secondary,
                                    theme,
                                    10.5,
                                );
                            });

                            if let Some(ref path) = proc.path {
                                ui.add_space(3.0);
                                ui.label(
                                    RichText::new(format!("Path: {}", path))
                                        .size(10.5)
                                        .color(theme.text_secondary),
                                );
                            }
                        });

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            // Deselect button
                            if ui
                                .add(
                                    egui::Button::new(
                                        RichText::new("✕").size(11.0).color(theme.text_secondary),
                                    )
                                    .frame(false),
                                )
                                .clicked()
                            {
                                state.selected_pid = None;
                            }

                            ui.add_space(8.0);

                            if proc.is_protected {
                                render_tinted_pill(
                                    ui,
                                    "Protected System Process",
                                    theme.warning_color,
                                    11.0,
                                );
                            } else {
                                if ui
                                    .add(
                                        theme.critical_button(
                                            RichText::new("Force Quit").size(11.5),
                                        ),
                                    )
                                    .clicked()
                                {
                                    action_request = Some(ProcessActionRequest::RequestQuit(
                                        proc.clone(),
                                        ConfirmAction::ForceQuit,
                                    ));
                                }

                                ui.add_space(4.0);

                                if ui
                                    .add(theme.subtle_button(RichText::new("Quit").size(11.5)))
                                    .clicked()
                                {
                                    action_request = Some(ProcessActionRequest::RequestQuit(
                                        proc.clone(),
                                        ConfirmAction::SafeQuit,
                                    ));
                                }
                            }

                            ui.add_space(4.0);

                            if ui
                                .add(
                                    theme
                                        .subtle_button(RichText::new("Inspect Details").size(11.5)),
                                )
                                .clicked()
                            {
                                action_request = Some(ProcessActionRequest::Inspect(proc.clone()));
                            }
                        });
                    });
                });

            ui.add_space(6.0);
        }
    }

    // 4. Responsive Column Geometry Calculation (Full 100% available width)
    let show_type = avail_width >= 620.0;
    let pad_x = 12.0;
    let col_spacing = 14.0;
    let pid_w = 68.0;
    let mem_w = 105.0;
    let cpu_w = 72.0;
    let type_w = if show_type { 95.0 } else { 0.0 };

    let num_spacings = if show_type { 4.0 } else { 3.0 };
    let fixed_total = pid_w + mem_w + cpu_w + type_w + (col_spacing * num_spacings) + (pad_x * 2.0);
    let name_w = (avail_width - fixed_total).max(160.0);

    let name_x = pad_x;
    let pid_x = name_x + name_w + col_spacing;
    let mem_x = pid_x + pid_w + col_spacing;
    let cpu_x = mem_x + mem_w + col_spacing;
    let type_x = cpu_x + cpu_w + col_spacing;

    // 5. Process Table Header (Apple HIG Table Header Bar)
    let (header_rect, _) = ui.allocate_exact_size(vec2(avail_width, 28.0), Sense::hover());
    ui.painter().rect(
        header_rect,
        CornerRadius::same(6),
        theme.card_bg,
        Stroke::new(1.0, theme.border_subtle),
        egui::StrokeKind::Inside,
    );

    // Name column header
    let name_cell = Rect::from_min_size(
        pos2(header_rect.left() + name_x, header_rect.min.y),
        vec2(name_w, 28.0),
    );
    cell_ui(ui, name_cell, Layout::left_to_right(Align::Center), |ui| {
        let arrow = if state.sort_column == ProcessSortColumn::Name {
            match state.sort_direction {
                SortDirection::Ascending => " ▲",
                SortDirection::Descending => " ▼",
            }
        } else {
            ""
        };
        let color = if state.sort_column == ProcessSortColumn::Name {
            theme.accent_color
        } else {
            theme.text_secondary
        };
        if ui
            .add(
                egui::Button::new(
                    RichText::new(format!("Process Name{}", arrow))
                        .size(11.5)
                        .strong()
                        .color(color),
                )
                .frame(false),
            )
            .clicked()
        {
            if state.sort_column == ProcessSortColumn::Name {
                state.sort_direction = match state.sort_direction {
                    SortDirection::Ascending => SortDirection::Descending,
                    SortDirection::Descending => SortDirection::Ascending,
                };
            } else {
                state.sort_column = ProcessSortColumn::Name;
                state.sort_direction = SortDirection::Ascending;
            }
        }
    });

    // PID column header
    let pid_cell = Rect::from_min_size(
        pos2(header_rect.left() + pid_x, header_rect.min.y),
        vec2(pid_w, 28.0),
    );
    cell_ui(ui, pid_cell, Layout::right_to_left(Align::Center), |ui| {
        let arrow = if state.sort_column == ProcessSortColumn::Pid {
            match state.sort_direction {
                SortDirection::Ascending => " ▲",
                SortDirection::Descending => " ▼",
            }
        } else {
            ""
        };
        let color = if state.sort_column == ProcessSortColumn::Pid {
            theme.accent_color
        } else {
            theme.text_secondary
        };
        if ui
            .add(
                egui::Button::new(
                    RichText::new(format!("PID{}", arrow))
                        .size(11.5)
                        .strong()
                        .color(color),
                )
                .frame(false),
            )
            .clicked()
        {
            if state.sort_column == ProcessSortColumn::Pid {
                state.sort_direction = match state.sort_direction {
                    SortDirection::Ascending => SortDirection::Descending,
                    SortDirection::Descending => SortDirection::Ascending,
                };
            } else {
                state.sort_column = ProcessSortColumn::Pid;
                state.sort_direction = SortDirection::Ascending;
            }
        }
    });

    // Memory column header
    let mem_cell = Rect::from_min_size(
        pos2(header_rect.left() + mem_x, header_rect.min.y),
        vec2(mem_w, 28.0),
    );
    cell_ui(ui, mem_cell, Layout::right_to_left(Align::Center), |ui| {
        let arrow = if state.sort_column == ProcessSortColumn::Memory {
            match state.sort_direction {
                SortDirection::Ascending => " ▲",
                SortDirection::Descending => " ▼",
            }
        } else {
            ""
        };
        let color = if state.sort_column == ProcessSortColumn::Memory {
            theme.accent_color
        } else {
            theme.text_secondary
        };
        if ui
            .add(
                egui::Button::new(
                    RichText::new(format!("Memory{}", arrow))
                        .size(11.5)
                        .strong()
                        .color(color),
                )
                .frame(false),
            )
            .clicked()
        {
            if state.sort_column == ProcessSortColumn::Memory {
                state.sort_direction = match state.sort_direction {
                    SortDirection::Ascending => SortDirection::Descending,
                    SortDirection::Descending => SortDirection::Ascending,
                };
            } else {
                state.sort_column = ProcessSortColumn::Memory;
                state.sort_direction = SortDirection::Descending;
            }
        }
    });

    // CPU % column header
    let cpu_cell = Rect::from_min_size(
        pos2(header_rect.left() + cpu_x, header_rect.min.y),
        vec2(cpu_w, 28.0),
    );
    cell_ui(ui, cpu_cell, Layout::right_to_left(Align::Center), |ui| {
        let arrow = if state.sort_column == ProcessSortColumn::Cpu {
            match state.sort_direction {
                SortDirection::Ascending => " ▲",
                SortDirection::Descending => " ▼",
            }
        } else {
            ""
        };
        let color = if state.sort_column == ProcessSortColumn::Cpu {
            theme.accent_color
        } else {
            theme.text_secondary
        };
        if ui
            .add(
                egui::Button::new(
                    RichText::new(format!("CPU %{}", arrow))
                        .size(11.5)
                        .strong()
                        .color(color),
                )
                .frame(false),
            )
            .clicked()
        {
            if state.sort_column == ProcessSortColumn::Cpu {
                state.sort_direction = match state.sort_direction {
                    SortDirection::Ascending => SortDirection::Descending,
                    SortDirection::Descending => SortDirection::Ascending,
                };
            } else {
                state.sort_column = ProcessSortColumn::Cpu;
                state.sort_direction = SortDirection::Descending;
            }
        }
    });

    // Type column header
    if show_type {
        let type_cell = Rect::from_min_size(
            pos2(header_rect.left() + type_x, header_rect.min.y),
            vec2(type_w, 28.0),
        );
        cell_ui(ui, type_cell, Layout::right_to_left(Align::Center), |ui| {
            ui.label(
                RichText::new("Type")
                    .size(11.5)
                    .strong()
                    .color(theme.text_secondary),
            );
        });
    }

    ui.add_space(4.0);

    // 6. Process Table Rows (macOS Activity Monitor Rows with Zebra Striping & Hover Feedback)
    ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.set_min_width(avail_width);

            if processes.is_empty() {
                ui.add_space(32.0);
                theme.card_frame().show(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.add_space(16.0);
                        ui.label(
                            RichText::new("No matching processes found")
                                .size(14.0)
                                .strong()
                                .color(theme.text_primary),
                        );
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new("Try adjusting your search query or filter category.")
                                .size(11.5)
                                .color(theme.text_secondary),
                        );
                        ui.add_space(12.0);
                        if ui
                            .add(theme.subtle_button(RichText::new("Clear Filters").size(11.5)))
                            .clicked()
                        {
                            state.search_query.clear();
                            state.filter_category = ProcessFilterCategory::All;
                        }
                        ui.add_space(16.0);
                    });
                });
                return;
            }

            for (idx, proc) in processes.iter().enumerate() {
                let is_selected = state.selected_pid == Some(proc.pid);
                let (row_rect, row_resp) =
                    ui.allocate_exact_size(vec2(avail_width, 27.0), Sense::click());

                // Subtle zebra striping and interactive hover
                let row_bg = if is_selected {
                    Color32::from_rgba_unmultiplied(
                        theme.accent_color.r(),
                        theme.accent_color.g(),
                        theme.accent_color.b(),
                        48,
                    )
                } else if row_resp.hovered() {
                    theme.subtle_bg
                } else if idx % 2 == 1 {
                    Color32::from_rgba_unmultiplied(
                        theme.subtle_bg.r(),
                        theme.subtle_bg.g(),
                        theme.subtle_bg.b(),
                        70,
                    )
                } else {
                    Color32::TRANSPARENT
                };

                if row_bg != Color32::TRANSPARENT {
                    ui.painter()
                        .rect_filled(row_rect, CornerRadius::same(5), row_bg);
                }

                if is_selected {
                    ui.painter().rect_stroke(
                        row_rect,
                        CornerRadius::same(5),
                        Stroke::new(
                            1.0,
                            Color32::from_rgba_unmultiplied(
                                theme.accent_color.r(),
                                theme.accent_color.g(),
                                theme.accent_color.b(),
                                140,
                            ),
                        ),
                        egui::StrokeKind::Inside,
                    );
                }

                // Row click selects, double-click opens inspector
                if row_resp.clicked() {
                    state.selected_pid = if is_selected { None } else { Some(proc.pid) };
                }
                if row_resp.double_clicked() {
                    action_request = Some(ProcessActionRequest::Inspect(proc.clone()));
                }

                // 1. Process Name cell (Left-aligned)
                let name_cell = Rect::from_min_size(
                    pos2(row_rect.left() + name_x, row_rect.min.y),
                    vec2(name_w, 27.0),
                );
                cell_ui(ui, name_cell, Layout::left_to_right(Align::Center), |ui| {
                    render_status_dot(ui, process_status_color(proc, theme), 3.0);
                    ui.add_space(6.0);
                    let name_color = if is_selected {
                        Color32::WHITE
                    } else {
                        theme.text_primary
                    };
                    ui.label(
                        RichText::new(&proc.name)
                            .size(12.5)
                            .strong()
                            .color(name_color),
                    );
                });

                // 2. PID cell (Right-aligned)
                let pid_cell = Rect::from_min_size(
                    pos2(row_rect.left() + pid_x, row_rect.min.y),
                    vec2(pid_w, 27.0),
                );
                cell_ui(ui, pid_cell, Layout::right_to_left(Align::Center), |ui| {
                    let pid_color = if is_selected {
                        Color32::WHITE
                    } else {
                        theme.text_secondary
                    };
                    ui.label(
                        RichText::new(format!("{}", proc.pid))
                            .size(11.5)
                            .color(pid_color),
                    );
                });

                // 3. Memory cell (Right-aligned)
                let mem_cell = Rect::from_min_size(
                    pos2(row_rect.left() + mem_x, row_rect.min.y),
                    vec2(mem_w, 27.0),
                );
                cell_ui(ui, mem_cell, Layout::right_to_left(Align::Center), |ui| {
                    let mem_color = if is_selected {
                        Color32::WHITE
                    } else {
                        theme.accent_color
                    };
                    ui.label(
                        RichText::new(format_bytes(proc.memory_bytes))
                            .size(12.0)
                            .strong()
                            .color(mem_color),
                    );
                });

                // 4. CPU % cell (Right-aligned)
                let cpu_cell = Rect::from_min_size(
                    pos2(row_rect.left() + cpu_x, row_rect.min.y),
                    vec2(cpu_w, 27.0),
                );
                cell_ui(ui, cpu_cell, Layout::right_to_left(Align::Center), |ui| {
                    let cpu_str = proc
                        .cpu_usage
                        .map_or("-".to_string(), |c| format!("{:.1}%", c));
                    let (cpu_color, is_bold) = if is_selected {
                        (Color32::WHITE, false)
                    } else if let Some(c) = proc.cpu_usage {
                        if c >= 50.0 {
                            (theme.critical_color, true)
                        } else if c >= 15.0 {
                            (theme.warning_color, true)
                        } else {
                            (theme.text_primary, false)
                        }
                    } else {
                        (theme.text_secondary, false)
                    };
                    let text = if is_bold {
                        RichText::new(cpu_str).size(11.5).strong().color(cpu_color)
                    } else {
                        RichText::new(cpu_str).size(11.5).color(cpu_color)
                    };
                    ui.label(text);
                });

                // 5. Type cell (Right-aligned, if visible)
                if show_type {
                    let type_cell = Rect::from_min_size(
                        pos2(row_rect.left() + type_x, row_rect.min.y),
                        vec2(type_w, 27.0),
                    );
                    cell_ui(ui, type_cell, Layout::right_to_left(Align::Center), |ui| {
                        let type_color = if is_selected {
                            Color32::WHITE
                        } else {
                            theme.text_secondary
                        };
                        let arch_str = match proc.architecture {
                            crate::models::ProcessArchitecture::Intel => " • Intel",
                            _ => "",
                        };
                        ui.label(
                            RichText::new(format!("{}{}", proc.display_type(), arch_str))
                                .size(11.0)
                                .color(type_color),
                        );
                    });
                }

                ui.add_space(1.0);
            }
        });

    action_request
}
