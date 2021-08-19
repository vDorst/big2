use eframe::egui::{self, Align2, Color32, Stroke, TextStyle};

pub fn button_pass_ui(ui: &mut egui::Ui, selected: &mut bool, passed: bool) -> egui::Response {
    let button_size = ui.spacing().interact_size.y * egui::vec2(4.0, 2.0);

    let (rect, mut response) = ui.allocate_exact_size(button_size, egui::Sense::click());

    if !passed {
        if response.clicked() {
            *selected = !*selected;
            response.mark_changed(); // report back that the value changed
        }
    }

    let select = !passed & *selected;
    // 4. Paint!
    // First let's ask for a simple animation from egui.
    // egui keeps track of changes in the boolean associated with the id and
    // returns an animated value in the 0-1 range for how much "on" we are.
    // let how_on = ui.ctx().animate_bool(response.id, select);
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

    ui.painter()
        .rect(rect, 1.0, visuals.bg_fill, visuals.fg_stroke);

    let col = match (passed, select) {
        (true, _) => Color32::GRAY,
        (false, false) => Color32::RED,
        (false, true) => Color32::BLUE,
    };
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        "Pass",
        TextStyle::Heading,
        col,
    );
    response
}

pub fn button_play_ui(ui: &mut egui::Ui, play: &mut bool, can_play: bool) -> egui::Response {
    let button_size = ui.spacing().interact_size.y * egui::vec2(4.0, 2.0);

    let (rect, mut response) = ui.allocate_exact_size(button_size, egui::Sense::click());

    if can_play {
        if response.clicked() {
            *play = true;
            response.mark_changed(); // report back that the value changed
        }
    }

    let select = can_play;
    // 4. Paint!
    // First let's ask for a simple animation from egui.
    // egui keeps track of changes in the boolean associated with the id and
    // returns an animated value in the 0-1 range for how much "on" we are.
    // let how_on = ui.ctx().animate_bool(response.id, select);
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

    ui.painter()
        .rect(rect, 1.0, visuals.bg_fill, visuals.fg_stroke);

    let col = if can_play {
        Color32::RED
    } else {
        Color32::GRAY
    };

    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        "Play",
        TextStyle::Heading,
        col,
    );
    //.circle(center, 0.75 * radius, visuals.bg_fill, visuals.fg_stroke);

    response
}

// A wrapper that allows the more idiomatic usage pattern: `ui.add(...)`
/// cards entry field with ability to toggle character hiding.
///
/// ## Example:
/// ``` ignore
/// ui.add(cards(&mut selected, &str value));
/// ```
pub fn button_pass<'a>(want_pass: &'a mut bool, passed: bool) -> impl egui::Widget + 'a {
    move |ui: &mut egui::Ui| button_pass_ui(ui, want_pass, passed)
}

pub fn button_play<'a>(want_play: &'a mut bool, can_play: bool) -> impl egui::Widget + 'a {
    move |ui: &mut egui::Ui| button_play_ui(ui, want_play, can_play)
}
