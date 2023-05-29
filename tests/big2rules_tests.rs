//use crate::big2rules;
mod big2rules_srv_test_vectors;

#[cfg(test)]
mod tests_big2rules {
    use crate::big2rules_srv_test_vectors::gameserver_vectors;
    use big2::big2rules::{self, cards::Cards};

    #[test]
    fn game_srv_object_test() {
        let mut gs = big2rules::SrvGameState::new(8);
        let mut cp: usize = 0;

        gs.deal(Some(
            &gameserver_vectors::TEST_VECTOR_CARDS_GAME1[0..4]
                .iter()
                .map(|x| Cards::try_from(*x).unwrap())
                .collect::<Vec<Cards>>()
                .try_into()
                .unwrap(),
        ));

        assert_eq!(gs.turn, 3);
        assert_eq!(gs.round, 1);
        assert_eq!(gs.rounds, 8);

        for play in gameserver_vectors::TEST_VECTOR_TRAIL_GAME1.iter() {
            let action = *play as i32 & 0xF00;
            let player = ((*play as i32 & 0x7) << 29) >> 29;
            let toact = ((*play as i32 & 0x70) << 25) >> 29;

            let mut error: Result<(), big2rules::SrvGameError> = Ok(());
            let hand: u64 = play & 0xFFFF_FFFF_FFFF_F000;

            match action {
                0x800 => {
                    println!("UPDATE {play:16x}");
                    if *play == 0x111_1800 {
                        cp += 4;
                        gs.deal(Some(
                            &gameserver_vectors::TEST_VECTOR_CARDS_GAME1[cp..cp + 4]
                                .iter()
                                .map(|x| Cards::try_from(*x).unwrap())
                                .collect::<Vec<Cards>>()
                                .try_into()
                                .unwrap(),
                        ));
                        println!("++ Start new game, round {}/{}", gs.round, gs.rounds);
                    }
                }
                0x000 => {
                    print!(
                        "++ PLAY: player {} hand {:16x} card {:16x} - ",
                        player, hand, gs.cards[player as usize].0
                    );
                    error = gs.play(player, Cards::from(hand));
                    if error.is_ok() {
                        let c = gs.cards[player as usize];
                        println!("card {:16x} c&h {:16x} p {:16x}", c, hand & c, play);
                        assert_eq!(hand & c, 0);
                        assert_eq!(gs.turn, toact);
                    } else {
                        println!("error");
                    }
                }
                0x100 => {
                    println!("++ PASS: player {player}");
                    error = gs.pass(player);
                }
                0x400 => {
                    println!("++ DEAL: {play:16x}");
                    // Match hand
                    assert_eq!(Cards::from(hand), gs.cards[player as usize]);
                    // turn and next user have to match
                    assert_eq!(toact, gs.turn);
                }
                _ => println!("Unknown action {action}"),
            }

            println!(
                " AFTER: BS{:?} HP{:4x} T{}",
                gs.board_score, gs.has_passed, gs.turn
            );

            if let Err(e) = error {
                println!("Error with hand {e:?}");
                match e {
                    big2rules::SrvGameError::PlayerPlayedIllegalCard(hand) => println!(
                        "PLAY: hand {:16x} card {:16x}",
                        hand, gs.cards[player as usize].0
                    ),
                    big2rules::SrvGameError::NotPlayersTurn => {
                        println!("Turn {} player {}", gs.turn, player)
                    }
                    _ => print!(""),
                }
            }
        }
    }
}
