use super::*;
use crate::{bottle, bottle_content};
use heapless::Vec;

#[test]
fn test_bottle_creation() {
    let bottle1: KnownBottle<4> = KnownBottle::try_new(4).unwrap();

    assert_eq!(bottle1.capacity(), 4);
    assert_eq!(*bottle1.get_content(), Vec::<ColoredWaterUnit, 4>::new());

    let bottle2_base_content = [ColoredWaterUnit::Aqua, ColoredWaterUnit::Blue];
    let bottle2: KnownBottle<4> = KnownBottle::try_with_content(&bottle2_base_content).unwrap();

    assert_eq!(bottle2.capacity(), 2);
    assert_eq!(bottle2.get_content(), &bottle2_base_content);
}

#[test]
fn test_bottle_resize() {
    let bottle_base_content = bottle_content!(Aqua, Blue, Brown, Blue);
    let base_bottle: KnownBottle<8> = KnownBottle::try_with_content(&bottle_base_content).unwrap();

    assert_eq!(base_bottle.capacity(), 4);
    assert_eq!(base_bottle.get_content(), &bottle_base_content);

    let smaller_bottle: KnownBottle<4> = base_bottle.try_get_resized(3).unwrap();

    assert_eq!(smaller_bottle.capacity(), 3);
    assert_eq!(smaller_bottle.get_content(), &bottle_base_content[..3]);

    let larger_bottle: KnownBottle<4> = smaller_bottle.try_get_resized(5).unwrap();
    assert_eq!(larger_bottle.capacity(), 5);
    assert_eq!(larger_bottle.get_content(), &bottle_base_content[..3]);

    let mut in_place_resized_bottle = base_bottle.clone();

    let resize_result = in_place_resized_bottle.resize_in_place(3);
    assert!(resize_result.is_ok());
    assert_eq!(in_place_resized_bottle.capacity(), 3);
    assert_eq!(
        in_place_resized_bottle.get_content(),
        &bottle_base_content[..3]
    );

    // ensure we still succeed up to MAX_CAP of 8
    let resize_result = in_place_resized_bottle.resize_in_place(8);
    assert!(resize_result.is_ok());
    assert_eq!(in_place_resized_bottle.capacity(), 8);
    assert_eq!(
        in_place_resized_bottle.get_content(),
        &bottle_base_content[..3]
    );

    // ensure we fail beyond MAX_CAP of 8
    let resize_result = in_place_resized_bottle.resize_in_place(9);
    assert!(resize_result.is_err());
    assert_eq!(in_place_resized_bottle.capacity(), 8);
    assert_eq!(
        in_place_resized_bottle.get_content(),
        &bottle_base_content[..3]
    );

    let resize_result = in_place_resized_bottle.resize_in_place(5);
    assert!(resize_result.is_ok());
    assert_eq!(in_place_resized_bottle.capacity(), 5);
    assert_eq!(
        in_place_resized_bottle.get_content(),
        &bottle_base_content[..3]
    );

    let taken_resized_bottle1 = base_bottle.clone();

    let taken_resized_bottle2 = taken_resized_bottle1.try_take_as_resized(3).unwrap();
    assert_eq!(taken_resized_bottle2.capacity(), 3);
    assert_eq!(
        taken_resized_bottle2.get_content(),
        &bottle_base_content[..3]
    );

    let taken_resized_bottle3 = taken_resized_bottle2.try_take_as_resized(5).unwrap();
    assert_eq!(taken_resized_bottle3.capacity(), 5);
    assert_eq!(
        taken_resized_bottle3.get_content(),
        &bottle_base_content[..3]
    );
}

#[test]
fn test_bottle_pour_in() {
    let mut bottle = bottle!([Red], 4, 4);

    assert_eq!(*bottle.get_content(), bottle_content!(Red));

    // Pour one unit of Red, should be fully successful
    let content = bottle_content!(Red).try_into().unwrap();
    let test_pour_result = bottle.test_pour_in(content);
    let try_pour_result = bottle.try_pour_in(content);
    assert_eq!(
        try_pour_result,
        Ok(ColoredWaterRun {
            color: ColoredWaterUnit::Red,
            size: 0
        })
    );
    assert_eq!(test_pour_result, try_pour_result);
    assert_eq!(*bottle.get_content(), bottle_content!(Red, Red));

    // Pour in 1 unit of Aqua, should be unsuccessful due to mismatched colors
    let content = bottle_content!(Aqua).try_into().unwrap();
    let test_pour_result = bottle.test_pour_in(content);
    let try_pour_result = bottle.try_pour_in(content);
    assert_eq!(try_pour_result, Err(PourInError::MismatchedColors));
    assert_eq!(test_pour_result, try_pour_result);
    assert_eq!(*bottle.get_content(), bottle_content!(Red, Red));

    // Pour in 4 units of Red, should be partially successful (2 poured in, 2 left over)
    let content = bottle_content!(Red, Red, Red, Red).try_into().unwrap();
    let test_pour_result = bottle.test_pour_in(content);
    let try_pour_result = bottle.try_pour_in(content);
    assert_eq!(
        try_pour_result,
        Ok(ColoredWaterRun {
            color: ColoredWaterUnit::Red,
            size: 2
        })
    );
    assert_eq!(test_pour_result, try_pour_result);
    assert_eq!(*bottle.get_content(), bottle_content!(Red, Red, Red, Red));

    // Pour in 1 unit of Red, should be unsuccessful due to bottle already being full
    let content = bottle_content!(Red).try_into().unwrap();
    let test_pour_result = bottle.test_pour_in(content);
    let try_pour_result = bottle.try_pour_in(content);
    assert_eq!(try_pour_result, Err(PourInError::AlreadyFull));
    assert_eq!(test_pour_result, try_pour_result);
    assert_eq!(*bottle.get_content(), bottle_content!(Red, Red, Red, Red));
}

#[test]
fn test_bottle_pour_out() {
    let mut source_bottle = bottle!([Green, Blue, Red, Red, Red], 5, 8);

    let mut dest_bottle: KnownBottle<8> = KnownBottle::try_new(2).unwrap();

    // Pour the top color run (2 reds) into destination, should be successful
    let test_pour_result = source_bottle.test_pour_out(&dest_bottle);
    let try_pour_result = source_bottle.try_pour_out(&mut dest_bottle);
    assert_eq!(try_pour_result, Ok(()));
    assert_eq!(test_pour_result, try_pour_result);
    assert_eq!(
        *source_bottle.get_content(),
        bottle_content!(Green, Blue, Red)
    );
    assert_eq!(*dest_bottle.get_content(), bottle_content!(Red, Red));

    let mut source_bottle = bottle!([Green, Blue], 2, 4);
    let mut dest_bottle = bottle!([Red], 4, 4);

    // Attempt to pour the top color run (1 blue) into destination, should fail due to mismatched colors
    let test_pour_result = source_bottle.test_pour_out(&dest_bottle);
    let try_pour_result = source_bottle.try_pour_out(&mut dest_bottle);
    assert_eq!(
        try_pour_result,
        Err(PourOutError::DestinationError(
            PourInError::MismatchedColors
        ))
    );
    assert_eq!(test_pour_result, try_pour_result);
    assert_eq!(*source_bottle.get_content(), bottle_content!(Green, Blue));
    assert_eq!(*dest_bottle.get_content(), bottle_content!(Red));

    let mut source_bottle = bottle!([Red, Blue, Green], 3, 4);
    let mut dest_bottle = bottle!([Green], 1, 4);

    // Attempt to pour the top color run (1 green) into destination, should fail due to no space
    let test_pour_result = source_bottle.test_pour_out(&dest_bottle);
    let try_pour_result = source_bottle.try_pour_out(&mut dest_bottle);
    assert_eq!(
        try_pour_result,
        Err(PourOutError::DestinationError(PourInError::AlreadyFull))
    );
    assert_eq!(test_pour_result, try_pour_result);
    assert_eq!(
        *source_bottle.get_content(),
        bottle_content!(Red, Blue, Green)
    );
    assert_eq!(dest_bottle.get_content(), &[ColoredWaterUnit::Green]);

    let mut source_bottle: KnownBottle<4> = KnownBottle::try_new(4).unwrap();
    let mut dest_bottle: KnownBottle<4> = KnownBottle::try_new(4).unwrap();

    //Attempt to pour the top color run (nothing) into destination, should fail due to source bottle being empty
    let test_pour_result = source_bottle.test_pour_out(&dest_bottle);
    let try_pour_result = source_bottle.try_pour_out(&mut dest_bottle);
    assert_eq!(try_pour_result, Err(PourOutError::Empty));
    assert_eq!(test_pour_result, try_pour_result);
}
