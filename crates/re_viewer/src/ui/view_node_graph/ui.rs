use std::collections::BTreeMap;

use egui::{Color32, RichText};

use re_data_store::{EntityPath, Timeline};
use re_log_types::TimePoint;

use crate::ViewerContext;

use super::{NodeGraphEntry, SceneNodeGraph};
// --- Main view ---

#[derive(Clone, Default, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct ViewNodeGraphState {
    /// Keeps track of the latest time selection made by the user.
    ///
    /// We need this because we want the user to be able to manually scroll the
    /// NodeGraph entry window however they please when the time cursor isn't moving.
    latest_time: i64,

    pub filters: ViewNodeGraphFilters,

    monospace: bool,
}

impl ViewNodeGraphState {
    pub fn selection_ui(&mut self, re_ui: &re_ui::ReUi, ui: &mut egui::Ui) {
        crate::profile_function!();
        re_log::info!("Holda from node graph");
    }
}

pub(crate) fn view_node_graph(
    ctx: &mut ViewerContext<'_>,
    ui: &mut egui::Ui,
    state: &mut ViewNodeGraphState,
    scene: &SceneNodeGraph,
) -> egui::Response {
    crate::profile_function!();

    ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
        if ui.button("Button text").clicked() {
            re_log::info!("Holda from node graph");
        }
    })
    .response
}

// --- Filters ---

// TODO(cmc): implement "body contains <value>" filter.
// TODO(cmc): beyond filters, it'd be nice to be able to swap columns at some point.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ViewNodeGraphFilters {
    // Column filters: which columns should be visible?
    // Timelines are special: each one has a dedicated column.
    pub col_timelines: BTreeMap<Timeline, bool>,
    pub col_entity_path: bool,
    pub col_log_level: bool,

    // Row filters: which rows should be visible?
    pub row_entity_paths: BTreeMap<EntityPath, bool>,
    pub row_log_levels: BTreeMap<String, bool>,
}

impl Default for ViewNodeGraphFilters {
    fn default() -> Self {
        Self {
            col_entity_path: true,
            col_log_level: true,
            col_timelines: Default::default(),
            row_entity_paths: Default::default(),
            row_log_levels: Default::default(),
        }
    }
}

impl ViewNodeGraphFilters {
    pub fn is_entity_path_visible(&self, entity_path: &EntityPath) -> bool {
        self.row_entity_paths
            .get(entity_path)
            .copied()
            .unwrap_or(true)
    }

    pub fn is_log_level_visible(&self, level: &str) -> bool {
        self.row_log_levels.get(level).copied().unwrap_or(true)
    }

    // Checks whether new values are available for any of the filters, and updates everything
    // accordingly.
    fn update(&mut self, ctx: &mut ViewerContext<'_>, NodeGraph_entries: &[NodeGraphEntry]) {
        crate::profile_function!();

        let Self {
            col_timelines,
            col_entity_path: _,
            col_log_level: _,
            row_entity_paths,
            row_log_levels,
        } = self;

        for timeline in ctx.log_db.timelines() {
            col_timelines.entry(*timeline).or_insert(true);
        }

        for entity_path in NodeGraph_entries.iter().map(|te| &te.entity_path) {
            row_entity_paths.entry(entity_path.clone()).or_insert(true);
        }

        for level in NodeGraph_entries.iter().filter_map(|te| te.level.as_ref()) {
            row_log_levels.entry(level.clone()).or_insert(true);
        }
    }
}

// ---

fn get_time_point(ctx: &ViewerContext<'_>, entry: &NodeGraphEntry) -> Option<TimePoint> {
    if let Some(time_point) = ctx
        .log_db
        .entity_db
        .data_store
        .get_msg_metadata(&entry.msg_id)
    {
        Some(time_point.clone())
    } else {
        re_log::warn_once!("Missing LogMsg for {:?}", entry.entity_path);
        None
    }
}

/// `scroll_to_row` indicates how far down we want to scroll in terms of logical rows,
/// as opposed to `scroll_to_offset` (computed below) which is how far down we want to
/// scroll in terms of actual points.
fn table_ui(
    ctx: &mut ViewerContext<'_>,
    ui: &mut egui::Ui,
    state: &mut ViewNodeGraphState,
    NodeGraph_entries: &[NodeGraphEntry],
    scroll_to_row: Option<usize>,
) {
    let timelines = state
        .filters
        .col_timelines
        .iter()
        .filter_map(|(timeline, visible)| visible.then_some(timeline))
        .collect::<Vec<_>>();

    use egui_extras::Column;

    let global_timeline = *ctx.rec_cfg.time_ctrl.timeline();
    let global_time = ctx.rec_cfg.time_ctrl.time_int();

    let mut table_builder = egui_extras::TableBuilder::new(ui)
        .resizable(true)
        .vscroll(true)
        .auto_shrink([false; 2]) // expand to take up the whole Space View
        .min_scrolled_height(0.0) // we can go as small as we need to be in order to fit within the space view!
        .max_scroll_height(f32::INFINITY) // Fill up whole height
        .cell_layout(egui::Layout::left_to_right(egui::Align::TOP));

    if let Some(scroll_to_row) = scroll_to_row {
        table_builder = table_builder.scroll_to_row(scroll_to_row, Some(egui::Align::Center));
    }

    let mut body_clip_rect = None;
    let mut current_time_y = None; // where to draw the current time indicator cursor

    {
        // timeline(s)
        table_builder =
            table_builder.columns(Column::auto().clip(true).at_least(32.0), timelines.len());

        // entity path
        if state.filters.col_entity_path {
            table_builder = table_builder.column(Column::auto().clip(true).at_least(32.0));
        }
        // log level
        if state.filters.col_log_level {
            table_builder = table_builder.column(Column::auto().at_least(30.0));
        }
        // body
        table_builder = table_builder.column(Column::remainder().at_least(100.0));
    }
    table_builder
        .header(re_ui::ReUi::table_header_height(), |mut header| {
            re_ui::ReUi::setup_table_header(&mut header);
            for timeline in &timelines {
                header.col(|ui| {
                    ctx.timeline_button(ui, timeline);
                });
            }
            if state.filters.col_entity_path {
                header.col(|ui| {
                    ui.strong("Entity path");
                });
            }
            if state.filters.col_log_level {
                header.col(|ui| {
                    ui.strong("Level");
                });
            }
            header.col(|ui| {
                ui.strong("Body");
            });
        })
        .body(|mut body| {
            re_ui::ReUi::setup_table_body(&mut body);

            body_clip_rect = Some(body.max_rect());

            let row_heights = NodeGraph_entries.iter().map(calc_row_height);
            body.heterogeneous_rows(row_heights, |index, mut row| {
                let NodeGraph_entry = &NodeGraph_entries[index];

                // NOTE: `try_from_props` is where we actually fetch data from the underlying
                // store, which is a costly operation.
                // Doing this here guarantees that it only happens for visible rows.
                let Some(time_point) = get_time_point(ctx, NodeGraph_entry) else {
                    row.col(|ui| {
                        ui.colored_label(
                            Color32::RED,
                            "<failed to load NodeGraphEntry from data store>",
                        );
                    });
                    return;
                };

                // timeline(s)
                for timeline in &timelines {
                    row.col(|ui| {
                        if let Some(row_time) = time_point.get(timeline).copied() {
                            ctx.time_button(ui, timeline, row_time);

                            if let Some(global_time) = global_time {
                                if *timeline == &global_timeline {
                                    #[allow(clippy::comparison_chain)]
                                    if global_time < row_time {
                                        // We've past the global time - it is thus above this row.
                                        if current_time_y.is_none() {
                                            current_time_y = Some(ui.max_rect().top());
                                        }
                                    } else if global_time == row_time {
                                        // This row is exactly at the current time.
                                        // We could draw the current time exactly onto this row, but that would look bad,
                                        // so let's draw it under instead. It looks better in the "following" mode.
                                        current_time_y = Some(ui.max_rect().bottom());
                                    }
                                }
                            }
                        }
                    });
                }

                // path
                if state.filters.col_entity_path {
                    row.col(|ui| {
                        ctx.entity_path_button(ui, None, &NodeGraph_entry.entity_path);
                    });
                }
                // body
                row.col(|ui| {
                    let mut some_text = egui::RichText::new(&NodeGraph_entry.body);

                    if state.monospace {
                        some_text = some_text.monospace();
                    }
                    if let Some([r, g, b, a]) = NodeGraph_entry.color {
                        some_text = some_text.color(Color32::from_rgba_unmultiplied(r, g, b, a));
                    }

                    ui.label(some_text);
                });
            });
        });

    // TODO(cmc): this draws on top of the headers :(
    if let (Some(body_clip_rect), Some(current_time_y)) = (body_clip_rect, current_time_y) {
        // Show that the current time is here:
        ui.painter().with_clip_rect(body_clip_rect).hline(
            ui.max_rect().x_range(),
            current_time_y,
            (1.0, Color32::WHITE),
        );
    }
}

fn calc_row_height(entry: &NodeGraphEntry) -> f32 {
    // Simple, fast, ugly, and functional
    let num_newlines = entry.body.bytes().filter(|&c| c == b'\n').count();
    let num_rows = 1 + num_newlines;
    num_rows as f32 * re_ui::ReUi::table_line_height()
}
