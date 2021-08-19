use crate::big2rules::cards::Cards;
use eframe::egui::{self, Align2, Color32, Stroke, TextStyle};

pub fn cards_ui(ui: &mut egui::Ui, selected: &mut u64, value: u64) -> egui::Response {
    let card_size = ui.spacing().interact_size.y * egui::vec2(3.0, 8.0);

    ui.horizontal_wrapped(|ui| {
        let cards = Cards::hand_from(value);
        if let Ok(cards) = cards {
            for (_bit, mask) in cards {
                let (rect, mut response) = ui.allocate_exact_size(card_size, egui::Sense::click());

                if response.clicked() {
                    *selected = *selected ^ mask;
                    response.mark_changed(); // report back that the value changed
                }

                if response.secondary_clicked() {
                    *selected = 0;
                    response.mark_changed(); // report back that the value changed
                }

                let select = *selected & mask != 0;
                // 4. Paint!
                // First let's ask for a simple animation from egui.
                // egui keeps track of changes in the boolean associated with the id and
                // returns an animated value in the 0-1 range for how much "on" we are.
                let how_on = ui.ctx().animate_bool(response.id, select);
                // We will follow the current style by asking
                // "how should something that is being interacted with be painted?".
                // This will, for instance, give us different colors when the widget is hovered or clicked.

                let mut visuals = *ui.style().visuals.widgets.style(&response);
                visuals.bg_fill = Color32::WHITE;
                visuals.fg_stroke = Stroke::new(1.0, Color32::BLACK);
                if select {
                    visuals.bg_fill = Color32::LIGHT_GRAY;
                    // visuals.bg_stroke = self.visuals.selection.stroke;
                    visuals.fg_stroke = Stroke::new(2.0, Color32::BLUE);
                }

                // All coordinates are in absolute screen coordinates so we use `rect` to place the elements.
                let rect = rect.expand(visuals.expansion);

                let mut rect_card = rect.clone();
                let height = rect_card.height() * 0.9;

                let card_y = egui::lerp((rect.bottom() - height)..=rect.top(), how_on);

                rect_card.set_top(card_y);
                rect_card.set_height(height);

                ui.painter()
                    .rect(rect, 0.0, Color32::GREEN, Stroke::default());
                ui.painter()
                    .rect(rect_card, 10.0, visuals.bg_fill, visuals.fg_stroke);

                let c = Cards::hand_from(mask).unwrap();
                let s = c.to_string();
                let col = match c.suit() {
                    0 => Color32::BLUE,
                    1 => Color32::from_rgb(0, 0xA0, 0x0),
                    2 => Color32::RED,
                    3 => Color32::BLACK,
                    _ => Color32::YELLOW,
                };
                ui.painter().text(
                    rect_card.center(),
                    Align2::CENTER_CENTER,
                    s,
                    TextStyle::Button,
                    col,
                );
                //.circle(center, 0.75 * radius, visuals.bg_fill, visuals.fg_stroke);
            }
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
pub fn cards<'a>(selected: &'a mut u64, value: u64) -> impl egui::Widget + 'a {
    move |ui: &mut egui::Ui| cards_ui(ui, selected, value)
}
