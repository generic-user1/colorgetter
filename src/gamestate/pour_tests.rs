use std::collections::HashSet;

use super::*;
use crate::{bottle, bottle::Bottle, colored_water::ColoredWaterUnit};

#[test]
fn test_valid_pour_iter() {
    let first_gamestate: GameState<5, 4> = GameState {
        bottles: [
            bottle!([Red, Red, Red], 4),
            bottle!([Orange, Green, Red], 4),
            bottle!([Yellow, Maroon], 3),
            bottle!([Brown, Lime, Aqua, Aqua], 4),
            bottle!([Maroon, Maroon], 4)
        ]
    };

    // We compare using HashSet instead of vec because we only care that the specific Pours generated
    // from iter_pours match, not their order.
    let all_pours: HashSet<_> = first_gamestate.iter_pours().collect();
    assert_eq!(
        all_pours,
        HashSet::from([
            ValidPour::try_new(&first_gamestate, 0, 1).unwrap(),
            ValidPour::try_new(&first_gamestate, 1, 0).unwrap(),
            ValidPour::try_new(&first_gamestate, 2, 4).unwrap(),
            ValidPour::try_new(&first_gamestate, 4, 2).unwrap()
        ])
    );

    let second_gamestate = ValidPour::try_new(&first_gamestate, 0, 1).unwrap().apply();
    let all_pours: HashSet<_> = second_gamestate.iter_pours().collect();
    assert_eq!(
        all_pours,
        HashSet::from([
            ValidPour::try_new(&second_gamestate, 1, 0).unwrap(),
            ValidPour::try_new(&second_gamestate, 2, 4).unwrap(),
            ValidPour::try_new(&second_gamestate, 4, 2).unwrap(),
        ])
    );
}
