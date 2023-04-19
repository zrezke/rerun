use crate::env_vars::RERUN_TRACK_ALLOCATIONS;
use egui::emath::History;
use egui::plot::{Line, Plot, PlotPoints};
use instant::Instant;
use itertools::Itertools;
use re_arrow_store::{DataStoreConfig, DataStoreRowStats, DataStoreStats};
use re_format::{format_bytes, format_number};
// ----------------------------------------------------------------------------

pub struct BandwidthPanel {
    history: History<u64>,
    start_time: Instant,
}

impl Default for BandwidthPanel {
    fn default() -> Self {
        Self {
            history: History::new(0..1000, 5.0),
            start_time: Instant::now(),
        }
    }
}

impl BandwidthPanel {
    /// Call once per frame
    pub fn update(&mut self, bandwidth: u64) {
        crate::profile_function!();
        self.history
            .add(self.start_time.elapsed().as_nanos() as f64 / 1e9, bandwidth);
    }

    pub fn ui(&self, ui: &mut egui::Ui) {
        crate::profile_function!();

        ui.ctx().request_repaint();

        egui::SidePanel::left("not_the_plot")
            .resizable(false)
            .min_width(250.0)
            .default_width(300.0)
            .show_inside(ui, |ui| {});

        egui::CentralPanel::default().show_inside(ui, |ui| {
            ui.label("Bandwidth");
            self.plot(ui);
        });
    }

    fn plot(&self, ui: &mut egui::Ui) {
        crate::profile_function!();
        Plot::new("bandwidth_plot").show(ui, |ui| {
            ui.line(Line::new(PlotPoints::new(
                self.history
                    .iter()
                    .map(|(x, y)| [x, y as f64])
                    .collect_vec(),
            )));
        });
    }
}
