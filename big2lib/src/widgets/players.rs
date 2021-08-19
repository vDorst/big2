use eframe::egui::{self, Align2, Color32, Stroke, TextStyle};

use crate::players::PlayerStatus;

pub fn player_ui(
    ui: &mut egui::Ui,
    name: &str,
    num_cards: u8,
    score: i16,
    player_status: PlayerStatus,
) -> egui::Response {
    let card_size = ui.spacing().interact_size.y * egui::vec2(8.0, 3.0);

    let (rect, response) = ui.allocate_exact_size(card_size, egui::Sense::nothing());

    let mut visuals = *ui.style().visuals.widgets.style(&response);
    visuals.bg_fill = Color32::from_rgb(0, 100, 0);
    visuals.fg_stroke = Stroke::new(2.0, Color32::GREEN);

    match player_status {
        PlayerStatus::Normal => (),
        PlayerStatus::Passed => {
            visuals.bg_fill = Color32::LIGHT_GRAY;
            visuals.fg_stroke = Stroke::new(2.0, Color32::YELLOW);
        }
        PlayerStatus::Ready => {
            visuals.bg_fill = Color32::LIGHT_BLUE;
            visuals.fg_stroke = Stroke::new(2.0, Color32::YELLOW);
        }
        PlayerStatus::ToAct => {
            visuals.fg_stroke = Stroke::new(2.0, Color32::YELLOW);
        }
    }

    ui.painter()
        .rect(rect, 1.0, visuals.bg_fill, visuals.fg_stroke);

    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        name,
        TextStyle::Heading,
        Color32::YELLOW,
    );

    response
}

// A wrapper that allows the more idiomatic usage pattern: `ui.add(...)`
/// cards entry field with ability to toggle character hiding.
///
/// ## Example:
/// ``` ignore
/// ui.add(cards(&mut selected, &str value));
/// ```
pub fn player<'a>(
    name: &'a str,
    num_cards: u8,
    score: i16,
    ps: PlayerStatus,
) -> impl egui::Widget + 'a {
    move |ui: &mut egui::Ui| player_ui(ui, name, num_cards, score, ps)
}
