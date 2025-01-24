use super::*;

#[test]
fn test_bottle_creation() {
    let bottle1 = Bottle::new(4);

    assert_eq!(bottle1.get_capacity(), 4);
    assert_eq!(bottle1.get_content(), Vec::new());

    let bottle2_base_content = [ColoredWaterUnit::Aqua, ColoredWaterUnit::Blue];
    let bottle2 = Bottle::with_content(&bottle2_base_content);

    assert_eq!(bottle2.get_capacity(), 2);
    assert_eq!(bottle2.get_content(), &bottle2_base_content);
}

#[test]
fn test_bottle_resize() {
    let bottle_base_content = [
        ColoredWaterUnit::Aqua,
        ColoredWaterUnit::Blue,
        ColoredWaterUnit::Brown,
        ColoredWaterUnit::Blue
    ];
    let base_bottle = Bottle::with_content(&bottle_base_content);

    assert_eq!(base_bottle.get_capacity(), 4);
    assert_eq!(base_bottle.get_content(), &bottle_base_content);

    let smaller_bottle = base_bottle.get_resized(3);

    assert_eq!(smaller_bottle.get_capacity(), 3);
    assert_eq!(smaller_bottle.get_content(), &bottle_base_content[..3]);

    let larger_bottle = smaller_bottle.get_resized(5);
    assert_eq!(larger_bottle.get_capacity(), 5);
    assert_eq!(larger_bottle.get_content(), &bottle_base_content[..3]);

    let mut in_place_resized_bottle = base_bottle.clone();

    in_place_resized_bottle.resize_in_place(3);
    assert_eq!(in_place_resized_bottle.get_capacity(), 3);
    assert_eq!(
        in_place_resized_bottle.get_content(),
        &bottle_base_content[..3]
    );

    in_place_resized_bottle.resize_in_place(5);
    assert_eq!(in_place_resized_bottle.get_capacity(), 5);
    assert_eq!(
        in_place_resized_bottle.get_content(),
        &bottle_base_content[..3]
    );
}

#[test]
fn test_bottle_pour_in() {
    let mut bottle = Bottle::new(4);
    bottle.content = vec![ColoredWaterUnit::Red];

    assert_eq!(bottle.get_content(), &[ColoredWaterUnit::Red]);

    // Pour one unit of Red, should be fully successful
    let try_pour_result = bottle.try_pour_in(ColoredWaterRun {
        color: ColoredWaterUnit::Red,
        size: 1
    });
    assert_eq!(
        try_pour_result,
        Ok(ColoredWaterRun {
            color: ColoredWaterUnit::Red,
            size: 0
        })
    );
    assert_eq!(
        bottle.get_content(),
        &[ColoredWaterUnit::Red, ColoredWaterUnit::Red]
    );

    // Pour in 1 unit of Aqua, should be unsuccessful due to mismatched colors
    let try_pour_result = bottle.try_pour_in(ColoredWaterRun {
        color: ColoredWaterUnit::Aqua,
        size: 1
    });
    assert_eq!(try_pour_result, Err(PourInError::MismatchedColors));
    assert_eq!(
        bottle.get_content(),
        &[ColoredWaterUnit::Red, ColoredWaterUnit::Red]
    );

    // Pour in 4 units of Red, should be partially successful (2 poured in, 2 left over)
    let try_pour_result = bottle.try_pour_in(ColoredWaterRun {
        color: ColoredWaterUnit::Red,
        size: 4
    });

    assert_eq!(
        try_pour_result,
        Ok(ColoredWaterRun {
            color: ColoredWaterUnit::Red,
            size: 2
        })
    );

    assert_eq!(
        bottle.get_content(),
        &[
            ColoredWaterUnit::Red,
            ColoredWaterUnit::Red,
            ColoredWaterUnit::Red,
            ColoredWaterUnit::Red
        ]
    );

    // Pour in 1 unit of Red, should be unsuccessful due to bottle already being full
    let try_pour_result = bottle.try_pour_in(ColoredWaterRun {
        color: ColoredWaterUnit::Red,
        size: 1
    });

    assert_eq!(try_pour_result, Err(PourInError::AlreadyFull));

    assert_eq!(
        bottle.get_content(),
        &[
            ColoredWaterUnit::Red,
            ColoredWaterUnit::Red,
            ColoredWaterUnit::Red,
            ColoredWaterUnit::Red
        ]
    );
}

#[test]
fn test_bottle_pour_out() {
    let mut source_bottle = Bottle::with_content(&[
        ColoredWaterUnit::Green,
        ColoredWaterUnit::Blue,
        ColoredWaterUnit::Red,
        ColoredWaterUnit::Red,
        ColoredWaterUnit::Red
    ]);

    let mut dest_bottle = Bottle::new(2);

    // Pour the top color run (2 reds) into destination, should be successful
    let try_pour_result = source_bottle.try_pour_out(&mut dest_bottle);
    assert_eq!(try_pour_result, Ok(()));
    assert_eq!(
        source_bottle.get_content(),
        &[
            ColoredWaterUnit::Green,
            ColoredWaterUnit::Blue,
            ColoredWaterUnit::Red
        ]
    );
    assert_eq!(
        dest_bottle.get_content(),
        &[ColoredWaterUnit::Red, ColoredWaterUnit::Red]
    );

    let mut source_bottle =
        Bottle::with_content(&[ColoredWaterUnit::Green, ColoredWaterUnit::Blue]);
    let mut dest_bottle = Bottle::with_content(&[ColoredWaterUnit::Red]);
    dest_bottle.resize_in_place(4);

    // Attempt to pour the top color run (1 blue) into destination, should fail due to mismatched colors
    let try_pour_result = source_bottle.try_pour_out(&mut dest_bottle);
    assert_eq!(
        try_pour_result,
        Err(PourOutError::DestinationError(
            PourInError::MismatchedColors
        ))
    );
    assert_eq!(
        source_bottle.get_content(),
        &[ColoredWaterUnit::Green, ColoredWaterUnit::Blue]
    );
    assert_eq!(dest_bottle.get_content(), &[ColoredWaterUnit::Red]);

    let mut source_bottle = Bottle::with_content(&[
        ColoredWaterUnit::Red,
        ColoredWaterUnit::Blue,
        ColoredWaterUnit::Green
    ]);
    let mut dest_bottle = Bottle::with_content(&[ColoredWaterUnit::Green]);

    // Attempt to pour the top color run (1 green) into destination, should fail due to no space
    let try_pour_result = source_bottle.try_pour_out(&mut dest_bottle);
    assert_eq!(
        try_pour_result,
        Err(PourOutError::DestinationError(PourInError::AlreadyFull))
    );
    assert_eq!(
        source_bottle.get_content(),
        &[
            ColoredWaterUnit::Red,
            ColoredWaterUnit::Blue,
            ColoredWaterUnit::Green
        ]
    );
    assert_eq!(dest_bottle.get_content(), &[ColoredWaterUnit::Green]);

    let mut source_bottle = Bottle::new(4);
    let mut dest_bottle = Bottle::new(4);

    //Attempt to pour the top color run (nothing) into destination, should fail due to source bottle being empty
    let try_pour_result = source_bottle.try_pour_out(&mut dest_bottle);
    assert_eq!(try_pour_result, Err(PourOutError::Empty));
}
