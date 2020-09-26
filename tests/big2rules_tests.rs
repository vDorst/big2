//use crate::big2rules;
mod big2rules_srv_test_vectors;

#[cfg(test)]
mod tests_big2rules {
    use super::big2rules_srv_test_vectors;
    use big2::big2rules;

    #[test]
    fn game_srv_object_test() {
        let mut gs = big2rules::SrvGameState::new(8);
        let mut cp: usize = 0;

        gs.deal(Some(
            &big2rules_srv_test_vectors::gameserver_vectors::TEST_VECTOR_CARDS_GAME1[0..4],
        ));

        assert_eq!(gs.turn, 3);
        assert_eq!(gs.round, 1);
        assert_eq!(gs.rounds, 8);

        for play in big2rules_srv_test_vectors::gameserver_vectors::TEST_VECTOR_TRAIL_GAME1.iter() {
            let action = *play as i32 & 0xF00;
            let player = ((*play as i32 & 0x7) << 29) >> 29;
            let toact = ((*play as i32 & 0x70) << 25) >> 29;

            let mut error: Result<(), big2rules::SrvGameError> = Ok(());
            let hand: u64 = play & 0xFFFF_FFFF_FFFF_F000;

            match action {
                0x800 => {
                    println!("UPDATE {:16x}", play);
                    if *play == 0x111_1800 {
                        cp += 4;
                        gs.deal(Some(&big2rules_srv_test_vectors::gameserver_vectors::TEST_VECTOR_CARDS_GAME1[cp..cp + 4]));
                        println!("++ Start new game, round {}/{}", gs.round, gs.rounds);
                    }
                }
                0x000 => {
                    print!(
                        "++ PLAY: player {} hand {:16x} card {:16x} - ",
                        player, hand, gs.cards[player as usize]
                    );
                    error = gs.play(player, hand);
                    if error.is_ok() {
                        let c = gs.cards[player as usize];
                        println!("card {:16x} c&h {:16x} p {:16x}", c, c & hand, play);
                        assert!(c & hand == 0);
                        assert_eq!(gs.turn, toact);
                    } else {
                        println!("error");
                    }
                }
                0x100 => {
                    println!("++ PASS: player {}", player);
                    error = gs.pass(player);
                }
                0x400 => {
                    println!("++ DEAL: {:16x}", play);
                    // Match hand
                    assert_eq!(hand, gs.cards[player as usize]);
                    // turn and next user have to match
                    assert_eq!(toact, gs.turn);
                }
                _ => println!("Unknown action {}", action),
            }

            println!(
                " AFTER: BS{:4x} HP{:4x} T{}",
                gs.board_score, gs.has_passed, gs.turn
            );

            if let Err(e) = error {
                println!("Error with hand {:?}", e);
                match e {
                    big2rules::SrvGameError::PlayerPlayedIllegalCard(hand) => println!(
                        "PLAY: hand {:16x} card {:16x}",
                        hand, gs.cards[player as usize]
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
