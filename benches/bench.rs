#![feature(test)]
extern crate test;

use big2::big2rules::cards::CardNum;
use big2::big2rules::cards::Cards;
use test::Bencher;

use big2::big2rules;
use big2::network;
mod benchfactor;

use std::convert::TryFrom;

#[bench]
fn bench_inlinelist8_convert(b: &mut Bencher) {
    let cards = [0u64, 0x1000, 0xF100_0000_0000_0000];
    b.iter(|| {
        for &hands in &cards {
            let il8 = network::legacy::muon::InlineList8::try_from(hands).unwrap();
            let cards = TryInto::<Cards>::try_into(il8);
            assert_eq!(cards, Ok(Cards(hands)));
        }
    });
}

#[bench]
fn bench_create_game_srv_obj(b: &mut Bencher) {
    b.iter(|| {
        let _gs = big2rules::SrvGameState::new(8);
    });
}

#[bench]
fn bench_game_srv_obj_deal_fix_cards(b: &mut Bencher) {
    let mut gs = big2rules::SrvGameState::new(8);
    let card_state = [
        Cards(0x0d00_8540_0417_4000),
        Cards(0x0002_2000_0000_8000 | 0x201_0ab6_0100_0000),
        Cards(0xf05c_1001_2a00_0000),
        Cards(0x00a0_4008_0000_1000 | 0x000_0000_d0e8_2000),
    ];
    b.iter(|| gs.deal(Some(&card_state)));
}

#[bench]
fn bench_game_srv_obj_deal_random_cargs(b: &mut Bencher) {
    let mut gs = big2rules::SrvGameState::new(8);
    b.iter(|| gs.deal(None));
}

#[bench]
fn bench_game_srv_obj_deal_full_play_8_rounds(b: &mut Bencher) {
    b.iter(|| {
        let mut gs = big2rules::SrvGameState::new(8);
        let mut card_state =
            benchfactor::gameserver_vectors::TEST_VECTOR_CARDS_GAME1.chunks_exact(4);

        let c: Option<[Cards; 4]> = card_state.next().and_then(|e| {
            if e.len() == 4 {
                Some([Cards(e[0]), Cards(e[1]), Cards(e[2]), Cards(e[3])])
            } else {
                None
            }
        });
        assert!(c.is_some());

        gs.deal(c.as_ref());

        assert_eq!(gs.turn, 3);
        assert_eq!(gs.round, 1);
        assert_eq!(gs.rounds, 8);

        for &play in benchfactor::gameserver_vectors::TEST_VECTOR_TRAIL_GAME1.iter() {
            let action = play as u16 & 0xF00;
            // Shift are needed to preseve the signbit!
            let player = (play as i32) << 29 >> 29;
            let toact = (play as i32) << 25 >> 29;

            let hand = Cards(play & 0xFFFF_FFFF_FFFF_F000);

            match action {
                0x800 => {
                    if play == 0x111_1800 {
                        let c: Option<[Cards; 4]> = card_state.next().and_then(|e| {
                            if e.len() == 4 {
                                Some([Cards(e[0]), Cards(e[1]), Cards(e[2]), Cards(e[3])])
                            } else {
                                None
                            }
                        });
                        assert!(c.is_some());

                        gs.deal(c.as_ref());
                    }
                }
                0x000 => {
                    assert!(gs.play(player, hand).is_ok());
                    let c = gs.cards[player as usize];
                    assert_eq!(c & hand, 0);
                    assert_eq!(gs.turn, toact);
                }
                0x100 => {
                    assert!(gs.pass(player).is_ok());
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

#[bench]
fn bench_score_hand(b: &mut Bencher) {
    b.iter(|| {
        for &hand in benchfactor::gameserver_vectors::TEST_VECTOR_TRAIL_GAME1 {
            let score = big2rules::rules::score_hand(Cards(hand & 0xFFFF_FFFF_FFFF_F000));
            // score is never 3, but to be sure that score is tested and not optimized out.
            assert_ne!(
                score,
                Some(big2rules::cards::ScoreKind::StraightFlush(CardNum::LOWCARD))
            );
        }
    });
}
