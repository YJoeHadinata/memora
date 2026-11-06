use std::path::PathBuf;
use std::time::Instant;

use egui::{vec2, Align, Color32, CornerRadius, Layout, RichText, ScrollArea, Stroke, Ui};

use crate::models::{
    AgeFilter, CacheScanResult, LargeFileCategory, LargeFilesScanResult, MemorySnapshot,
    SizeFilter, StorageJunkReport,
};
use crate::ui::helpers::{render_search_icon, render_section_header};
use crate::ui::theme::AppTheme;
use crate::utils::format::format_bytes;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CleanupSubTab {
    #[default]
    Storage,
    LargeFiles,
    Caches,
    Maintenance,
}

#[derive(Debug, Clone, Default)]
pub struct CleanupViewState {
    pub active_subtab: CleanupSubTab,

    // Storage Optimizer
    pub storage_junk: Option<StorageJunkReport>,
    pub is_scanning_storage: bool,
    pub is_cleaning_storage: bool,

    // Large & Old Files
    pub large_files_result: Option<LargeFilesScanResult>,
    pub is_scanning_large_files: bool,
    pub is_cleaning_large_files: bool,
    pub large_files_size_filter: SizeFilter,
    pub large_files_age_filter: AgeFilter,
    pub large_files_category_filter: LargeFileCategory,
    pub large_files_search_query: String,

    // Application Caches
    pub cache_scan_result: Option<CacheScanResult>,
    pub is_scanning_caches: bool,
    pub is_cleaning_caches: bool,
    pub cache_search_query: String,

    // System RAM & Maintenance
    pub is_purging_ram: bool,

    // Status banner
    pub last_action_message: Option<(String, Instant, bool)>,
}

// Backward-compatible alias
pub type CacheViewState = CleanupViewState;

#[derive(Debug, Clone)]
pub enum CleanupActionRequest {
    // Storage Optimizer
    ScanStorageJunk,
    EmptyTrash,
    DeleteSelectedInstallers(Vec<PathBuf>),
    PurgeApfsSnapshots,
    PruneUnavailableSimulators,

    // Large & Old Files
    ScanLargeFiles(u64),
    RevealLargeFileInFinder(PathBuf),
    DeleteSelectedLargeFiles(Vec<PathBuf>),

    // Application Caches
    ScanCaches,
    CleanSelectedCaches(Vec<PathBuf>),

    // System RAM & Maintenance
    PurgeRamCache,
    FlushDns,
    FlushQuickLook,
}

// Backward-compatible alias
pub type CacheActionRequest = CleanupActionRequest;

pub fn render_caches(
    ui: &mut Ui,
    snapshot: Option<&MemorySnapshot>,
    state: &mut CleanupViewState,
    theme: &AppTheme,
) -> Option<CleanupActionRequest> {
    let mut requested_action = None;

    ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let avail_width = (ui.available_width() - 4.0).max(200.0);
            ui.set_max_width(avail_width);
            ui.add_space(8.0);

            // Header Title & Description
            ui.vertical(|ui| {
                ui.label(
                    RichText::new("Cleanup & Storage")
                        .size(22.0)
                        .strong()
                        .color(theme.text_primary),
                );
                ui.label(
                    RichText::new(
                        "Reclaim disk storage, clear developer junk, and manage system caches with complete transparency.",
                    )
                    .size(12.0)
                    .color(theme.text_secondary),
                );
            });

            ui.add_space(14.0);

            // Apple-style Sub-tab Segmented Control
            ui.horizontal(|ui| {
                let subtabs = [
                    (CleanupSubTab::Storage, "Storage Optimizer"),
                    (CleanupSubTab::LargeFiles, "Large & Old Files"),
                    (CleanupSubTab::Caches, "Application Caches"),
                    (CleanupSubTab::Maintenance, "RAM & Diagnostics"),
                ];

                for (tab, label) in subtabs {
                    let is_active = state.active_subtab == tab;
                    let btn = if is_active {
                        theme.primary_button(RichText::new(label).size(12.0).strong())
                    } else {
                        theme.subtle_button(RichText::new(label).size(12.0))
                    };

                    if ui.add(btn).clicked() {
                        state.active_subtab = tab;
                    }
                }
            });

            ui.add_space(14.0);

            // Render Active Sub-tab View
            match state.active_subtab {
                CleanupSubTab::Storage => {
                    render_storage_optimizer(ui, state, theme, &mut requested_action);
                }
                CleanupSubTab::LargeFiles => {
                    render_large_files_finder(ui, state, theme, &mut requested_action);
                }
                CleanupSubTab::Caches => {
                    render_application_caches(ui, state, theme, &mut requested_action);
                }
                CleanupSubTab::Maintenance => {
                    render_ram_and_maintenance(ui, snapshot, state, theme, &mut requested_action);
                }
            }

            ui.add_space(20.0);
        });

    requested_action
}

/// Renders the Storage Optimizer view (Trash, Downloads installers, APFS snapshots, Simulators).
fn render_storage_optimizer(
    ui: &mut Ui,
    state: &mut CleanupViewState,
    theme: &AppTheme,
    action: &mut Option<CleanupActionRequest>,
) {
    // Summary Card
    theme.card_frame().show(ui, |ui| {
        ui.horizontal(|ui| {
            render_section_header(ui, "Storage Reclaim Overview", theme);

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let scan_label = if state.is_scanning_storage {
                    "Scanning Storage..."
                } else {
                    "Rescan Storage"
                };

                if ui
                    .add_enabled(
                        !state.is_scanning_storage,
                        theme.subtle_button(RichText::new(scan_label).size(11.5)),
                    )
                    .clicked()
                {
                    *action = Some(CleanupActionRequest::ScanStorageJunk);
                }
            });
        });

        ui.add_space(10.0);

        let total_reclaimable = state
            .storage_junk
            .as_ref()
            .map(|j| j.total_reclaimable_bytes())
            .unwrap_or(0);

        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(
                    RichText::new(format_bytes(total_reclaimable))
                        .size(24.0)
                        .strong()
                        .color(if total_reclaimable > 0 {
                            theme.accent_color
                        } else {
                            theme.text_secondary
                        }),
                );
                ui.label(
                    RichText::new("Total Readily Reclaimable Space")
                        .size(11.5)
                        .color(theme.text_secondary),
                );
            });

            if let Some(ref junk) = state.storage_junk {
                ui.add_space(32.0);

                ui.vertical(|ui| {
                    ui.label(
                        RichText::new(format_bytes(junk.trash.total_bytes))
                            .size(18.0)
                            .strong()
                            .color(theme.text_primary),
                    );
                    ui.label(
                        RichText::new(format!("Trash ({} items)", junk.trash.item_count))
                            .size(11.0)
                            .color(theme.text_secondary),
                    );
                });

                ui.add_space(24.0);

                let installer_bytes: u64 = junk.installers.iter().map(|i| i.size_bytes).sum();
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new(format_bytes(installer_bytes))
                            .size(18.0)
                            .strong()
                            .color(theme.text_primary),
                    );
                    ui.label(
                        RichText::new(format!("Installers ({} files)", junk.installers.len()))
                            .size(11.0)
                            .color(theme.text_secondary),
                    );
                });

                ui.add_space(24.0);

                ui.vertical(|ui| {
                    ui.label(
                        RichText::new(format!("{}", junk.apfs_snapshots.len()))
                            .size(18.0)
                            .strong()
                            .color(theme.text_primary),
                    );
                    ui.label(
                        RichText::new("Local APFS Snapshots")
                            .size(11.0)
                            .color(theme.text_secondary),
                    );
                });
            }
        });
    });

    ui.add_space(14.0);

    // Card 1: macOS Trash
    theme.card_frame().show(ui, |ui| {
        let btn_width = 120.0;
        let left_width = (ui.available_width() - btn_width - 16.0).max(180.0);

        ui.horizontal(|ui| {
            ui.allocate_ui_with_layout(vec2(left_width, 0.0), Layout::top_down(Align::Min), |ui| {
                render_section_header(ui, "macOS Trash (~/.Trash)", theme);
                let msg = if let Some(ref junk) = state.storage_junk {
                    if junk.trash.item_count > 0 {
                        format!(
                            "{} across {} items ready to be permanently deleted.",
                            format_bytes(junk.trash.total_bytes),
                            junk.trash.item_count
                        )
                    } else {
                        "The macOS Trash is completely empty.".to_string()
                    }
                } else {
                    "Checking Trash status...".to_string()
                };

                ui.add(
                    egui::Label::new(RichText::new(msg).size(12.0).color(theme.text_secondary))
                        .wrap(),
                );
            });

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let has_items = state
                    .storage_junk
                    .as_ref()
                    .map(|j| j.trash.item_count > 0)
                    .unwrap_or(false);
                let empty_btn = theme.critical_button(RichText::new("Empty Trash").size(11.5));

                if ui.add_enabled(has_items, empty_btn).clicked() {
                    *action = Some(CleanupActionRequest::EmptyTrash);
                }
            });
        });
    });

    ui.add_space(14.0);

    // Card 2: Leftover Installers in ~/Downloads
    theme.card_frame().show(ui, |ui| {
        let selected_paths: Vec<PathBuf> = state
            .storage_junk
            .as_ref()
            .map(|j| {
                j.installers
                    .iter()
                    .filter(|i| i.selected)
                    .map(|i| i.path.clone())
                    .collect()
            })
            .unwrap_or_default();

        let btn_width = 190.0;
        let left_width = (ui.available_width() - btn_width - 16.0).max(180.0);

        ui.horizontal(|ui| {
            ui.allocate_ui_with_layout(
                vec2(left_width, 0.0),
                Layout::top_down(Align::Min),
                |ui| {
                    render_section_header(ui, "Leftover Installers & Disk Images", theme);
                    ui.add(
                        egui::Label::new(
                            RichText::new("Downloaded .dmg, .pkg, and .iso disk packages across user folders and volumes.")
                                .size(12.0)
                                .color(theme.text_secondary),
                        )
                        .wrap(),
                    );
                },
            );

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let has_selected = !selected_paths.is_empty();
                let del_btn =
                    theme.critical_button(RichText::new("Delete Selected Installers").size(11.5));

                if ui.add_enabled(has_selected, del_btn).clicked() {
                    *action = Some(CleanupActionRequest::DeleteSelectedInstallers(
                        selected_paths,
                    ));
                }
            });
        });

        ui.add_space(10.0);

        if let Some(ref mut junk) = state.storage_junk {
            if junk.installers.is_empty() {
                ui.label(
                    RichText::new("✓ No leftover installer packages found in user folders or volumes.")
                        .size(12.0)
                        .color(theme.normal_color),
                );
            } else {
                ui.horizontal(|ui| {
                    if ui.add(theme.subtle_button("Select All")).clicked() {
                        for item in &mut junk.installers {
                            item.selected = true;
                        }
                    }
                    if ui.add(theme.subtle_button("Deselect All")).clicked() {
                        for item in &mut junk.installers {
                            item.selected = false;
                        }
                    }
                });

                ui.add_space(8.0);

                for item in &mut junk.installers {
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut item.selected, "");
                        ui.label(
                            RichText::new(&item.name)
                                .size(12.5)
                                .strong()
                                .color(theme.text_primary),
                        );
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            ui.label(
                                RichText::new(format_bytes(item.size_bytes))
                                    .size(12.0)
                                    .strong()
                                    .color(theme.text_secondary),
                            );
                            ui.label(
                                RichText::new(format!("{}d ago", item.modified_days_ago))
                                    .size(11.0)
                                    .color(theme.text_secondary),
                            );
                        });
                    });
                    ui.add_space(4.0);
                }
            }
        } else {
            ui.label(
                RichText::new("Scanning user folders and volumes for installers...")
                    .size(12.0)
                    .color(theme.text_secondary),
            );
        }
    });

    ui.add_space(14.0);

    // Card 3: APFS Local Snapshots
    theme.card_frame().show(ui, |ui| {
        let snapshot_count = state
            .storage_junk
            .as_ref()
            .map(|j| j.apfs_snapshots.len())
            .unwrap_or(0);

        let btn_width = 180.0;
        let left_width = (ui.available_width() - btn_width - 16.0).max(180.0);

        ui.horizontal(|ui| {
            ui.allocate_ui_with_layout(
                vec2(left_width, 0.0),
                Layout::top_down(Align::Min),
                |ui| {
                    render_section_header(ui, "APFS Local Snapshots (Time Machine)", theme);
                    ui.add(
                        egui::Label::new(
                            RichText::new(
                                "Time Machine automatically retains local snapshots on APFS drives when backup targets are disconnected. These often consume dozens of GBs in macOS 'System Data'.",
                            )
                            .size(12.0)
                            .color(theme.text_secondary),
                        )
                        .wrap(),
                    );
                },
            );

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let has_snapshots = snapshot_count > 0;
                let purge_btn = theme
                    .primary_button(RichText::new("Purge Local Snapshots").size(11.5));

                if ui.add_enabled(has_snapshots, purge_btn).clicked() {
                    *action = Some(CleanupActionRequest::PurgeApfsSnapshots);
                }
            });
        });

        ui.add_space(8.0);

        if snapshot_count == 0 {
            ui.label(
                RichText::new("✓ No orphaned APFS local snapshots found on root drive.")
                    .size(12.0)
                    .color(theme.normal_color),
            );
        } else {
            ui.label(
                RichText::new(format!(
                    "Found {} local APFS snapshot(s). Purging them immediately recovers disk headroom.",
                    snapshot_count
                ))
                .size(12.0)
                .color(theme.warning_color),
            );
        }
    });

    ui.add_space(14.0);

    // Card 4: Xcode Simulators
    theme.card_frame().show(ui, |ui| {
        let btn_width = 210.0;
        let left_width = (ui.available_width() - btn_width - 16.0).max(180.0);

        ui.horizontal(|ui| {
            ui.allocate_ui_with_layout(
                vec2(left_width, 0.0),
                Layout::top_down(Align::Min),
                |ui| {
                    render_section_header(ui, "Developer Simulator Runtimes", theme);
                    ui.add(
                        egui::Label::new(
                            RichText::new(
                                "Removes unavailable or corrupted iOS/watchOS/tvOS simulator runtimes left behind by previous Xcode updates.",
                            )
                            .size(12.0)
                            .color(theme.text_secondary),
                        )
                        .wrap(),
                    );
                },
            );

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui
                    .add(theme.subtle_button(
                        RichText::new("Prune Unavailable Simulators").size(11.5),
                    ))
                    .clicked()
                {
                    *action = Some(CleanupActionRequest::PruneUnavailableSimulators);
                }
            });
        });
    });
}

/// Renders the Application Caches view.
fn render_application_caches(
    ui: &mut Ui,
    state: &mut CleanupViewState,
    theme: &AppTheme,
    action: &mut Option<CleanupActionRequest>,
) {
    let scan_result = state.cache_scan_result.as_mut();

    theme.card_frame().show(ui, |ui| {
        ui.horizontal(|ui| {
            render_section_header(ui, "Application & Developer Caches on Disk", theme);

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let scan_text = if state.is_scanning_caches {
                    "Scanning..."
                } else {
                    "Scan Caches"
                };

                if ui
                    .add_enabled(
                        !state.is_scanning_caches,
                        theme.subtle_button(RichText::new(scan_text).size(11.5)),
                    )
                    .clicked()
                {
                    *action = Some(CleanupActionRequest::ScanCaches);
                }

                if let Some(ref scan) = scan_result {
                    let selected_count = scan.selected_count();
                    let selected_bytes = scan.selected_total_bytes();

                    let clean_text = if state.is_cleaning_caches {
                        "Cleaning...".to_string()
                    } else if selected_count > 0 {
                        format!(
                            "Clean {} ({})",
                            selected_count,
                            format_bytes(selected_bytes)
                        )
                    } else {
                        "Clean Selected".to_string()
                    };

                    let clean_btn = theme.critical_button(RichText::new(clean_text).size(11.5));

                    let has_selected = selected_count > 0 && !state.is_cleaning_caches;

                    if ui.add_enabled(has_selected, clean_btn).clicked() {
                        let selected_paths: Vec<PathBuf> = scan
                            .items
                            .iter()
                            .filter(|item| item.selected)
                            .map(|item| item.path.clone())
                            .collect();

                        *action = Some(CleanupActionRequest::CleanSelectedCaches(selected_paths));
                    }
                }
            });
        });

        ui.add_space(10.0);

        if let Some(scan) = scan_result {
            // Search & Selection Bar
            ui.horizontal(|ui| {
                render_search_icon(ui, theme.text_secondary, 13.0);
                ui.add_space(4.0);
                ui.add(
                    egui::TextEdit::singleline(&mut state.cache_search_query)
                        .hint_text("Filter caches by name...")
                        .desired_width(220.0),
                );

                ui.add_space(12.0);

                if ui.add(theme.subtle_button("Select Safe")).clicked() {
                    scan.select_safe();
                }

                if ui.add(theme.subtle_button("Select All")).clicked() {
                    scan.select_all();
                }

                if ui.add(theme.subtle_button("Clear Selection")).clicked() {
                    scan.deselect_all();
                }

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(
                        RichText::new(format!(
                            "Total disk cache found: {}",
                            format_bytes(scan.total_bytes)
                        ))
                        .size(12.0)
                        .color(theme.text_secondary),
                    );
                });
            });

            ui.add_space(12.0);

            // Caches Table Header
            ui.horizontal(|ui| {
                ui.add_space(28.0);
                ui.label(
                    RichText::new("Application")
                        .size(11.0)
                        .strong()
                        .color(theme.text_secondary),
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new("Safety")
                            .size(11.0)
                            .strong()
                            .color(theme.text_secondary),
                    );
                    ui.add_space(32.0);
                    ui.label(
                        RichText::new("Disk Size")
                            .size(11.0)
                            .strong()
                            .color(theme.text_secondary),
                    );
                });
            });

            ui.separator();
            ui.add_space(4.0);

            let query = state.cache_search_query.to_lowercase();
            let mut displayed_count = 0;

            for item in &mut scan.items {
                if !query.is_empty()
                    && !item.name.to_lowercase().contains(&query)
                    && !item.id.to_lowercase().contains(&query)
                {
                    continue;
                }

                displayed_count += 1;

                ui.horizontal(|ui| {
                    ui.checkbox(&mut item.selected, "");
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new(&item.name)
                                .size(12.5)
                                .strong()
                                .color(theme.text_primary),
                        );
                        ui.label(
                            RichText::new(&item.description)
                                .size(10.5)
                                .color(theme.text_secondary),
                        );
                    });

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.add_space(8.0);
                        if item.is_safe {
                            ui.label(
                                RichText::new("Safe to Clean")
                                    .size(10.5)
                                    .color(theme.normal_color),
                            );
                        } else {
                            ui.label(
                                RichText::new("Review Needed")
                                    .size(10.5)
                                    .color(theme.warning_color),
                            );
                        }

                        ui.add_space(20.0);
                        ui.label(
                            RichText::new(format_bytes(item.size_bytes))
                                .size(12.0)
                                .strong()
                                .color(theme.text_primary),
                        );
                    });
                });

                ui.add_space(6.0);
            }

            if displayed_count == 0 {
                ui.add_space(12.0);
                ui.label(
                    RichText::new("No caches match your current search query.")
                        .size(12.0)
                        .color(theme.text_secondary),
                );
            }
        } else {
            ui.vertical_centered(|ui| {
                ui.add_space(20.0);
                ui.label(
                    RichText::new("Application caches have not been scanned yet.")
                        .size(12.0)
                        .color(theme.text_secondary),
                );
                ui.add_space(8.0);
                if ui.add(theme.primary_button("Scan Caches Now")).clicked() {
                    *action = Some(CleanupActionRequest::ScanCaches);
                }
                ui.add_space(20.0);
            });
        }
    });
}

/// Renders the RAM & Diagnostics view.
fn render_ram_and_maintenance(
    ui: &mut Ui,
    snapshot: Option<&MemorySnapshot>,
    state: &mut CleanupViewState,
    theme: &AppTheme,
    action: &mut Option<CleanupActionRequest>,
) {
    // RAM Inactive File Cache Card
    theme.card_frame().show(ui, |ui| {
        ui.horizontal(|ui| {
            render_section_header(ui, "System RAM File Cache", theme);

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let btn_label = if state.is_purging_ram {
                    "Purging RAM..."
                } else {
                    "Purge RAM Cache"
                };

                let btn = theme.primary_button(RichText::new(btn_label).size(11.5));

                if ui.add_enabled(!state.is_purging_ram, btn).clicked() {
                    *action = Some(CleanupActionRequest::PurgeRamCache);
                }
            });
        });

        ui.add_space(10.0);

        if let Some(snap) = snapshot {
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = vec2(28.0, 10.0);

                ui.vertical(|ui| {
                    ui.label(
                        RichText::new(format_bytes(snap.cached_files_bytes))
                            .size(23.0)
                            .strong()
                            .color(theme.text_primary),
                    );
                    ui.label(RichText::new("Cached Files in RAM").size(11.0).color(theme.text_secondary));
                });

                ui.vertical(|ui| {
                    ui.label(
                        RichText::new(format_bytes(snap.purgeable_bytes))
                            .size(23.0)
                            .strong()
                            .color(theme.normal_color),
                    );
                    ui.label(RichText::new("Immediately Purgeable").size(11.0).color(theme.text_secondary));
                });

                ui.vertical(|ui| {
                    ui.label(
                        RichText::new(format_bytes(snap.free_bytes))
                            .size(23.0)
                            .strong()
                            .color(theme.text_secondary),
                    );
                    ui.label(RichText::new("Completely Free RAM").size(11.0).color(theme.text_secondary));
                });
            });
        }

        ui.add_space(12.0);

        ui.label(
            RichText::new(
                "How macOS handles RAM cache: macOS retains closed files in RAM as 'Cached Files' so reopening them is instantaneous. Under memory pressure, macOS purges them automatically. Purging forces the XNU kernel to flush inactive buffers back to free memory.",
            )
            .size(11.0)
            .color(theme.text_secondary),
        );
    });

    ui.add_space(16.0);

    // Quick System Diagnostics Card
    theme.card_frame().show(ui, |ui| {
        render_section_header(ui, "Quick System Diagnostics", theme);

        ui.add_space(10.0);

        ui.horizontal(|ui| {
            if ui
                .add(theme.subtle_button(RichText::new("Flush DNS Cache").size(11.5)))
                .clicked()
            {
                *action = Some(CleanupActionRequest::FlushDns);
            }

            ui.add_space(8.0);

            if ui
                .add(theme.subtle_button(RichText::new("Reset QuickLook Thumbnails").size(11.5)))
                .clicked()
            {
                *action = Some(CleanupActionRequest::FlushQuickLook);
            }
        });
    });
}

/// Renders the Large & Old Files Finder view.
fn render_large_files_finder(
    ui: &mut Ui,
    state: &mut CleanupViewState,
    theme: &AppTheme,
    action: &mut Option<CleanupActionRequest>,
) {
    let min_bytes = state.large_files_size_filter.min_bytes();

    // Summary & Control Card
    theme.card_frame().show(ui, |ui| {
        ui.horizontal(|ui| {
            render_section_header(ui, "Large & Old Files Finder", theme);

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let scan_label = if state.is_scanning_large_files {
                    "Scanning Drives & Folders..."
                } else if state.large_files_result.is_none() {
                    "Scan Storage & Volumes"
                } else {
                    "Rescan Storage"
                };

                if ui
                    .add_enabled(
                        !state.is_scanning_large_files,
                        theme.primary_button(RichText::new(scan_label).size(11.5)),
                    )
                    .clicked()
                {
                    *action = Some(CleanupActionRequest::ScanLargeFiles(min_bytes));
                }

                if let Some(ref scan) = state.large_files_result {
                    let selected_files: Vec<PathBuf> = scan
                        .files
                        .iter()
                        .filter(|f| f.selected)
                        .map(|f| f.path.clone())
                        .collect();

                    let selected_count = selected_files.len();
                    let selected_bytes = scan.total_selected_bytes();

                    if selected_count > 0 {
                        let clean_label = format!(
                            "Move to Trash ({} • {})",
                            selected_count,
                            format_bytes(selected_bytes)
                        );
                        let clean_btn = theme.critical_button(RichText::new(clean_label).size(11.5));
                        if ui
                            .add_enabled(!state.is_cleaning_large_files, clean_btn)
                            .clicked()
                        {
                            *action =
                                Some(CleanupActionRequest::DeleteSelectedLargeFiles(selected_files));
                        }
                    }
                }
            });
        });

        ui.add_space(6.0);
        ui.label(
            RichText::new(
                "Uncover huge forgotten video exports, disk images, archives, and untouched files taking up space across user folders and mounted volumes.",
            )
            .size(12.0)
            .color(theme.text_secondary),
        );

        ui.add_space(10.0);

        if let Some(ref scan) = state.large_files_result {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new(format_bytes(scan.total_bytes()))
                            .size(24.0)
                            .strong()
                            .color(theme.accent_color),
                    );
                    ui.label(
                        RichText::new(format!("Total Large Files ({} files)", scan.files.len()))
                            .size(11.5)
                            .color(theme.text_secondary),
                    );
                });

                ui.add_space(32.0);

                let selected_bytes = scan.total_selected_bytes();
                let selected_count = scan.files.iter().filter(|f| f.selected).count();
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new(format_bytes(selected_bytes))
                            .size(24.0)
                            .strong()
                            .color(if selected_bytes > 0 {
                                theme.critical_color
                            } else {
                                theme.text_secondary
                            }),
                    );
                    ui.label(
                        RichText::new(format!("Selected to Trash ({} files)", selected_count))
                            .size(11.5)
                            .color(theme.text_secondary),
                    );
                });
            });
        }
    });

    ui.add_space(14.0);

    // Filter Toolbar & File List
    if let Some(ref mut scan) = state.large_files_result {
        theme.card_frame().show(ui, |ui| {
            // Row 1: Search & Selection Controls
            ui.horizontal(|ui| {
                render_search_icon(ui, theme.text_secondary, 13.0);
                ui.add_space(4.0);
                ui.add(
                    egui::TextEdit::singleline(&mut state.large_files_search_query)
                        .hint_text("Filter files by name...")
                        .desired_width(220.0),
                );

                ui.add_space(12.0);

                if ui.add(theme.subtle_button("Select All Filtered")).clicked() {
                    let q = state.large_files_search_query.to_lowercase();
                    let cat = state.large_files_category_filter;
                    let age = state.large_files_age_filter;
                    let size_min = state.large_files_size_filter.min_bytes();

                    for f in &mut scan.files {
                        let matches_q = q.is_empty() || f.name.to_lowercase().contains(&q);
                        let matches_cat = cat == LargeFileCategory::All || f.category == cat;
                        let matches_age = f.modified_days_ago >= age.min_days();
                        let matches_size = f.size_bytes >= size_min;
                        if matches_q && matches_cat && matches_age && matches_size {
                            f.selected = true;
                        }
                    }
                }

                if ui.add(theme.subtle_button("Clear Selection")).clicked() {
                    for f in &mut scan.files {
                        f.selected = false;
                    }
                }
            });

            ui.add_space(10.0);

            // Row 2: Category Filters
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("Category:")
                        .size(11.5)
                        .strong()
                        .color(theme.text_secondary),
                );

                let categories = [
                    LargeFileCategory::All,
                    LargeFileCategory::DiskImage,
                    LargeFileCategory::Archive,
                    LargeFileCategory::Media,
                    LargeFileCategory::Document,
                    LargeFileCategory::Other,
                ];

                for cat in categories {
                    let is_active = state.large_files_category_filter == cat;
                    let btn = if is_active {
                        theme.primary_button(RichText::new(cat.label()).size(11.0).strong())
                    } else {
                        theme.subtle_button(RichText::new(cat.label()).size(11.0))
                    };
                    if ui.add(btn).clicked() {
                        state.large_files_category_filter = cat;
                    }
                }
            });

            ui.add_space(8.0);

            // Row 3: Size & Age Filters
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("Minimum Size:")
                        .size(11.5)
                        .strong()
                        .color(theme.text_secondary),
                );
                let sizes = [
                    SizeFilter::Min50MB,
                    SizeFilter::Min100MB,
                    SizeFilter::Min500MB,
                    SizeFilter::Min1GB,
                ];
                for s in sizes {
                    let is_active = state.large_files_size_filter == s;
                    let btn = if is_active {
                        theme.primary_button(RichText::new(s.label()).size(11.0).strong())
                    } else {
                        theme.subtle_button(RichText::new(s.label()).size(11.0))
                    };
                    if ui.add(btn).clicked() {
                        state.large_files_size_filter = s;
                    }
                }

                ui.add_space(16.0);
                ui.label(
                    RichText::new("Age:")
                        .size(11.5)
                        .strong()
                        .color(theme.text_secondary),
                );
                let ages = [
                    AgeFilter::Any,
                    AgeFilter::OlderThan1Month,
                    AgeFilter::OlderThan6Months,
                    AgeFilter::OlderThan1Year,
                ];
                for a in ages {
                    let is_active = state.large_files_age_filter == a;
                    let btn = if is_active {
                        theme.primary_button(RichText::new(a.label()).size(11.0).strong())
                    } else {
                        theme.subtle_button(RichText::new(a.label()).size(11.0))
                    };
                    if ui.add(btn).clicked() {
                        state.large_files_age_filter = a;
                    }
                }
            });

            ui.add_space(14.0);

            // Items List
            let q = state.large_files_search_query.to_lowercase();
            let cat = state.large_files_category_filter;
            let age = state.large_files_age_filter;
            let size_min = state.large_files_size_filter.min_bytes();

            let mut visible_count = 0;

            for file in &mut scan.files {
                let matches_q = q.is_empty() || file.name.to_lowercase().contains(&q);
                let matches_cat = cat == LargeFileCategory::All || file.category == cat;
                let matches_age = file.modified_days_ago >= age.min_days();
                let matches_size = file.size_bytes >= size_min;

                if !(matches_q && matches_cat && matches_age && matches_size) {
                    continue;
                }

                visible_count += 1;

                ui.horizontal(|ui| {
                    ui.checkbox(&mut file.selected, "");

                    // Render Category Pill
                    render_category_pill(ui, file.category);

                    ui.add_space(4.0);

                    // Name and path details
                    let parent_display = file
                        .path
                        .parent()
                        .map(|p| p.to_string_lossy().to_string())
                        .unwrap_or_default();

                    let compact_parent = if let Some(home) =
                        std::env::var_os("HOME").and_then(|h| h.into_string().ok())
                    {
                        parent_display.replace(&home, "~")
                    } else {
                        parent_display
                    };

                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new(&file.name)
                                .size(12.5)
                                .strong()
                                .color(theme.text_primary),
                        )
                        .on_hover_text(file.path.to_string_lossy());

                        ui.label(
                            RichText::new(format!(
                                "{} • Modified {}d ago",
                                compact_parent, file.modified_days_ago
                            ))
                            .size(10.5)
                            .color(theme.text_secondary),
                        );
                    });

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        // Quick Trash button
                        if ui
                            .add(
                                theme.subtle_button(
                                    RichText::new("Trash")
                                        .size(11.0)
                                        .color(theme.critical_color),
                                ),
                            )
                            .on_hover_text("Move this file to Trash")
                            .clicked()
                        {
                            *action =
                                Some(CleanupActionRequest::DeleteSelectedLargeFiles(vec![file
                                    .path
                                    .clone()]));
                        }

                        ui.add_space(6.0);

                        // Reveal in Finder button
                        if ui
                            .add(theme.subtle_button(RichText::new("Finder").size(11.0)))
                            .on_hover_text("Reveal file in macOS Finder")
                            .clicked()
                        {
                            *action = Some(CleanupActionRequest::RevealLargeFileInFinder(
                                file.path.clone(),
                            ));
                        }

                        ui.add_space(10.0);

                        // File size
                        ui.label(
                            RichText::new(format_bytes(file.size_bytes))
                                .size(12.5)
                                .strong()
                                .color(theme.text_primary),
                        );
                    });
                });

                ui.add_space(6.0);
            }

            if visible_count == 0 {
                ui.add_space(10.0);
                ui.vertical_centered(|ui| {
                    ui.label(
                        RichText::new("No files match the active filters.")
                            .size(12.0)
                            .color(theme.text_secondary),
                    );
                });
                ui.add_space(10.0);
            }
        });
    } else if !state.is_scanning_large_files {
        theme.card_frame().show(ui, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(20.0);
                ui.label(
                    RichText::new("No Scan Results Yet")
                        .size(16.0)
                        .strong()
                        .color(theme.text_primary),
                );
                ui.add_space(6.0);
                ui.label(
                    RichText::new(
                        "Click 'Scan for Large Files' above to scan your Downloads, Documents, Movies, and Desktop for files over 50 MB.",
                    )
                    .size(12.0)
                    .color(theme.text_secondary),
                );
                ui.add_space(12.0);
                if ui
                    .add(theme.primary_button(RichText::new("Scan for Large Files").size(12.0)))
                    .clicked()
                {
                    *action = Some(CleanupActionRequest::ScanLargeFiles(min_bytes));
                }
                ui.add_space(20.0);
            });
        });
    }
}

fn render_category_pill(ui: &mut Ui, cat: LargeFileCategory) {
    let (label, bg_color, text_color) = match cat {
        LargeFileCategory::DiskImage => (
            "Disk Image",
            Color32::from_rgb(30, 58, 138),
            Color32::from_rgb(147, 197, 253),
        ),
        LargeFileCategory::Archive => (
            "Archive",
            Color32::from_rgb(120, 53, 15),
            Color32::from_rgb(253, 230, 138),
        ),
        LargeFileCategory::Media => (
            "Media",
            Color32::from_rgb(88, 28, 135),
            Color32::from_rgb(216, 180, 254),
        ),
        LargeFileCategory::Document => (
            "Document",
            Color32::from_rgb(19, 78, 74),
            Color32::from_rgb(153, 246, 228),
        ),
        _ => (
            "Other",
            Color32::from_rgb(51, 65, 85),
            Color32::from_rgb(203, 213, 225),
        ),
    };

    let padding = vec2(6.0, 2.0);
    let font_id = egui::FontId::proportional(10.0);
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_string(), font_id, text_color);
    let size = galley.size() + padding * 2.0;

    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    ui.painter().rect(
        rect,
        CornerRadius::same(4),
        bg_color,
        Stroke::NONE,
        egui::StrokeKind::Inside,
    );
    ui.painter().galley(rect.min + padding, galley, text_color);
}
