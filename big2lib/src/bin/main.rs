#![forbid(unsafe_code)]
#![cfg_attr(not(debug_assertions), deny(warnings))] // Forbid warnings in release builds
#![warn(clippy::all, rust_2018_idioms)]

use big2lib::{
    big2rules,
    messages::{GameState, RoomInfo},
    players::Player,
    TemplateApp,
};

// When compiling natively:
#[cfg(not(target_arch = "wasm32"))]
fn main() {
    let mut big2game = big2rules::SrvGameState::new(8);
    big2game.deal(None);

    let mut gs = RoomInfo::new("dsafasdfhasidfh".to_string(), Some(8));

    gs.user_add("Super Bot 1".to_string());
    gs.user_add("Real User 12345678".to_string());
    let rene_p1 = gs.user_add("René".to_string()).unwrap();
    gs.user_add("1235627888".to_string());

    gs.update.hand = Some(big2game.cards[3]);
    gs.update.board = 0x1_1000;
    gs.update.state = GameState::ToAct(Player::from_idx(big2game.turn as u8).unwrap());

    let app = TemplateApp {
        ri: gs,
        cards_selected: 0,
        want_pass: false,
    };
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(Box::new(app), native_options);
}
