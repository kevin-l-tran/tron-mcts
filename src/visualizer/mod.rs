pub mod app;
mod board;
mod controls;

use std::sync::mpsc::Receiver;

use crate::{runner::replay::Replay, visualizer::app::MatchEvent};

pub fn run(replay_factory: fn() -> (Replay, Receiver<MatchEvent>)) -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default().with_inner_size([1180.0, 850.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Tron Replay Viewer",
        options,
        Box::new(move |cc| {
            cc.egui_ctx.set_visuals(eframe::egui::Visuals::dark());
            Ok(Box::new(app::VisualizerApp::new(replay_factory)))
        }),
    )
}
