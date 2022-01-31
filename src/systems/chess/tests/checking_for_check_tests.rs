use crate::model::{AllValidMoves, CastlingData, Move, Piece, PieceColour, PieceKind, SpecialMoveData, Square};
use crate::systems::chess::{calculate_all_moves, GameState, PlayerTurn};
use bevy::prelude::*;

fn setup() -> (World, SystemStage) {
    let mut world = World::new();

    world.insert_resource(AllValidMoves::default());
    world.insert_resource(PlayerTurn(PieceColour::Black));
    world.insert_resource(State::new(GameState::NothingSelected));
    world.insert_resource(SpecialMoveData {
        last_pawn_double_step: None,
        black_castling_data: CastlingData {
            king_moved: true,
            ..Default::default()
        },
        white_castling_data: CastlingData {
            king_moved: true,
            ..Default::default()
        },
    });

    let mut update_stage = SystemStage::parallel();
    update_stage.add_system_set(State::<GameState>::get_driver());
    update_stage.add_system(calculate_all_moves.system());

    (world, update_stage)
}

#[test]
fn should_not_allow_a_king_to_remain_in_check_if_it_can_move() {
    let (mut world, mut update_stage) = setup();

    let king_id = world
        .spawn()
        .insert(Piece {
            kind: PieceKind::King,
            colour: PieceColour::Black,
            square: (7, 4).into(),
        })
        .id();

    // knight has king in check
    world.spawn().insert(Piece {
        kind: PieceKind::Knight,
        colour: PieceColour::White,
        square: (5, 3).into(),
    });

    // add other pieces around King to restrict (but not totally block) movement that can't take the knight,
    // to verify that even though they can move, they aren't selectable while the king is in check
    let rook_id = world
        .spawn()
        .insert(Piece {
            kind: PieceKind::Rook,
            colour: PieceColour::Black,
            square: (7, 5).into(),
        })
        .id();

    let knight_id = world
        .spawn()
        .insert(Piece {
            kind: PieceKind::Knight,
            colour: PieceColour::Black,
            square: (6, 3).into(),
        })
        .id();

    let queen_id = world
        .spawn()
        .insert(Piece {
            kind: PieceKind::Queen,
            colour: PieceColour::Black,
            square: (7, 3).into(),
        })
        .id();

    update_stage.run(&mut world);
    let valid_moves = world.get_resource::<AllValidMoves>().unwrap();

    assert_eq!(
        valid_moves.get(king_id),
        &vec![Move::standard((6, 4).into())]
    );
    assert_eq!(valid_moves.get(rook_id), &vec![]);
    assert_eq!(valid_moves.get(knight_id), &vec![]);
    assert_eq!(valid_moves.get(queen_id), &vec![]);
}

#[test]
fn should_detect_checkmate_when_the_king_cannot_move_and_the_opposing_piece_cannot_be_taken_or_blocked(
) {
    let (mut world, mut update_stage) = setup();

    world.spawn().insert(Piece {
        kind: PieceKind::King,
        colour: PieceColour::Black,
        square: (7, 4).into(),
    });

    // knight has king in check
    world.spawn().insert(Piece {
        kind: PieceKind::Knight,
        colour: PieceColour::White,
        square: (5, 3).into(),
    });

    // pieces blocking the king but tragically unable to take the knight
    let mut spawn_pawn = |x: u8, y: u8| {
        world.spawn().insert(Piece {
            kind: PieceKind::Pawn,
            colour: PieceColour::Black,
            square: Square::new(x, y),
        });
    };

    spawn_pawn(7, 3);
    spawn_pawn(6, 3);
    spawn_pawn(7, 5);
    spawn_pawn(6, 5);

    // can't place pawn here or it would be able to take the knight
    world.spawn().insert(Piece {
        kind: PieceKind::Rook,
        colour: PieceColour::Black,
        square: (6, 4).into(),
    });

    update_stage.run(&mut world);

    let game_state = world.get_resource::<State<GameState>>().unwrap();
    assert_eq!(
        game_state.current(),
        &GameState::Checkmate(PieceColour::Black)
    );
}

#[test]
fn should_not_detect_checkmate_if_the_king_cannot_move_but_the_opposing_piece_can_be_taken() {
    let (mut world, mut update_stage) = setup();
    let mut ids = vec![];

    ids.push(
        world
            .spawn()
            .insert(Piece {
                kind: PieceKind::King,
                colour: PieceColour::Black,
                square: (7, 4).into(),
            })
            .id(),
    );

    // knight has king in check
    world.spawn().insert(Piece {
        kind: PieceKind::Knight,
        colour: PieceColour::White,
        square: (5, 3).into(),
    });

    let mut spawn_pawn = |x: u8, y: u8| {
        world
            .spawn()
            .insert(Piece {
                kind: PieceKind::Pawn,
                colour: PieceColour::Black,
                square: Square::new(x, y),
            })
            .id()
    };

    ids.push(spawn_pawn(7, 3));
    ids.push(spawn_pawn(6, 3));
    ids.push(spawn_pawn(7, 5));
    ids.push(spawn_pawn(6, 5));

    let pawn_id = spawn_pawn(6, 4);

    update_stage.run(&mut world);

    let all_valid_moves = world.get_resource::<AllValidMoves>().unwrap();
    ids.into_iter()
        .for_each(|id| assert!(all_valid_moves.get(id).is_empty()));

    assert_eq!(
        all_valid_moves.get(pawn_id),
        &vec![Move::standard((5, 3).into())]
    );

    let game_state = world.get_resource::<State<GameState>>().unwrap();
    assert_eq!(game_state.current(), &GameState::NothingSelected);
}

#[test]
fn should_detect_checkmate_if_multiple_pieces_have_the_king_in_check() {
    let (mut world, mut update_stage) = setup();

    world.spawn().insert(Piece {
        kind: PieceKind::King,
        colour: PieceColour::Black,
        square: (7, 4).into(),
    });

    // knight has king in check
    world.spawn().insert(Piece {
        kind: PieceKind::Knight,
        colour: PieceColour::White,
        square: (5, 3).into(),
    });

    // also has king in check
    world.spawn().insert(Piece {
        kind: PieceKind::Knight,
        colour: PieceColour::White,
        square: (5, 5).into(),
    });

    let mut spawn_pawn = |x: u8, y: u8| {
        world.spawn().insert(Piece {
            kind: PieceKind::Pawn,
            colour: PieceColour::Black,
            square: Square::new(x, y),
        });
    };

    spawn_pawn(7, 3);
    spawn_pawn(6, 3);
    spawn_pawn(7, 5);
    spawn_pawn(6, 5);
    spawn_pawn(6, 4);

    update_stage.run(&mut world);

    let game_state = world.get_resource::<State<GameState>>().unwrap();
    assert_eq!(
        game_state.current(),
        &GameState::Checkmate(PieceColour::Black)
    );
}

#[test]
fn should_not_allow_the_king_to_move_into_check() {
    let (mut world, mut update_stage) = setup();

    let king_id = world
        .spawn()
        .insert(Piece {
            kind: PieceKind::King,
            colour: PieceColour::Black,
            square: (7, 4).into(),
        })
        .id();

    world.spawn().insert(Piece {
        kind: PieceKind::Rook,
        colour: PieceColour::White,
        square: (0, 3).into(),
    });

    world.spawn().insert(Piece {
        kind: PieceKind::Rook,
        colour: PieceColour::White,
        square: (0, 5).into(),
    });

    update_stage.run(&mut world);

    let all_valid_moves = world.get_resource::<AllValidMoves>().unwrap();
    assert_eq!(
        all_valid_moves.get(king_id),
        &vec![Move::standard((6, 4).into())]
    );
}

#[test]
fn should_not_allow_the_king_to_take_a_piece_if_it_would_leave_the_king_in_check() {
    let (mut world, mut update_stage) = setup();

    world.spawn().insert(Piece {
        kind: PieceKind::King,
        colour: PieceColour::Black,
        square: (3, 3).into(),
    });

    world.spawn().insert(Piece {
        kind: PieceKind::Pawn,
        colour: PieceColour::White,
        square: (2, 4).into(),
    });
    world.spawn().insert(Piece {
        kind: PieceKind::Rook,
        colour: PieceColour::White,
        square: (7, 4).into(),
    });
    world.spawn().insert(Piece {
        kind: PieceKind::Rook,
        colour: PieceColour::White,
        square: (0, 2).into(),
    });
    world.spawn().insert(Piece {
        kind: PieceKind::Rook,
        colour: PieceColour::White,
        square: (4, 0).into(),
    });
    world.spawn().insert(Piece {
        kind: PieceKind::Rook,
        colour: PieceColour::White,
        square: (2, 0).into(),
    });

    update_stage.run(&mut world);

    assert_eq!(
        world.get_resource::<State<GameState>>().unwrap().current(),
        &GameState::Checkmate(PieceColour::Black)
    );
}

#[test]
fn should_detect_checkmate_if_the_king_is_in_check_and_cannot_move_out_of_check() {
    let (mut world, mut update_stage) = setup();

    world.spawn().insert(Piece {
        kind: PieceKind::King,
        colour: PieceColour::Black,
        square: (7, 4).into(),
    });

    // bishop has king in check
    world.spawn().insert(Piece {
        kind: PieceKind::Bishop,
        colour: PieceColour::White,
        square: (5, 2).into(),
    });

    let mut spawn_pawn = |x: u8, y: u8| {
        world.spawn().insert(Piece {
            kind: PieceKind::Pawn,
            colour: PieceColour::Black,
            square: Square::new(x, y),
        });
    };

    spawn_pawn(7, 5);
    spawn_pawn(6, 5);
    spawn_pawn(6, 4);

    world.spawn().insert(Piece {
        kind: PieceKind::Bishop,
        colour: PieceColour::Black,
        square: (7, 3).into(),
    });

    update_stage.run(&mut world);

    let game_state = world.get_resource::<State<GameState>>().unwrap();
    assert_eq!(
        game_state.current(),
        &GameState::Checkmate(PieceColour::Black)
    );
}

#[test]
fn should_not_detect_checkmate_if_a_piece_can_be_moved_to_block_check() {
    let (mut world, mut update_stage) = setup();
    let mut ids = vec![];

    ids.push(
        world
            .spawn()
            .insert(Piece {
                kind: PieceKind::King,
                colour: PieceColour::Black,
                square: (7, 4).into(),
            })
            .id(),
    );

    // bishop has king in check
    world.spawn().insert(Piece {
        kind: PieceKind::Bishop,
        colour: PieceColour::White,
        square: (5, 2).into(),
    });

    let mut spawn_pawn = |x: u8, y: u8| {
        world
            .spawn()
            .insert(Piece {
                kind: PieceKind::Pawn,
                colour: PieceColour::Black,
                square: Square::new(x, y),
            })
            .id()
    };

    ids.push(spawn_pawn(7, 5));
    ids.push(spawn_pawn(6, 5));
    ids.push(spawn_pawn(6, 4));

    let blocking_pawn = spawn_pawn(7, 3);

    update_stage.run(&mut world);

    let all_valid_moves = world.get_resource::<AllValidMoves>().unwrap();
    ids.into_iter().for_each(|id| {
        assert!(
            all_valid_moves.get(id).is_empty(),
            "{:?}, {:?}",
            world.get_entity(id).unwrap().get::<Piece>(),
            all_valid_moves.get(id)
        )
    });

    assert_eq!(
        all_valid_moves.get(blocking_pawn),
        &vec![Move::standard((6, 3).into())]
    );

    let game_state = world.get_resource::<State<GameState>>().unwrap();
    assert_eq!(game_state.current(), &GameState::NothingSelected);
}

#[test]
fn should_not_allow_moving_a_piece_if_it_would_leave_the_king_in_check() {
    let (mut world, mut update_stage) = setup();

    world.spawn().insert(Piece {
        kind: PieceKind::King,
        colour: PieceColour::Black,
        square: (7, 4).into(),
    });

    // blocked by rook
    world.spawn().insert(Piece {
        kind: PieceKind::Bishop,
        colour: PieceColour::White,
        square: (5, 2).into(),
    });

    // currently blocking bishop from taking the king
    let rook_id = world
        .spawn()
        .insert(Piece {
            kind: PieceKind::Rook,
            colour: PieceColour::Black,
            square: (6, 3).into(),
        })
        .id();

    update_stage.run(&mut world);

    let all_valid_moves = world.get_resource::<AllValidMoves>().unwrap();
    assert!(all_valid_moves.get(rook_id).is_empty());

    let game_state = world.get_resource::<State<GameState>>().unwrap();
    assert_eq!(game_state.current(), &GameState::NothingSelected);
}

#[test]
fn should_allow_moving_a_piece_protecting_the_king_within_the_path_between_the_blocked_piece_and_the_king(
) {
    let (mut world, mut update_stage) = setup();

    world.spawn().insert(Piece {
        kind: PieceKind::King,
        colour: PieceColour::Black,
        square: (7, 4).into(),
    });

    // blocked by black bishop
    world.spawn().insert(Piece {
        kind: PieceKind::Bishop,
        colour: PieceColour::White,
        square: (3, 0).into(),
    });

    let bishop_id = world
        .spawn()
        .insert(Piece {
            kind: PieceKind::Bishop,
            colour: PieceColour::Black,
            square: (5, 2).into(),
        })
        .id();

    update_stage.run(&mut world);

    let all_valid_moves = world.get_resource::<AllValidMoves>().unwrap();
    assert_eq!(
        all_valid_moves.get(bishop_id),
        &vec![
            Move::standard((6, 3).into()),
            Move::standard((4, 1).into()),
            Move::standard((3, 0).into()),
        ]
    );
}

#[test]
fn should_not_be_able_to_move_a_piece_to_take_a_second_piece_with_the_king_in_check_if_it_is_blocking_a_third_piece(
) {
    let (mut world, mut update_stage) = setup();

    let king_id = world
        .spawn()
        .insert(Piece {
            kind: PieceKind::King,
            colour: PieceColour::Black,
            square: (7, 4).into(),
        })
        .id();

    // has the king in check
    world.spawn().insert(Piece {
        kind: PieceKind::Knight,
        colour: PieceColour::White,
        square: (5, 3).into(),
    });

    // could move to take the knight, but would expose the king to the rook
    let pawn_id = world
        .spawn()
        .insert(Piece {
            kind: PieceKind::Pawn,
            colour: PieceColour::Black,
            square: (6, 4).into(),
        })
        .id();

    // blocked by pawn
    world.spawn().insert(Piece {
        kind: PieceKind::Rook,
        colour: PieceColour::White,
        square: (5, 4).into(),
    });

    update_stage.run(&mut world);

    let all_valid_moves = world.get_resource::<AllValidMoves>().unwrap();
    assert!(all_valid_moves.get(pawn_id).is_empty());
    assert_eq!(
        all_valid_moves.get(king_id),
        &vec![
            Move::standard((6, 3).into()),
            Move::standard((7, 3).into()),
            Move::standard((7, 5).into())
        ]
    );
}

#[test]
fn should_detect_checkmate_when_multiple_pieces_have_the_king_in_check_even_when_they_can_both_be_taken(
) {
    let (mut world, mut update_stage) = setup();

    world.spawn().insert(Piece {
        kind: PieceKind::King,
        colour: PieceColour::Black,
        square: (7, 4).into(),
    });

    let mut spawn_pawn = |x: u8, y: u8| {
        world.spawn().insert(Piece {
            kind: PieceKind::Pawn,
            colour: PieceColour::Black,
            square: Square::new(x, y),
        });
    };

    spawn_pawn(7, 3);
    spawn_pawn(6, 3);
    spawn_pawn(7, 5);
    spawn_pawn(6, 5);

    world.spawn().insert(Piece {
        kind: PieceKind::Knight,
        colour: PieceColour::White,
        square: (5, 3).into(),
    });

    world.spawn().insert(Piece {
        kind: PieceKind::Queen,
        colour: PieceColour::White,
        square: (5, 4).into(),
    });

    update_stage.run(&mut world);

    let game_state = world.get_resource::<State<GameState>>().unwrap();
    assert_eq!(
        game_state.current(),
        &GameState::Checkmate(PieceColour::Black)
    );
}

#[test]
fn should_not_allow_the_king_to_move_onto_a_square_attacked_by_a_pawn() {
    let (mut world, mut update_stage) = setup();

    let king_id = world
        .spawn()
        .insert(Piece {
            kind: PieceKind::King,
            colour: PieceColour::Black,
            square: (7, 4).into(),
        })
        .id();

    world.spawn().insert(Piece {
        kind: PieceKind::Pawn,
        colour: PieceColour::White,
        square: (5, 4).into(),
    });

    update_stage.run(&mut world);

    let all_valid_moves = world.get_resource::<AllValidMoves>().unwrap();
    assert_eq!(
        all_valid_moves.get(king_id),
        &vec![
            Move::standard((6, 4).into()),
            Move::standard((7, 3).into()),
            Move::standard((7, 5).into())
        ]
    );
}

#[test]
fn the_king_should_be_able_to_move_to_squares_that_are_potentially_attackable_but_blocked() {
    let (mut world, mut update_stage) = setup();

    let king_id = world
        .spawn()
        .insert(Piece {
            kind: PieceKind::King,
            colour: PieceColour::Black,
            square: (7, 4).into(),
        })
        .id();

    world.spawn().insert(Piece {
        kind: PieceKind::Bishop,
        colour: PieceColour::Black,
        square: (7, 5).into(),
    });

    world.spawn().insert(Piece {
        kind: PieceKind::Pawn,
        colour: PieceColour::Black,
        square: (6, 5).into(),
    });

    world.spawn().insert(Piece {
        kind: PieceKind::Pawn,
        colour: PieceColour::Black,
        square: (6, 4).into(),
    });

    world.spawn().insert(Piece {
        kind: PieceKind::Pawn,
        colour: PieceColour::Black,
        square: (6, 3).into(),
    });

    world.spawn().insert(Piece {
        kind: PieceKind::Knight,
        colour: PieceColour::White,
        square: (5, 3).into(),
    });

    world.spawn().insert(Piece {
        kind: PieceKind::Queen,
        colour: PieceColour::White,
        square: (0, 3).into(),
    });

    update_stage.run(&mut world);

    let all_valid_moves = world.get_resource::<AllValidMoves>().unwrap();
    assert_eq!(
        all_valid_moves.get(king_id),
        &vec![Move::standard((7, 3).into()),]
    );
}

#[test]
fn should_detect_stalemate_when_the_current_player_cannot_make_any_moves_but_is_not_in_check() {
    let (mut world, mut update_stage) = setup();

    world
        .spawn()
        .insert(Piece::black(PieceKind::King, Square::new(7, 4)))
        .id();
    world
        .spawn()
        .insert(Piece::black(PieceKind::Pawn, Square::new(6, 3)))
        .id();

    // moving the pawn would put the king in check
    world
        .spawn()
        .insert(Piece::white(PieceKind::Bishop, Square::new(4, 1)));
    // prevent the king from moving elsewhere
    world
        .spawn()
        .insert(Piece::white(PieceKind::Rook, Square::new(6, 7)));
    world
        .spawn()
        .insert(Piece::white(PieceKind::Rook, Square::new(0, 5)));
    world
        .spawn()
        .insert(Piece::white(PieceKind::Queen, Square::new(6, 2)));

    update_stage.run(&mut world);

    let state = world.get_resource::<State<GameState>>().unwrap();
    assert_eq!(state.current(), &GameState::Stalemate(PieceColour::Black));
}

// see bug screenshots 1
#[test]
fn fix_bug_1_incorrectly_restricted_move_calculations() {
    let (mut world, mut update_stage) = setup();

    world
        .spawn()
        .insert(Piece::white(PieceKind::King, Square::new(0, 4)));
    world
        .spawn()
        .insert(Piece::white(PieceKind::Pawn, Square::new(2, 3)));
    world
        .spawn()
        .insert(Piece::white(PieceKind::Pawn, Square::new(1, 4)));
    world
        .spawn()
        .insert(Piece::white(PieceKind::Pawn, Square::new(1, 2)));

    world
        .spawn()
        .insert(Piece::black(PieceKind::King, Square::new(7, 4)));
    world
        .spawn()
        .insert(Piece::black(PieceKind::Queen, Square::new(6, 4)));
    world
        .spawn()
        .insert(Piece::black(PieceKind::Pawn, Square::new(5, 4)));

    let queen_id = world
        .spawn()
        .insert(Piece::white(PieceKind::Queen, Square::new(0, 3)))
        .id();

    world.insert_resource(PlayerTurn(PieceColour::White));
    update_stage.run(&mut world);

    let all_valid_moves = world.get_resource::<AllValidMoves>().unwrap();
    assert_eq!(
        all_valid_moves.get(queen_id),
        &vec![
            Move::standard((1, 3).into()),
            Move::standard((0, 2).into()),
            Move::standard((0, 1).into()),
            Move::standard((0, 0).into()),
        ]
    );
}

// see bug screenshots 2
#[test]
fn fix_bug_2_incorrect_king_move_calculations() {
    let (mut world, mut update_stage) = setup();

    let king_id = world
        .spawn()
        .insert(Piece::white(PieceKind::King, Square::new(0, 3)))
        .id();
    world
        .spawn()
        .insert(Piece::white(PieceKind::Knight, Square::new(4, 4)));
    world
        .spawn()
        .insert(Piece::black(PieceKind::Rook, Square::new(7, 4)));

    world.insert_resource(PlayerTurn(PieceColour::White));
    update_stage.run(&mut world);

    let all_valid_moves = world.get_resource::<AllValidMoves>().unwrap();
    assert_eq!(
        all_valid_moves.get(king_id),
        &vec![
            Move::standard((0, 2).into()),
            Move::standard((0, 4).into()),
            Move::standard((1, 2).into()),
            Move::standard((1, 3).into()),
            Move::standard((1, 4).into()),
        ]
    );
}

// see bug screenshots 3
#[test]
fn fix_bug_3_illegal_king_move_allowed() {
    let (mut world, mut update_stage) = setup();

    let king_id = world
        .spawn()
        .insert(Piece::white(PieceKind::King, Square::new(0, 4)))
        .id();
    world
        .spawn()
        .insert(Piece::black(PieceKind::Queen, Square::new(0, 1)));

    world.insert_resource(PlayerTurn(PieceColour::White));
    update_stage.run(&mut world);

    let all_valid_moves = world.get_resource::<AllValidMoves>().unwrap();
    assert_eq!(
        all_valid_moves.get(king_id),
        &vec![
            Move::standard((1, 3).into()),
            Move::standard((1, 4).into()),
            Move::standard((1, 5).into()),
        ]
    );
}
