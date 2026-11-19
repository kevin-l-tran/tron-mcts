use std::time::{Duration, Instant};

use eframe::egui::{self, Key};

use crate::runner::replay::{Replay, ReplayPlayer};

use super::{
    board,
    controls::{self, Command, SPEEDS},
};

pub(super) struct VisualizerApp {
    player: ReplayPlayer,
    replay_factory: fn() -> Replay,
    playing: bool,
    speed_index: usize,
    elapsed: f32,
    last_update: Instant,
    error: Option<String>,
}

impl VisualizerApp {
    pub(super) fn new(player: ReplayPlayer, replay_factory: fn() -> Replay) -> Self {
        Self {
            player,
            replay_factory,
            playing: false,
            speed_index: 4,
            elapsed: 0.0,
            last_update: Instant::now(),
            error: None,
        }
    }

    fn new_game(&mut self) {
        let replay = (self.replay_factory)();
        let player = ReplayPlayer::new(replay).expect("replay factory should create valid replays");

        self.player = player;
        self.playing = true;
        self.elapsed = 0.0;
        self.last_update = Instant::now();
        self.error = None;
    }

    fn seek(&mut self, tick: usize) -> bool {
        match self.player.seek(tick) {
            Ok(()) => {
                self.error = None;
                true
            }
            Err(error) => {
                self.error = Some(format!("Could not seek replay: {error:?}"));
                false
            }
        }
    }

    fn advance(&mut self) -> bool {
        match self.player.step_forward() {
            Ok(moved) => {
                self.error = None;
                moved
            }
            Err(error) => {
                self.error = Some(format!("Could not advance replay: {error:?}"));
                false
            }
        }
    }

    fn command(&mut self, command: Command) {
        self.elapsed = 0.0;
        self.last_update = Instant::now();

        match command {
            Command::NewGame => self.new_game(),
            Command::PlayPause => {
                if !self.playing
                    && (self.player.tick() == self.player.total_ticks()
                        || self.player.state().is_terminal())
                    && !self.seek(0)
                {
                    return;
                }
                self.playing = !self.playing;
            }
            Command::First => {
                self.playing = false;
                self.seek(0);
            }
            Command::Previous => {
                self.playing = false;
                if let Err(error) = self.player.step_backward() {
                    self.error = Some(format!("Could not rewind replay: {error:?}"));
                } else {
                    self.error = None;
                }
            }
            Command::Next => {
                self.playing = false;
                self.advance();
            }
            Command::Last => {
                self.playing = false;
                self.seek(self.player.total_ticks());
            }
            Command::Seek(tick) => {
                self.playing = false;
                self.seek(tick);
            }
        }
    }

    fn handle_keyboard(&mut self, ctx: &egui::Context) {
        let command = ctx.input(|input| {
            if input.key_pressed(Key::N) {
                Some(Command::NewGame)
            } else if input.key_pressed(Key::Space) {
                Some(Command::PlayPause)
            } else if input.key_pressed(Key::ArrowRight) || input.key_pressed(Key::D) {
                Some(Command::Next)
            } else if input.key_pressed(Key::ArrowLeft) || input.key_pressed(Key::A) {
                Some(Command::Previous)
            } else if input.key_pressed(Key::R) {
                Some(Command::First)
            } else {
                None
            }
        });

        if let Some(command) = command {
            self.command(command);
        }

        ctx.input(|input| {
            if input.key_pressed(Key::ArrowUp) || input.key_pressed(Key::W) {
                self.speed_index = (self.speed_index + 1).min(SPEEDS.len() - 1);
            }
            if input.key_pressed(Key::ArrowDown) || input.key_pressed(Key::S) {
                self.speed_index = self.speed_index.saturating_sub(1);
            }
        });
    }

    fn update_playback(&mut self, ctx: &egui::Context) {
        let now = Instant::now();
        let delta = now.duration_since(self.last_update).as_secs_f32().min(0.25);
        self.last_update = now;

        if !self.playing {
            return;
        }

        self.elapsed += delta;
        let seconds_per_tick = 1.0 / SPEEDS[self.speed_index];
        while self.elapsed >= seconds_per_tick {
            self.elapsed -= seconds_per_tick;
            if !self.advance()
                || self.player.tick() == self.player.total_ticks()
                || self.player.state().is_terminal()
            {
                self.playing = false;
                self.elapsed = 0.0;
                break;
            }
        }

        if self.playing {
            ctx.request_repaint_after(Duration::from_millis(16));
        }
    }
}

impl eframe::App for VisualizerApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.handle_keyboard(&ctx);
        self.update_playback(&ctx);

        egui::Panel::top("header").resizable(false).show(ui, |ui| {
            ui.heading("TRON / REPLAY");
            ui.label(format!(
                "Tick {} / {}   |   {} alive",
                self.player.tick(),
                self.player.total_ticks(),
                self.player.state().alive_count,
            ));
        });

        let command = egui::Panel::bottom("playback")
            .resizable(false)
            .show(ui, |ui| {
                let command =
                    controls::draw_playback(ui, &self.player, self.playing, &mut self.speed_index);
                if let Some(error) = &self.error {
                    ui.colored_label(egui::Color32::LIGHT_RED, error);
                }
                command
            })
            .inner;

        if let Some(command) = command {
            self.command(command);
        }

        egui::Panel::right("players")
            .default_size(230.0)
            .show(ui, |ui| controls::draw_players(ui, self.player.state()));

        egui::CentralPanel::default().show(ui, |ui| {
            board::draw(ui, self.player.state());
        });
    }
}
