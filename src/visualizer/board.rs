use eframe::egui::{self, Color32, Rect, Sense, Stroke, pos2, vec2};

use crate::engine::state::GameState;

const EMPTY: Color32 = Color32::from_rgb(17, 27, 43);
const OBSTACLE: Color32 = Color32::from_rgb(100, 116, 139);
const COLLISION: Color32 = Color32::from_rgb(226, 232, 240);

const PLAYER_COLORS: [Color32; 12] = [
    Color32::from_rgb(56, 189, 248),
    Color32::from_rgb(251, 113, 133),
    Color32::from_rgb(74, 222, 128),
    Color32::from_rgb(251, 191, 36),
    Color32::from_rgb(192, 132, 252),
    Color32::from_rgb(45, 212, 191),
    Color32::from_rgb(249, 115, 22),
    Color32::from_rgb(163, 230, 53),
    Color32::from_rgb(129, 140, 248),
    Color32::from_rgb(244, 114, 182),
    Color32::from_rgb(34, 211, 238),
    Color32::from_rgb(250, 204, 21),
];

pub(super) fn player_color(id: usize) -> Color32 {
    PLAYER_COLORS[id % PLAYER_COLORS.len()]
}

pub(super) fn draw(ui: &mut egui::Ui, state: &GameState) {
    let arena = &state.arena;
    if arena.width == 0 || arena.height == 0 {
        return;
    }

    let (available, _) = ui.allocate_exact_size(ui.available_size(), Sense::hover());
    let available = available.shrink(8.0);
    let cell =
        (available.width() / arena.width as f32).min(available.height() / arena.height as f32);
    if cell <= 0.0 {
        return;
    }

    let board_size = vec2(arena.width as f32 * cell, arena.height as f32 * cell);
    let origin = available.center() - board_size / 2.0;
    let board = Rect::from_min_size(origin, board_size);
    let painter = ui.painter();

    painter.rect_filled(board, 2.0, EMPTY);

    if cell >= 10.0 {
        let grid_color = Color32::from_rgb(27, 40, 59);
        for x in 1..arena.width {
            let px = origin.x + x as f32 * cell;
            painter.line_segment(
                [pos2(px, board.top()), pos2(px, board.bottom())],
                Stroke::new(0.5, grid_color),
            );
        }
        for y in 1..arena.height {
            let py = origin.y + y as f32 * cell;
            painter.line_segment(
                [pos2(board.left(), py), pos2(board.right(), py)],
                Stroke::new(0.5, grid_color),
            );
        }
    }

    let padding = if cell >= 5.0 { 0.6 } else { 0.0 };
    for y in 0..arena.height {
        for x in 0..arena.width {
            let trail = state.trails[(x, y)];
            let color = if arena[(x, y)] {
                Some(OBSTACLE)
            } else if trail == u8::MAX {
                Some(COLLISION)
            } else if trail != 0 {
                Some(player_color(trail as usize - 1))
            } else {
                None
            };

            if let Some(color) = color {
                let position = origin + vec2(x as f32 * cell + padding, y as f32 * cell + padding);
                let size = vec2(
                    (cell - 2.0 * padding).max(0.5),
                    (cell - 2.0 * padding).max(0.5),
                );
                painter.rect_filled(Rect::from_min_size(position, size), 0.0, color);
            }
        }
    }

    for (id, player) in state.players.iter().enumerate() {
        if player.eliminated_at.is_some() {
            continue;
        }

        let center = origin
            + vec2(
                (player.x as f32 + 0.5) * cell,
                (player.y as f32 + 0.5) * cell,
            );
        painter.circle_filled(center, cell * 0.41, Color32::WHITE);
        painter.circle_filled(center, cell * 0.28, player_color(id));

        if cell > 7.0 {
            let (dx, dy) = player.direction.delta();
            let end = center + vec2(dx as f32, dy as f32) * (cell * 0.28);
            painter.line_segment(
                [center, end],
                Stroke::new((cell * 0.11).max(1.0), Color32::WHITE),
            );
        }
    }
}
