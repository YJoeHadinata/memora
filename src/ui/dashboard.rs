use egui::{vec2, Color32, CornerRadius, Rect, RichText, ScrollArea, Ui};
use egui_plot::{Legend, Line, Plot, PlotPoints};

use crate::models::{MemoryPressure, MemorySnapshot, ProcessInfo, StorageSnapshot};
use crate::services::HistoryService;
use crate::ui::components::{render_pressure_badge, MetricCard};
use crate::ui::helpers::{
    process_status_color, render_progress_bar, render_section_header, render_status_dot,
    render_storage_icon, render_tinted_pill, StorageIconKind,
};
use crate::ui::theme::AppTheme;
use crate::utils::format::{format_bytes, format_percent, format_time_hhmmss};

pub enum DashboardAction {
    ExportReport,
    InspectProcess(ProcessInfo),
    NavigateToProcesses,
    NavigateToCaches,
}

pub fn render_dashboard(
    ui: &mut Ui,
    snapshot: Option<&MemorySnapshot>,
    storage: Option<&StorageSnapshot>,
    history_svc: &HistoryService,
    top_processes: &[ProcessInfo],
    is_history_paused: &mut bool,
    theme: &AppTheme,
) -> Option<DashboardAction> {
    let mut action = None;

    ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let avail_width = (ui.available_width() - 4.0).max(200.0);
            ui.set_max_width(avail_width);
            ui.add_space(6.0);

        if let Some(snap) = snapshot {
            // Header with title and pressure status (macOS window title feel)
            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    render_pressure_badge(ui, snap.pressure, theme);

                    ui.add_space(10.0);

                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new("Export Snapshot")
                                    .size(11.5)
                                    .color(theme.text_primary),
                            )
                            .fill(theme.card_bg)
                            .stroke(theme.card_stroke())
                            .corner_radius(theme.button_radius()),
                        )
                        .on_hover_text("Export diagnostic JSON report (⌘E)")
                        .clicked()
                    {
                        action = Some(DashboardAction::ExportReport);
                    }

                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                        ui.vertical(|ui| {
                            ui.label(
                                RichText::new("Memory Overview")
                                    .size(22.0)
                                    .strong()
                                    .color(theme.text_primary),
                            );
                            ui.label(
                                RichText::new(format!(
                                    "Updated {} • {}",
                                    format_time_hhmmss(snap.timestamp),
                                    snap.pressure.description()
                                ))
                                .size(12.0)
                                .color(theme.text_secondary),
                            );
                        });
                    });
                });
            });

            // Elevated Pressure Alert Banner
            if snap.pressure == MemoryPressure::Warning || snap.pressure == MemoryPressure::Critical {
                ui.add_space(12.0);
                let is_critical = snap.pressure == MemoryPressure::Critical;
                let banner_color = if is_critical {
                    theme.critical_color
                } else {
                    theme.warning_color
                };
                let bg_color = Color32::from_rgba_unmultiplied(
                    banner_color.r(),
                    banner_color.g(),
                    banner_color.b(),
                    26,
                );

                egui::Frame::new()
                    .fill(bg_color)
                    .corner_radius(CornerRadius::same(10))
                    .stroke(egui::Stroke::new(1.0, banner_color))
                    .inner_margin(egui::Margin::symmetric(14, 10))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let msg = if is_critical {
                                "Critical Memory Pressure: System is paging compressed memory to disk swapfile."
                            } else {
                                "Elevated Memory Pressure: macOS compressor is actively compacting inactive memory."
                            };

                            ui.label(
                                RichText::new(msg)
                                    .size(12.0)
                                    .strong()
                                    .color(banner_color),
                            );

                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui
                                    .add(
                                        egui::Button::new(
                                            RichText::new("Manage Processes →")
                                                .size(11.5)
                                                .strong()
                                                .color(Color32::WHITE),
                                        )
                                        .fill(banner_color)
                                        .corner_radius(theme.button_radius()),
                                    )
                                    .clicked()
                                {
                                    action = Some(DashboardAction::NavigateToProcesses);
                                }
                            });
                        });
                    });
            }

            ui.add_space(14.0);

            // Primary Metric Cards Grid (Fluid Responsive: 3 cols on desktop, 2 cols on medium, 1 col on small)
            let (cols, spacing) = if avail_width >= 720.0 {
                (3usize, 12.0)
            } else if avail_width >= 480.0 {
                (2usize, 10.0)
            } else {
                (1usize, 8.0)
            };
            let card_w = ((avail_width - (cols as f32 - 1.0) * spacing) / cols as f32).floor().max(100.0);

            let mut cards: Vec<MetricCard<'static>> = Vec::with_capacity(6);

            // 1. Physical RAM
            cards.push(
                MetricCard::new("PHYSICAL MEMORY", format_bytes(snap.total_bytes))
                    .subtitle("Apple Silicon Unified Memory"),
            );

            // 2. Memory Used
            let used_pct = snap.used_percent();
            let used_color = if used_pct > 85.0 {
                theme.critical_color
            } else if used_pct > 70.0 {
                theme.warning_color
            } else {
                theme.accent_color
            };
            cards.push(
                MetricCard::new("MEMORY USED", format_bytes(snap.used_bytes))
                    .subtitle(format!("{} of total RAM", format_percent(used_pct)))
                    .progress(used_pct / 100.0, used_color),
            );

            // 3. Memory Available
            let avail_pct = snap.available_percent();
            cards.push(
                MetricCard::new("AVAILABLE MEMORY", format_bytes(snap.available_bytes))
                    .subtitle(format!("{} free or reclaimable", format_percent(avail_pct)))
                    .progress(avail_pct / 100.0, theme.normal_color),
            );

            // 4. Compressed Memory
            cards.push(
                MetricCard::new("COMPRESSED", format_bytes(snap.compressed_bytes))
                    .subtitle("macOS memory compressor"),
            );

            // 5. Swap Used
            let swap_sub = if snap.swap_total_bytes > 0 {
                format!("of {} total swap", format_bytes(snap.swap_total_bytes))
            } else {
                "No swap files allocated".to_string()
            };
            let swap_color = if snap.swap_used_bytes > 1024 * 1024 * 1024 {
                theme.warning_color
            } else {
                theme.text_secondary
            };
            cards.push(
                MetricCard::new("SWAP USED", format_bytes(snap.swap_used_bytes))
                    .subtitle(swap_sub)
                    .progress(
                        if snap.swap_total_bytes > 0 {
                            snap.swap_used_percent() / 100.0
                        } else {
                            0.0
                        },
                        swap_color,
                    ),
            );

            // 6. Storage Used (Primary Internal SSD)
            if let Some(storage) = storage {
                if let Some(ref primary) = storage.primary_volume {
                    let used_pct = primary.used_percent();
                    let disk_color = if used_pct > 88.0 {
                        theme.critical_color
                    } else if used_pct > 75.0 {
                        theme.warning_color
                    } else {
                        theme.accent_color
                    };
                    cards.push(
                        MetricCard::new("STORAGE USED", format_bytes(primary.used_bytes))
                            .subtitle(format!(
                                "{} used ({} free)",
                                format_percent(used_pct),
                                format_bytes(primary.available_bytes)
                            ))
                            .progress(used_pct / 100.0, disk_color),
                    );
                }
            }

            // Render cards in strictly bounded rows that NEVER overflow horizontally
            for (row_idx, chunk) in cards.chunks(cols).enumerate() {
                if row_idx > 0 {
                    ui.add_space(spacing);
                }
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = vec2(spacing, 0.0);
                    for card in chunk {
                        card.clone().width(card_w).ui(ui, theme);
                    }
                });
            }

            ui.add_space(18.0);

            // Memory Composition Breakdown Card (Apple HIG style)
            theme.card_frame().show(ui, |ui| {
                render_section_header(ui, "Memory Composition", theme);
                ui.add_space(10.0);

                // Stacked visual bar (Apple rounded multi-segment bar)
                let total_f = snap.total_bytes as f32;
                let app_pct = snap.app_memory_bytes as f32 / total_f;
                let wired_pct = snap.wired_bytes as f32 / total_f;
                let comp_pct = snap.compressed_bytes as f32 / total_f;
                let cached_pct = snap.cached_files_bytes as f32 / total_f;
                let free_pct = snap.free_bytes as f32 / total_f;

                let bar_height = 16.0;
                let (bar_rect, _resp) = ui.allocate_exact_size(
                    vec2(ui.available_width(), bar_height),
                    egui::Sense::hover(),
                );

                // Background track with smooth Apple capsule radius
                ui.painter().rect_filled(bar_rect, CornerRadius::same(5), theme.border_subtle);

                let mut x = bar_rect.min.x;
                let segments = [
                    (app_pct, theme.accent_color, "App Memory"),
                    (wired_pct, theme.purple_color, "Wired"),
                    (comp_pct, theme.warning_color, "Compressed"),
                    (cached_pct, theme.cyan_color, "Cached Files"),
                    (free_pct, theme.normal_color, "Free"),
                ];

                for (pct, color, _) in segments.iter() {
                    let width = bar_rect.width() * pct;
                    if width > 0.5 {
                        let seg_rect = Rect::from_min_size(
                            egui::pos2(x, bar_rect.min.y),
                            vec2(width, bar_height),
                        );
                        ui.painter().rect_filled(seg_rect, CornerRadius::ZERO, *color);
                        x += width;
                    }
                }

                ui.add_space(14.0);

                // Responsive Legend items with values
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = vec2(16.0, 6.0);

                    for (_, color, name) in segments.iter() {
                        ui.horizontal(|ui| {
                            let (dot_rect, _) =
                                ui.allocate_exact_size(vec2(8.0, 8.0), egui::Sense::hover());
                            ui.painter()
                                .rect_filled(dot_rect, CornerRadius::same(3), *color);
                            let val_str = match *name {
                                "App Memory" => format_bytes(snap.app_memory_bytes),
                                "Wired" => format_bytes(snap.wired_bytes),
                                "Compressed" => format_bytes(snap.compressed_bytes),
                                "Cached Files" => format_bytes(snap.cached_files_bytes),
                                "Free" => format_bytes(snap.free_bytes),
                                _ => String::new(),
                            };
                            ui.label(
                                RichText::new(format!("{}: {}", name, val_str))
                                    .size(11.5)
                                    .color(theme.text_secondary),
                            );
                        });
                    }
                });

                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new("Clear & Manage Caches →")
                                        .size(11.0)
                                        .color(theme.accent_color),
                                )
                                .fill(Color32::TRANSPARENT),
                            )
                            .clicked()
                        {
                            action = Some(DashboardAction::NavigateToCaches);
                        }

                        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                            ui.label(
                                RichText::new(format!(
                                    "Cached files in RAM: {} (reclaimable: {})",
                                    format_bytes(snap.cached_files_bytes),
                                    format_bytes(snap.purgeable_bytes)
                                ))
                                .size(11.0)
                                .color(theme.text_secondary),
                            );
                        });
                    });
                });
            });

            ui.add_space(18.0);

            // All Disk & Storage Volumes Card (Apple Storage style)
            if let Some(storage) = storage {
                theme.card_frame().show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let (status_title, _) = storage.swap_headroom_status();
                            let badge_color = match status_title {
                                "Healthy" => theme.normal_color,
                                "Moderate" => theme.warning_color,
                                _ => theme.critical_color,
                            };

                            render_tinted_pill(
                                ui,
                                &format!("Swap Headroom: {}", status_title),
                                badge_color,
                                10.5,
                            );

                            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                                render_section_header(
                                    ui,
                                    &format!("STORAGE VOLUMES ({} MOUNTED)", storage.volumes.len()),
                                    theme,
                                );
                            });
                        });
                    });

                    ui.add_space(6.0);
                    ui.label(
                        RichText::new(format!(
                            "{} • Sisa ruang SSD tersedia untuk swap memori: {}",
                            storage.swap_headroom_status().1,
                            format_bytes(storage.swap_headroom_bytes)
                        ))
                        .size(11.0)
                        .color(theme.text_secondary),
                    );

                    ui.add_space(14.0);

                    let user_vols = storage.user_volumes();
                    let helper_vols = storage.helper_volumes();

                    for (idx, vol) in user_vols.iter().enumerate() {
                        if idx > 0 {
                            ui.add_space(8.0);
                            ui.separator();
                            ui.add_space(8.0);
                        }

                        render_volume_row(ui, vol, theme);
                    }

                    if !helper_vols.is_empty() {
                        let show_helpers_id = ui.make_persistent_id("show_system_helper_volumes");
                        let mut show_helpers =
                            ui.data_mut(|d| *d.get_temp_mut_or_default::<bool>(show_helpers_id));

                        ui.add_space(10.0);
                        ui.horizontal(|ui| {
                            let label = if show_helpers {
                                format!("Hide Internal System Volumes ({} hidden)", helper_vols.len())
                            } else {
                                format!("Show Internal System Volumes ({} hidden)", helper_vols.len())
                            };
                            if ui
                                .add(theme.subtle_button(RichText::new(label).size(11.0)))
                                .clicked()
                            {
                                show_helpers = !show_helpers;
                                ui.data_mut(|d| d.insert_temp(show_helpers_id, show_helpers));
                            }
                        });

                        if show_helpers {
                            ui.add_space(8.0);
                            for (idx, vol) in helper_vols.iter().enumerate() {
                                if idx > 0 || !user_vols.is_empty() {
                                    ui.add_space(8.0);
                                    ui.separator();
                                    ui.add_space(8.0);
                                }
                                render_volume_row(ui, vol, theme);
                            }
                        }
                    }
                });

                ui.add_space(18.0);
            }

            // Top Memory Consumers Card (Quick Action)
            if !top_processes.is_empty() {
                theme.card_frame().show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui
                                .add(
                                    egui::Button::new(
                                        RichText::new("View All Processes →")
                                            .size(11.5)
                                            .color(theme.accent_color),
                                    )
                                    .frame(false),
                                )
                                .clicked()
                            {
                                action = Some(DashboardAction::NavigateToProcesses);
                            }

                            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                                render_section_header(ui, "Top Memory Consumers", theme);
                            });
                        });
                    });

                    ui.add_space(10.0);

                    let count = top_processes.len().min(5);
                    for proc in &top_processes[..count] {
                        ui.horizontal(|ui| {
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui
                                    .add(theme.subtle_button(RichText::new("Inspect").size(11.0)))
                                    .clicked()
                                {
                                    action = Some(DashboardAction::InspectProcess(proc.clone()));
                                }

                                ui.add_space(12.0);

                                if let Some(cpu) = proc.cpu_usage {
                                    ui.label(
                                        RichText::new(format!("{:.1}% CPU", cpu))
                                            .size(11.0)
                                            .color(theme.text_secondary),
                                    );
                                    ui.add_space(12.0);
                                }

                                ui.label(
                                    RichText::new(format_bytes(proc.memory_bytes))
                                        .size(13.0)
                                        .strong()
                                        .color(theme.accent_color),
                                );

                                ui.with_layout(
                                    egui::Layout::left_to_right(egui::Align::Center),
                                    |ui| {
                                        render_status_dot(ui, process_status_color(proc, theme), 3.0);

                                        ui.label(
                                            RichText::new(&proc.name)
                                                .size(13.0)
                                                .strong()
                                                .color(theme.text_primary),
                                        );

                                        ui.label(
                                            RichText::new(format!("(PID: {})", proc.pid))
                                                .size(11.0)
                                                .color(theme.text_secondary),
                                        );
                                    },
                                );
                            });
                        });

                        ui.add_space(6.0);
                    }
                });

                ui.add_space(18.0);
            }

            // History Chart & Session Stats
            theme.card_frame().show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            RichText::new(format!(
                                "Peak: {} • Avg: {}",
                                format_bytes(history_svc.peak_used_bytes()),
                                format_bytes(history_svc.average_used_bytes())
                            ))
                            .size(11.0)
                            .color(theme.text_secondary),
                        );

                        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                            render_section_header(ui, "Memory History (GB)", theme);

                            ui.add_space(10.0);

                            let pause_label = if *is_history_paused {
                                "Resume Live"
                            } else {
                                "Pause Live"
                            };

                            let pause_color = if *is_history_paused {
                                theme.warning_color
                            } else {
                                theme.text_secondary
                            };

                            if ui
                                .add(theme.subtle_button(
                                    RichText::new(pause_label).size(11.0).color(pause_color),
                                ))
                                .clicked()
                            {
                                *is_history_paused = !*is_history_paused;
                            }

                            // Quick Time Range Filter Pill Capsule
                            ui.add_space(8.0);
                            let range_id = ui.make_persistent_id("history_time_range_secs");
                            let mut range_secs: u32 =
                                ui.data_mut(|d| *d.get_temp_mut_or_default::<u32>(range_id));

                            let ranges = [(0u32, "All"), (60u32, "1m"), (180u32, "3m")];
                            for (r_val, r_label) in ranges {
                                let is_sel = range_secs == r_val;
                                let (bg, fg) = if is_sel {
                                    (theme.accent_color, Color32::WHITE)
                                } else {
                                    (Color32::TRANSPARENT, theme.text_secondary)
                                };
                                let btn = egui::Button::new(
                                    RichText::new(r_label).size(10.5).strong().color(fg),
                                )
                                .fill(bg)
                                .corner_radius(CornerRadius::same(5));
                                if ui.add(btn).clicked() {
                                    range_secs = r_val;
                                    ui.data_mut(|d| d.insert_temp(range_id, range_secs));
                                }
                            }
                        });
                    });
                });

                ui.add_space(8.0);

                let range_id = ui.make_persistent_id("history_time_range_secs");
                let range_secs: u32 =
                    ui.data_mut(|d| *d.get_temp_mut_or_default::<u32>(range_id));

                let max_ts = history_svc
                    .buffer()
                    .points()
                    .back()
                    .map_or(0.0, |p| p.timestamp_secs);
                let min_ts = if range_secs > 0 {
                    (max_ts - range_secs as f64).max(0.0)
                } else {
                    0.0
                };

                let used_points: PlotPoints = history_svc
                    .buffer()
                    .points()
                    .iter()
                    .filter(|p| p.timestamp_secs >= min_ts)
                    .map(|p| [p.timestamp_secs, p.used_gb])
                    .collect();

                let compressed_points: PlotPoints = history_svc
                    .buffer()
                    .points()
                    .iter()
                    .filter(|p| p.timestamp_secs >= min_ts)
                    .map(|p| [p.timestamp_secs, p.compressed_gb])
                    .collect();

                let swap_points: PlotPoints = history_svc
                    .buffer()
                    .points()
                    .iter()
                    .filter(|p| p.timestamp_secs >= min_ts)
                    .map(|p| [p.timestamp_secs, p.swap_gb])
                    .collect();

                    let plot = Plot::new("memory_history_plot")
                        .legend(Legend::default().position(egui_plot::Corner::RightBottom))
                        .show_axes([true, true])
                        .show_grid(true)
                        .height(200.0)
                        .include_y(0.0)
                        .include_y(snap.total_bytes as f64 / (1024.0 * 1024.0 * 1024.0));

                    plot.show(ui, |plot_ui| {
                        plot_ui.line(
                            Line::new("Used Memory", used_points)
                                .color(theme.accent_color)
                                .width(2.0),
                        );
                        plot_ui.line(
                            Line::new("Compressed", compressed_points)
                                .color(theme.warning_color)
                                .width(1.5),
                        );
                        plot_ui.line(
                            Line::new("Swap", swap_points)
                                .color(theme.critical_color)
                                .width(1.5),
                        );
                    });
                });
        } else {
            ui.vertical_centered(|ui| {
                ui.add_space(80.0);
                ui.spinner();
                ui.add_space(12.0);
                ui.label(
                    RichText::new("Collecting macOS memory metrics...")
                        .size(14.0)
                        .color(theme.text_secondary),
                );
            });
        }

        ui.add_space(20.0);
    });

    action
}

fn render_volume_row(ui: &mut Ui, vol: &crate::models::DiskVolume, theme: &AppTheme) {
    let used_pct = vol.used_percent();
    let bar_color = if used_pct > 88.0 {
        theme.critical_color
    } else if used_pct > 75.0 {
        theme.warning_color
    } else {
        theme.accent_color
    };

    let icon_kind = if vol.is_removable {
        StorageIconKind::ExternalDrive
    } else if vol.mount_point == "/System/Volumes/Data" {
        StorageIconKind::InternalData
    } else if vol.mount_point == "/" {
        StorageIconKind::SystemSnapshot
    } else if vol.mount_point == "/System/Volumes/VM" {
        StorageIconKind::SwapVolume
    } else {
        StorageIconKind::StandardVolume
    };

    ui.horizontal(|ui| {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(
                RichText::new(format!(
                    "{} / {} ({})",
                    format_bytes(vol.used_bytes),
                    format_bytes(vol.total_bytes),
                    format_percent(used_pct)
                ))
                .size(12.0)
                .strong()
                .color(theme.text_primary),
            );
            if ui.available_width() > 80.0 {
                ui.label(
                    RichText::new(format!("{} free •", format_bytes(vol.available_bytes)))
                        .size(11.0)
                        .color(theme.text_secondary),
                );
            }

            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                render_storage_icon(ui, icon_kind, theme.accent_color, 16.0);
                ui.add_space(6.0);
                ui.label(
                    RichText::new(&vol.name)
                        .size(13.0)
                        .strong()
                        .color(theme.text_primary),
                );
                if vol.mount_point != "/" && !vol.name.contains(&vol.mount_point) {
                    let short_mount = if vol.mount_point.len() > 30 {
                        format!("...{}", &vol.mount_point[vol.mount_point.len() - 26..])
                    } else {
                        vol.mount_point.clone()
                    };
                    ui.label(
                        RichText::new(format!("({})", short_mount))
                            .size(11.0)
                            .color(theme.text_secondary),
                    );
                }
            });
        });
    });

    ui.add_space(6.0);

    // Progress Bar for each volume using reusable helper
    render_progress_bar(
        ui,
        used_pct / 100.0,
        6.0,
        bar_color,
        theme.border_subtle,
        CornerRadius::same(3),
    );
}
