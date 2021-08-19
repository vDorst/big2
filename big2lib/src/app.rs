use crate::{
    messages::{GameState, RoomInfo, MAX_PLAYERS},
    players::{Player, PlayerStatus},
    widgets,
};
use eframe::{egui, epi};

// /// We derive Deserialize/Serialize so we can persist app state on shutdown.
// #[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
// #[cfg_attr(feature = "persistence", serde(default))] // if we add new fields, give them default values when deserializing old state
pub struct TemplateApp {
    pub ri: RoomInfo,
    pub cards_selected: u64,
    pub want_pass: bool,
}

impl epi::App for TemplateApp {
    fn name(&self) -> &str {
        concat!(
            env!("CARGO_PKG_NAME"),
            " (Alpha) ",
            env!("CARGO_PKG_VERSION")
        )
    }

    /// Called by the framework to load old app state (if any).
    // #[cfg(feature = "persistence")]
    // fn setup(
    //     &mut self,
    //     _ctx: &egui::CtxRef,
    //     _frame: &mut epi::Frame<'_>,
    //     storage: Option<&dyn epi::Storage>,
    // ) {
    //     if let Some(storage) = storage {
    //         *self = epi::get_value(storage, epi::APP_KEY).unwrap_or_default()
    //     }
    // }

    /// Called by the frame work to save state before shutdown.
    // #[cfg(feature = "persistence")]
    // fn save(&mut self, storage: &mut dyn epi::Storage) {
    //     epi::set_value(storage, epi::APP_KEY, self);
    // }

    /// Called each time the UI needs repainting, which may be many times per second.
    /// Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
    fn update(&mut self, ctx: &egui::CtxRef, _frame: &mut epi::Frame<'_>) {
        let Self {
            ri,
            cards_selected,
            want_pass,
        } = self;

        let cards_board = &ri.update.board;

        let fake_hand = 0_u64;
        let cards_hand = &ri.update.hand.unwrap_or(fake_hand);

        // egui::SidePanel::left("side_panel").show(ctx, |ui| {
        //     ui.heading("Side Panel");

        //     ui.horizontal(|ui| {
        //         ui.label("Write something: ");
        //         ui.text_edit_singleline(label);
        //     });

        //     ui.add(egui::Slider::new(value, 0.0..=10.0).text("value"));
        //     if ui.button("Increment").clicked() {
        //         *value += 1.0;
        //     }

        //     ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
        //         ui.add(
        //             egui::Hyperlink::new("https://github.com/emilk/egui/").text("powered by egui"),
        //         );
        //     });
        // });

        egui::CentralPanel::default().show(ctx, |ui| {
            // The central panel the region left after adding TopPanel's and SidePanel's
            ui.horizontal(|ui| {
                for p in 0..MAX_PLAYERS {
                    let name = ri.player[p].clone().unwrap_or("--".to_string());
                    let name = name.as_str();
                    let player = &ri.update.players[p];
                    let num_cards = player.num_cards;
                    let score = player.score;
                    let player = Player::from_idx(p as u8);
                    let to_act: PlayerStatus =
                        if ri.update.state == GameState::ToAct(player.unwrap()) {
                            PlayerStatus::ToAct
                        } else {
                            PlayerStatus::Normal
                        };
                    ui.add(widgets::players::player(
                        name,
                        num_cards,
                        i16::from(score),
                        to_act,
                    ));
                }
            });

            ui.add(widgets::board::board(cards_board));
            ui.add(widgets::hand::cards(cards_selected, *cards_hand));

            let can_play =
                *cards_board == 0 || (*cards_board).count_ones() == (*cards_selected).count_ones();
            let mut want_play = false;
            let want_play = &mut want_play;

            ui.horizontal(|ui| {
                ui.add(widgets::buttons::button_pass(want_pass, false));
                ui.add(widgets::buttons::button_play(want_play, can_play));
            });

            // if can_play && *want_play {
            //     *cards_board = *cards_selected;
            //     *cards_hand ^= *cards_selected;
            //     *cards_selected = 0;
            // }

            egui::warn_if_debug_build(ui);
        });

        if false {
            egui::Window::new("Window").show(ctx, |ui| {
                ui.label("Windows can be moved by dragging them.");
                ui.label("They are automatically sized based on contents.");
                ui.label("You can turn on resizing and scrolling if you like.");
                ui.label("You would normally chose either panels OR windows.");
            });
        }
    }
}
