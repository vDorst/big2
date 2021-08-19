use crate::big2rules::cards::Cards;
use eframe::egui::{self, Align2, Color32, Stroke, TextStyle};

pub fn board_ui(ui: &mut egui::Ui, value: &u64) -> egui::Response {
    let card_size = ui.spacing().interact_size.y * egui::vec2(4.0, 7.2);

    ui.horizontal_wrapped(|ui| {
        for (_bit, mask) in Cards::board_from(*value).unwrap() {
            let (rect, response) = ui.allocate_exact_size(card_size, egui::Sense::hover());

            let mut visuals = *ui.style().visuals.widgets.style(&response);
            visuals.bg_fill = Color32::WHITE;
            visuals.fg_stroke = Stroke::new(1.0, Color32::BLACK);

            // All coordinates are in absolute screen coordinates so we use `rect` to place the elements.
            let rect = rect.expand(visuals.expansion);

            ui.painter()
                .rect(rect, 10.0, visuals.bg_fill, visuals.fg_stroke);

            let c = Cards::board_from(mask).unwrap();
            let s = c.to_string();
            let col = match c.suit() {
                0 => Color32::BLUE,
                1 => Color32::from_rgb(0, 0xA0, 0x0),
                2 => Color32::RED,
                3 => Color32::BLACK,
                _ => Color32::YELLOW,
            };
            ui.painter().text(
                rect.center(),
                Align2::CENTER_CENTER,
                s,
                TextStyle::Button,
                col,
            );
            //.circle(center, 0.75 * radius, visuals.bg_fill, visuals.fg_stroke);
        }
    })
    .response
}

// A wrapper that allows the more idiomatic usage pattern: `ui.add(...)`
/// cards entry field with ability to toggle character hiding.
///
/// ## Example:
/// ``` ignore
/// ui.add(cards(&mut selected, &str value));
/// ```
pub fn board<'a>(value: &'a u64) -> impl egui::Widget + 'a {
    move |ui: &mut egui::Ui| board_ui(ui, value)
}
