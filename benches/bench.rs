#![feature(test)]

extern crate test;
use test::Bencher;

use big2::big2rules;
mod benchfactor;

#[bench]
fn bench_create_game_srv_obj(b: &mut Bencher) {
    b.iter(|| {
        let mut gs = big2rules::SrvGameState::new(8);
        drop(gs);
    });
}

#[bench]
fn bench_game_srv_obj_deal_fix_cards(b: &mut Bencher) {
    let mut gs = big2rules::SrvGameState::new(8);
    b.iter(|| {
        gs.deal(Some(&[
            0x0d00854004174000,
            0x2200000008000 | 0x201_0ab6_0100_0000,
            0xf05c10012a000000,
            0xa0400800001000 | 0x000_0000_d0e8_2000,
        ]))
    });
}

#[bench]
fn bench_game_srv_obj_deal_random_cargs(b: &mut Bencher) {
    let mut gs = big2rules::SrvGameState::new(8);
    b.iter(|| gs.deal(None));
}

#[bench]
fn bench_game_srv_obj_deal_full_play_8_rounds(b: &mut Bencher) {
    let mut i = 0;

    b.iter(|| {
        let mut gs = big2rules::SrvGameState::new(8);
        let mut cp: usize = 0;

        gs.deal(Some(
            &benchfactor::gameserver_vectors::TEST_VECTOR_CARDS_GAME1[0..4],
        ));

        assert_eq!(gs.turn, 3);
        assert_eq!(gs.round, 1);
        assert_eq!(gs.rounds, 8);

        for play in benchfactor::gameserver_vectors::TEST_VECTOR_TRAIL_GAME1.iter() {
            let action = *play as i32 & 0xF00;
            let player = ((*play as i32 & 0x7) << 29) >> 29;
            let toact = ((*play as i32 & 0x70) << 25) >> 29;

            let mut error: Result<(), big2rules::SrvGameError> = Ok(());
            let hand: u64 = play & 0xFFFF_FFFF_FFFF_F000;

            match action {
                0x800 => {
                    if *play == 0x111_1800 {
                        cp += 4;
                        gs.deal(Some(
                            &benchfactor::gameserver_vectors::TEST_VECTOR_CARDS_GAME1[cp..cp + 4],
                        ));
                    }
                }
                0x000 => {
                    error = gs.play(player, hand);
                    if error.is_ok() {
                        let c = gs.cards[player as usize];
                        assert!(c & hand == 0);
                        assert_eq!(gs.turn, toact);
                    }
                }
                0x100 => {
                    error = gs.pass(player);
                }
                0x400 => {
                    // Match hand
                    assert_eq!(hand, gs.cards[player as usize]);
                    // turn and next user have to match
                    assert_eq!(toact, gs.turn);
                }
                _ => (),
            }
        }
    });
}
