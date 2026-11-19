use eframe::egui::{self, RichText};

use crate::{engine::state::GameState, runner::replay::ReplayPlayer};

use super::board::player_color;

pub(super) const SPEEDS: [f32; 7] = [1.0, 2.0, 5.0, 10.0, 20.0, 40.0, 60.0];

pub(super) enum Command {
    PlayPause,
    First,
    Previous,
    Next,
    Last,
    Seek(usize),
}

pub(super) fn draw_playback(
    ui: &mut egui::Ui,
    player: &ReplayPlayer,
    playing: bool,
    speed_index: &mut usize,
) -> Option<Command> {
    let mut command = None;
    let mut tick = player.tick();

    if ui
        .add(egui::Slider::new(&mut tick, 0..=player.total_ticks()).text("Tick"))
        .changed()
    {
        command = Some(Command::Seek(tick));
    }

    ui.horizontal(|ui| {
        if ui.button("|<").clicked() {
            command = Some(Command::First);
        }
        if ui.button("<").clicked() {
            command = Some(Command::Previous);
        }
        let label = if playing {
            "Pause"
        } else if player.tick() == player.total_ticks() || player.state().is_terminal() {
            "Restart"
        } else {
            "Play"
        };
        if ui.button(label).clicked() {
            command = Some(Command::PlayPause);
        }
        if ui.button(">").clicked() {
            command = Some(Command::Next);
        }
        if ui.button(">|").clicked() {
            command = Some(Command::Last);
        }

        ui.separator();
        egui::ComboBox::from_id_salt("playback_speed")
            .selected_text(format!("{} ticks/s", SPEEDS[*speed_index] as usize))
            .show_ui(ui, |ui| {
                for (index, speed) in SPEEDS.iter().enumerate() {
                    ui.selectable_value(speed_index, index, format!("{} ticks/s", *speed as usize));
                }
            });
    });

    ui.label(
        RichText::new("Space: play/pause  |  Left/Right: step  |  Up/Down: speed  |  R: restart")
            .small()
            .weak(),
    );

    command
}

pub(super) fn draw_players(ui: &mut egui::Ui, state: &GameState) {
    ui.heading("Players");
    ui.separator();

    egui::ScrollArea::vertical().show(ui, |ui| {
        for (id, player) in state.players.iter().enumerate() {
            ui.horizontal(|ui| {
                ui.colored_label(player_color(id), "\u{25cf}");
                ui.label(format!("Player {}", id + 1));
                let status = match player.eliminated_at {
                    Some(tick) => format!("OUT {tick}"),
                    None => "ALIVE".to_string(),
                };
                ui.label(RichText::new(status).weak().small());
            });
        }
    });
}
