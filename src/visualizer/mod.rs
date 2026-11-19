mod app;
mod board;
mod controls;

use crate::runner::replay::{Replay, ReplayPlayer};

pub fn run(replay_factory: fn() -> Replay) -> eframe::Result {
    let player =
        ReplayPlayer::new(replay_factory()).expect("replay factory must construct a valid initial state");

    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default().with_inner_size([1180.0, 850.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Tron Replay Viewer",
        options,
        Box::new(move |cc| {
            cc.egui_ctx.set_visuals(eframe::egui::Visuals::dark());
            Ok(Box::new(app::VisualizerApp::new(player, replay_factory)))
        }),
    )
}
