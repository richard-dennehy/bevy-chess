use crate::model::{AllValidMoves, CastlingData, LastPawnDoubleStep, Move, Piece, PieceColour, PieceKind, SpecialMoveData, Square};
use crate::systems::chess::{
    calculate_all_moves, apply_piece_move, GameState, MovePiece, PlayerTurn, PromotedPawn, SelectedPiece,
    SelectedSquare, Taken,
};
use bevy::ecs::component::Component;
use bevy::prelude::*;

trait WorldTestUtils {
    fn overwrite_resource<T: Component>(&mut self, resource: T);
    fn check_and_overwrite_state(&mut self, expected_state: GameState, new_state: GameState);
    fn move_piece(&mut self, piece_id: Entity, square: Square);
}

impl WorldTestUtils for World {
    fn overwrite_resource<T: Component>(&mut self, resource: T) {
        *self.get_resource_mut::<T>().unwrap() = resource;
    }

    fn check_and_overwrite_state(&mut self, expected_state: GameState, new_state: GameState) {
        let mut state = self.get_resource_mut::<State<GameState>>().unwrap();
        assert_eq!(state.current(), &expected_state);
        state.overwrite_set(new_state).unwrap();
    }

    fn move_piece(&mut self, piece_id: Entity, square: Square) {
        let all_valid_moves = self.get_resource::<AllValidMoves>().unwrap();
        let piece_moves = all_valid_moves.get(piece_id);
        assert!(
            all_valid_moves.contains(piece_id, square),
            "({}, {}) is not a valid move; valid moves: {:?}",
            square.rank,
            square.file,
            piece_moves
        );

        let piece = self.get::<Piece>(piece_id).unwrap();
        let turn = self.get_resource::<PlayerTurn>().unwrap();
        assert_eq!(
            piece.colour, turn.0,
            "Moving {:?} piece on {:?}'s turn",
            piece.colour, turn.0
        );

        self.check_and_overwrite_state(GameState::NothingSelected, GameState::TargetSquareSelected);
        self.overwrite_resource(SelectedPiece(Some(piece_id)));
        let square = self
            .query::<(Entity, &Square)>()
            .iter(self)
            .find_map(|(entity, s)| (square == *s).then(|| entity))
            .unwrap();
        self.overwrite_resource(SelectedSquare(Some(square)));
    }
}

fn setup() -> (World, SystemStage) {
    let mut world = World::new();

    world.insert_resource(AllValidMoves::default());
    world.insert_resource(PlayerTurn(PieceColour::Black));
    world.insert_resource(State::new(GameState::NothingSelected));
    world.insert_resource(SelectedSquare::default());
    world.insert_resource(SelectedPiece::default());
    world.insert_resource(PromotedPawn::default());
    world.insert_resource(SpecialMoveData::default());

    (0..8).for_each(|x| {
        (0..8).for_each(|y| {
            world.spawn().insert(Square { rank: x, file: y });
        })
    });

    let mut update_stage = SystemStage::parallel();
    update_stage.add_system_set(State::<GameState>::get_driver());
    update_stage.add_system_set(
        SystemSet::on_update(GameState::NothingSelected).with_system(calculate_all_moves.system()),
    );
    update_stage.add_system_set(
        SystemSet::on_update(GameState::TargetSquareSelected).with_system(apply_piece_move.system()),
    );
    update_stage.add_system_set(
        SystemSet::on_update(GameState::MovingPiece)
            .with_system(fake_piece_movement.system())
            .with_system(fake_despawn.system()),
    );

    (world, update_stage)
}

fn fake_piece_movement(
    mut commands: Commands,
    mut state: ResMut<State<GameState>>,
    mut turn: ResMut<PlayerTurn>,
    query: Query<(Entity, &MovePiece, &mut Piece)>,
) {
    assert_eq!(state.current(), &GameState::MovingPiece);

    query.for_each_mut(|(piece_entity, move_piece, mut piece)| {
        piece.square = move_piece.target_square();

        commands.entity(piece_entity).remove::<MovePiece>();
    });

    turn.next();
    state.set(GameState::NothingSelected).unwrap();
}

fn fake_despawn(mut commands: Commands, query: Query<Entity, With<Taken>>) {
    // leave entity with Taken component so it can be asserted against, but remove from board so it doesn't get in the way
    query.for_each_mut(|entity| {
        commands.entity(entity).remove::<Piece>();
    })
}

#[test]
fn when_a_pawn_makes_a_two_step_move_an_adjacent_pawn_can_take_it_en_passant_on_the_next_turn() {
    let (mut world, mut stage) = setup();

    world.spawn().insert(Piece {
        kind: PieceKind::King,
        colour: PieceColour::Black,
        square: (7, 4).into(),
    });

    world.spawn().insert(Piece {
        kind: PieceKind::King,
        colour: PieceColour::White,
        square: (0, 4).into(),
    });

    let black_pawn = world
        .spawn()
        .insert(Piece {
            kind: PieceKind::Pawn,
            colour: PieceColour::Black,
            square: (6, 4).into(),
        })
        .id();

    let white_pawn = world
        .spawn()
        .insert(Piece {
            kind: PieceKind::Pawn,
            colour: PieceColour::White,
            square: (4, 3).into(),
        })
        .id();

    let mut special_moves = world.get_resource_mut::<SpecialMoveData>().unwrap();
    special_moves.black_castling_data.king_moved = true;
    special_moves.white_castling_data.king_moved = true;

    stage.run(&mut world);

    world.move_piece(black_pawn, (4, 4).into());
    stage.run(&mut world);

    let special_moves = world.get_resource::<SpecialMoveData>().unwrap();
    assert_eq!(
        &special_moves.last_pawn_double_step,
        &Some(LastPawnDoubleStep {
            pawn_id: black_pawn,
            square: (4, 4).into(),
        })
    );

    assert_eq!(
        world.get_resource::<State<GameState>>().unwrap().current(),
        &GameState::NothingSelected
    );

    stage.run(&mut world);

    world.move_piece(white_pawn, (5, 4).into());
    stage.run(&mut world);

    assert!(world
        .get_resource::<SpecialMoveData>()
        .unwrap()
        .last_pawn_double_step
        .is_none());
    assert!(world.get::<Taken>(black_pawn).is_some())
}

#[test]
fn when_a_pawn_makes_a_two_step_move_an_adjacent_pawn_cannot_take_it_en_passant_if_a_turn_has_passed(
) {
    let (mut world, mut stage) = setup();

    let black_king = world
        .spawn()
        .insert(Piece {
            kind: PieceKind::King,
            colour: PieceColour::Black,
            square: (7, 4).into(),
        })
        .id();

    let white_king = world
        .spawn()
        .insert(Piece {
            kind: PieceKind::King,
            colour: PieceColour::White,
            square: (0, 4).into(),
        })
        .id();

    let black_pawn = world
        .spawn()
        .insert(Piece {
            kind: PieceKind::Pawn,
            colour: PieceColour::Black,
            square: (6, 4).into(),
        })
        .id();

    let white_pawn = world
        .spawn()
        .insert(Piece {
            kind: PieceKind::Pawn,
            colour: PieceColour::White,
            square: (4, 3).into(),
        })
        .id();

    let mut special_moves = world.get_resource_mut::<SpecialMoveData>().unwrap();
    special_moves.black_castling_data.king_moved = true;
    special_moves.white_castling_data.king_moved = true;

    stage.run(&mut world);

    // turn 0 move black pawn 2 steps forward
    world.move_piece(black_pawn, (4, 4).into());
    stage.run(&mut world);

    let special_moves = world.get_resource::<SpecialMoveData>().unwrap();
    assert_eq!(
        &special_moves.last_pawn_double_step,
        &Some(LastPawnDoubleStep {
            pawn_id: black_pawn,
            square: (4, 4).into()
        })
    );

    stage.run(&mut world);
    let all_valid_moves = world.get_resource::<AllValidMoves>().unwrap();
    assert_eq!(
        all_valid_moves.get(white_pawn),
        &vec![
            Move::standard((5, 3).into()),
            Move::en_passant((5, 4).into(), black_pawn)
        ]
    );

    world.move_piece(white_king, (1, 4).into());
    stage.run(&mut world);

    world.move_piece(black_king, (6, 4).into());
    stage.run(&mut world);

    // check white pawn can't still move en passant
    let all_valid_moves = world.get_resource::<AllValidMoves>().unwrap();
    assert_eq!(
        all_valid_moves.get(white_pawn),
        &vec![Move::standard((5, 3).into())]
    );
}

#[test]
fn it_should_be_possible_to_take_a_pawn_with_the_king_in_check_using_en_passant() {
    let (mut world, mut stage) = setup();

    world.spawn().insert(Piece {
        kind: PieceKind::King,
        colour: PieceColour::Black,
        square: (7, 4).into(),
    });

    let white_king = world
        .spawn()
        .insert(Piece {
            kind: PieceKind::King,
            colour: PieceColour::White,
            square: (3, 3).into(),
        })
        .id();

    let black_pawn = world
        .spawn()
        .insert(Piece {
            kind: PieceKind::Pawn,
            colour: PieceColour::Black,
            square: (6, 4).into(),
        })
        .id();

    let white_pawn = world
        .spawn()
        .insert(Piece {
            kind: PieceKind::Pawn,
            colour: PieceColour::White,
            square: (4, 5).into(),
        })
        .id();

    // prevent the white king from being able to move
    world.spawn().insert(Piece {
        kind: PieceKind::Rook,
        colour: PieceColour::Black,
        square: (0, 4).into(),
    });
    world.spawn().insert(Piece {
        kind: PieceKind::Rook,
        colour: PieceColour::Black,
        square: (0, 2).into(),
    });
    world.spawn().insert(Piece {
        kind: PieceKind::Rook,
        colour: PieceColour::Black,
        square: (4, 0).into(),
    });
    world.spawn().insert(Piece {
        kind: PieceKind::Rook,
        colour: PieceColour::Black,
        square: (2, 0).into(),
    });

    let mut special_moves = world.get_resource_mut::<SpecialMoveData>().unwrap();
    special_moves.black_castling_data.king_moved = true;
    special_moves.white_castling_data.king_moved = true;

    stage.run(&mut world);

    world.move_piece(black_pawn, (4, 4).into());
    stage.run(&mut world);

    let special_moves = world.get_resource::<SpecialMoveData>().unwrap();
    assert_eq!(
        &special_moves.last_pawn_double_step,
        &Some(LastPawnDoubleStep {
            pawn_id: black_pawn,
            square: (4, 4).into()
        })
    );

    assert_eq!(
        world.get_resource::<State<GameState>>().unwrap().current(),
        &GameState::NothingSelected
    );

    let all_valid_moves = world.get_resource::<AllValidMoves>().unwrap();
    assert_eq!(all_valid_moves.get(white_king), &vec![]);
    assert_eq!(
        all_valid_moves.get(white_pawn),
        &vec![Move::en_passant((5, 4).into(), black_pawn)]
    );
}

#[test]
fn it_should_not_be_possible_to_take_a_pawn_en_passant_if_it_would_expose_the_king_to_check() {
    let (mut world, mut stage) = setup();

    world.spawn().insert(Piece {
        kind: PieceKind::King,
        colour: PieceColour::Black,
        square: (7, 4).into(),
    });

    world.spawn().insert(Piece {
        kind: PieceKind::King,
        colour: PieceColour::White,
        square: (3, 3).into(),
    });

    let black_pawn = world
        .spawn()
        .insert(Piece {
            kind: PieceKind::Pawn,
            colour: PieceColour::Black,
            square: (6, 4).into(),
        })
        .id();

    world.spawn().insert(Piece {
        kind: PieceKind::Pawn,
        colour: PieceColour::White,
        square: (4, 3).into(),
    });

    // prevent the white king from being able to move
    world.spawn().insert(Piece {
        kind: PieceKind::Rook,
        colour: PieceColour::Black,
        square: (0, 4).into(),
    });
    world.spawn().insert(Piece {
        kind: PieceKind::Rook,
        colour: PieceColour::Black,
        square: (0, 2).into(),
    });
    world.spawn().insert(Piece {
        kind: PieceKind::Rook,
        colour: PieceColour::Black,
        square: (4, 0).into(),
    });
    world.spawn().insert(Piece {
        kind: PieceKind::Rook,
        colour: PieceColour::Black,
        square: (2, 0).into(),
    });

    // prevents the pawn from moving
    world.spawn().insert(Piece {
        kind: PieceKind::Rook,
        colour: PieceColour::Black,
        square: (0, 3).into(),
    });

    let mut special_moves = world.get_resource_mut::<SpecialMoveData>().unwrap();
    special_moves.black_castling_data.king_moved = true;
    special_moves.white_castling_data.king_moved = true;

    stage.run(&mut world);

    world.move_piece(black_pawn, (4, 4).into());
    stage.run(&mut world);

    let special_moves = world.get_resource::<SpecialMoveData>().unwrap();
    assert_eq!(
        &special_moves.last_pawn_double_step,
        &Some(LastPawnDoubleStep {
            pawn_id: black_pawn,
            square: (4, 4).into()
        })
    );

    assert_eq!(
        world.get_resource::<State<GameState>>().unwrap().current(),
        &GameState::Checkmate(PieceColour::White)
    );
}

#[test]
fn it_should_not_be_possible_to_use_en_passant_if_the_king_is_in_check_and_en_passant_would_not_counter_it(
) {
    let (mut world, mut stage) = setup();

    world.spawn().insert(Piece {
        kind: PieceKind::King,
        colour: PieceColour::Black,
        square: (7, 0).into(),
    });

    world.spawn().insert(Piece {
        kind: PieceKind::King,
        colour: PieceColour::White,
        square: (4, 2).into(),
    });

    let black_pawn = world
        .spawn()
        .insert(Piece {
            kind: PieceKind::Pawn,
            colour: PieceColour::Black,
            square: (6, 4).into(),
        })
        .id();

    world.spawn().insert(Piece {
        kind: PieceKind::Bishop,
        colour: PieceColour::Black,
        square: (7, 5).into(),
    });

    let white_pawn = world
        .spawn()
        .insert(Piece {
            kind: PieceKind::Pawn,
            colour: PieceColour::White,
            square: (4, 3).into(),
        })
        .id();

    world.overwrite_resource(SpecialMoveData {
        black_castling_data: CastlingData {
            kingside_rook_moved: true,
            queenside_rook_moved: true,
            king_moved: true,
        },
        white_castling_data: CastlingData {
            kingside_rook_moved: true,
            queenside_rook_moved: true,
            king_moved: true,
        },
        ..Default::default()
    });

    stage.run(&mut world);

    world.move_piece(black_pawn, (4, 4).into());
    stage.run(&mut world);

    let special_moves = world.get_resource::<SpecialMoveData>().unwrap();
    assert_eq!(
        &special_moves.last_pawn_double_step,
        &Some(LastPawnDoubleStep {
            pawn_id: black_pawn,
            square: (4, 4).into()
        })
    );

    assert_eq!(
        world.get_resource::<State<GameState>>().unwrap().current(),
        &GameState::NothingSelected
    );

    let all_valid_moves = world.get_resource::<AllValidMoves>().unwrap();
    assert_eq!(
        all_valid_moves.get(white_pawn),
        &vec![Move::standard((5, 3).into())]
    );
}

#[test]
fn it_should_not_be_possible_to_capture_en_passant_if_the_target_is_not_directly_to_the_left_or_right(
) {
    let (mut world, mut stage) = setup();

    world
        .spawn()
        .insert(Piece::black(PieceKind::King, Square::new(7, 4)));
    world
        .spawn()
        .insert(Piece::white(PieceKind::King, Square::new(0, 4)));

    let black_pawn = world
        .spawn()
        .insert(Piece::black(PieceKind::Pawn, Square::new(6, 4)))
        .id();

    let white_pawn = world
        .spawn()
        .insert(Piece::white(PieceKind::Pawn, Square::new(1, 3)))
        .id();

    let mut special_moves = world.get_resource_mut::<SpecialMoveData>().unwrap();
    special_moves.black_castling_data.king_moved = true;
    special_moves.white_castling_data.king_moved = true;

    stage.run(&mut world);

    world.move_piece(black_pawn, (4, 4).into());
    stage.run(&mut world);

    let special_moves = world.get_resource::<SpecialMoveData>().unwrap();
    assert_eq!(
        &special_moves.last_pawn_double_step,
        &Some(LastPawnDoubleStep {
            pawn_id: black_pawn,
            square: (4, 4).into(),
        })
    );

    assert_eq!(
        world.get_resource::<State<GameState>>().unwrap().current(),
        &GameState::NothingSelected
    );

    stage.run(&mut world);

    let all_valid_moves = world.get_resource::<AllValidMoves>().unwrap();
    assert_eq!(
        all_valid_moves.get(white_pawn),
        &vec![
            Move::standard((2, 3).into()),
            Move::pawn_double_step((3, 3).into())
        ]
    );
}

#[test]
fn a_pawn_on_an_edge_of_the_board_double_stepping_should_not_cause_overflow_when_checking_for_en_passant(
) {
    let (mut world, mut stage) = setup();

    world
        .spawn()
        .insert(Piece::black(PieceKind::King, Square::new(7, 4)));
    world
        .spawn()
        .insert(Piece::white(PieceKind::King, Square::new(0, 4)));

    let black_pawn = world
        .spawn()
        .insert(Piece::black(PieceKind::Pawn, Square::new(6, 0)))
        .id();

    let white_pawn = world
        .spawn()
        .insert(Piece::white(PieceKind::Pawn, Square::new(4, 1)))
        .id();

    let mut special_moves = world.get_resource_mut::<SpecialMoveData>().unwrap();
    special_moves.black_castling_data.king_moved = true;
    special_moves.white_castling_data.king_moved = true;

    stage.run(&mut world);

    world.move_piece(black_pawn, (4, 0).into());
    stage.run(&mut world);

    let special_moves = world.get_resource::<SpecialMoveData>().unwrap();
    assert_eq!(
        &special_moves.last_pawn_double_step,
        &Some(LastPawnDoubleStep {
            pawn_id: black_pawn,
            square: (4, 0).into(),
        })
    );

    assert_eq!(
        world.get_resource::<State<GameState>>().unwrap().current(),
        &GameState::NothingSelected
    );

    stage.run(&mut world);

    world.move_piece(white_pawn, (5, 0).into());
    stage.run(&mut world);

    assert!(world
        .get_resource::<SpecialMoveData>()
        .unwrap()
        .last_pawn_double_step
        .is_none());
    assert!(world.get::<Taken>(black_pawn).is_some())
}

#[test]
fn it_should_be_possible_to_castle_queenside_if_neither_the_king_nor_the_rook_have_moved() {
    let (mut world, mut stage) = setup();

    let black_king = world
        .spawn()
        .insert(Piece {
            kind: PieceKind::King,
            colour: PieceColour::Black,
            square: (7, 4).into(),
        })
        .id();

    world.spawn().insert(Piece {
        kind: PieceKind::King,
        colour: PieceColour::White,
        square: (0, 3).into(),
    });

    let black_rook = world
        .spawn()
        .insert(Piece {
            kind: PieceKind::Rook,
            colour: PieceColour::Black,
            square: (7, 0).into(),
        })
        .id();

    let mut special_moves = world.get_resource_mut::<SpecialMoveData>().unwrap();
    special_moves.black_castling_data.kingside_rook_moved = true;

    stage.run(&mut world);

    world.move_piece(black_king, (7, 0).into());
    stage.run(&mut world);

    let black_king = world.get::<Piece>(black_king).unwrap();
    assert_eq!(black_king.square.rank, 7);
    assert_eq!(black_king.square.file, 2);

    let black_rook = world.get::<Piece>(black_rook).unwrap();
    assert_eq!(black_rook.square.rank, 7);
    assert_eq!(black_rook.square.file, 3);
}

#[test]
fn it_should_be_possible_to_castle_kingside_if_neither_the_king_nor_the_rook_have_moved() {
    let (mut world, mut stage) = setup();

    let white_king = world
        .spawn()
        .insert(Piece {
            kind: PieceKind::King,
            colour: PieceColour::White,
            square: (0, 4).into(),
        })
        .id();

    world.spawn().insert(Piece {
        kind: PieceKind::King,
        colour: PieceColour::Black,
        square: (7, 4).into(),
    });

    let white_rook = world
        .spawn()
        .insert(Piece {
            kind: PieceKind::Rook,
            colour: PieceColour::White,
            square: (0, 7).into(),
        })
        .id();

    let mut special_moves = world.get_resource_mut::<SpecialMoveData>().unwrap();
    special_moves.white_castling_data.queenside_rook_moved = true;
    special_moves.black_castling_data.king_moved = true;

    world.overwrite_resource(PlayerTurn(PieceColour::White));

    stage.run(&mut world);

    world.move_piece(white_king, (0, 7).into());
    stage.run(&mut world);

    let white_king = world.get::<Piece>(white_king).unwrap();
    assert_eq!(white_king.square.rank, 0);
    assert_eq!(white_king.square.file, 6);

    let white_rook = world.get::<Piece>(white_rook).unwrap();
    assert_eq!(white_rook.square.rank, 0);
    assert_eq!(white_rook.square.file, 5);
}

#[test]
fn it_should_not_be_possible_to_castle_if_the_king_has_moved() {
    let (mut world, mut stage) = setup();

    let white_king = world
        .spawn()
        .insert(Piece {
            kind: PieceKind::King,
            colour: PieceColour::White,
            square: (0, 4).into(),
        })
        .id();

    let black_king = world
        .spawn()
        .insert(Piece {
            kind: PieceKind::King,
            colour: PieceColour::Black,
            square: (7, 4).into(),
        })
        .id();

    let kingside_rook = Piece::white(PieceKind::Rook, (0, 7).into());
    let kingside_rook_id = world.spawn().insert(kingside_rook).id();

    let queenside_rook = Piece::white(PieceKind::Rook, (0, 0).into());
    let queenside_rook_id = world.spawn().insert(queenside_rook).id();

    world.overwrite_resource(PlayerTurn(PieceColour::White));
    let mut special_moves = world.get_resource_mut::<SpecialMoveData>().unwrap();
    special_moves.black_castling_data.queenside_rook_moved = true;
    special_moves.black_castling_data.kingside_rook_moved = true;

    stage.run(&mut world);

    let all_valid_moves = world.get_resource::<AllValidMoves>().unwrap();
    assert_eq!(
        all_valid_moves.get(white_king),
        &vec![
            Move::standard((0, 3).into()),
            Move::standard((0, 5).into()),
            Move::standard((1, 3).into()),
            Move::standard((1, 4).into()),
            Move::standard((1, 5).into()),
            Move::queenside_castle((0, 0).into(), queenside_rook_id, queenside_rook),
            Move::kingside_castle((0, 7).into(), kingside_rook_id, kingside_rook)
        ]
    );

    world.move_piece(white_king, (0, 5).into());
    stage.run(&mut world);

    world.move_piece(black_king, (7, 5).into());
    stage.run(&mut world);

    let all_valid_moves = world.get_resource::<AllValidMoves>().unwrap();
    assert_eq!(
        all_valid_moves.get(white_king),
        &vec![
            Move::standard((0, 4).into()),
            Move::standard((0, 6).into()),
            Move::standard((1, 4).into()),
            Move::standard((1, 5).into()),
            Move::standard((1, 6).into())
        ]
    );

    world.move_piece(white_king, (0, 4).into());
    stage.run(&mut world);

    world.move_piece(black_king, (7, 4).into());
    stage.run(&mut world);

    let all_valid_moves = world.get_resource::<AllValidMoves>().unwrap();
    assert_eq!(
        all_valid_moves.get(white_king),
        &vec![
            Move::standard((0, 3).into()),
            Move::standard((0, 5).into()),
            Move::standard((1, 3).into()),
            Move::standard((1, 4).into()),
            Move::standard((1, 5).into())
        ]
    );
}

#[test]
fn it_should_not_be_possible_to_castle_kingside_if_the_rook_has_moved() {
    let (mut world, mut stage) = setup();

    let white_king = world
        .spawn()
        .insert(Piece {
            kind: PieceKind::King,
            colour: PieceColour::White,
            square: (0, 4).into(),
        })
        .id();

    let black_king = world
        .spawn()
        .insert(Piece {
            kind: PieceKind::King,
            colour: PieceColour::Black,
            square: (7, 4).into(),
        })
        .id();

    let kingside_rook = Piece::white(PieceKind::Rook, (0, 7).into());
    let kingside_rook_id = world.spawn().insert(kingside_rook).id();

    let queenside_rook = Piece::white(PieceKind::Rook, (0, 0).into());
    let queenside_rook_id = world.spawn().insert(queenside_rook).id();

    world.overwrite_resource(PlayerTurn(PieceColour::White));
    let mut special_moves = world.get_resource_mut::<SpecialMoveData>().unwrap();
    special_moves.black_castling_data.queenside_rook_moved = true;
    special_moves.black_castling_data.kingside_rook_moved = true;

    stage.run(&mut world);

    let all_valid_moves = world.get_resource::<AllValidMoves>().unwrap();
    assert_eq!(
        all_valid_moves.get(white_king),
        &vec![
            Move::standard((0, 3).into()),
            Move::standard((0, 5).into()),
            Move::standard((1, 3).into()),
            Move::standard((1, 4).into()),
            Move::standard((1, 5).into()),
            Move::queenside_castle((0, 0).into(), queenside_rook_id, queenside_rook),
            Move::kingside_castle((0, 7).into(), kingside_rook_id, kingside_rook)
        ]
    );

    world.move_piece(kingside_rook_id, (1, 7).into());
    stage.run(&mut world);

    world.move_piece(black_king, (7, 5).into());
    stage.run(&mut world);

    let all_valid_moves = world.get_resource::<AllValidMoves>().unwrap();
    assert_eq!(
        all_valid_moves.get(white_king),
        &vec![
            Move::standard((0, 3).into()),
            Move::standard((0, 5).into()),
            Move::standard((1, 3).into()),
            Move::standard((1, 4).into()),
            Move::standard((1, 5).into()),
            Move::queenside_castle((0, 0).into(), queenside_rook_id, queenside_rook)
        ]
    );

    world.move_piece(kingside_rook_id, (0, 7).into());
    stage.run(&mut world);

    world.move_piece(black_king, (7, 4).into());
    stage.run(&mut world);

    let all_valid_moves = world.get_resource::<AllValidMoves>().unwrap();
    assert_eq!(
        all_valid_moves.get(white_king),
        &vec![
            Move::standard((0, 3).into()),
            Move::standard((0, 5).into()),
            Move::standard((1, 3).into()),
            Move::standard((1, 4).into()),
            Move::standard((1, 5).into()),
            Move::queenside_castle((0, 0).into(), queenside_rook_id, queenside_rook)
        ]
    );
}

#[test]
fn it_should_not_be_possible_to_castle_queenside_if_the_rook_has_moved() {
    let (mut world, mut stage) = setup();

    let white_king = world
        .spawn()
        .insert(Piece {
            kind: PieceKind::King,
            colour: PieceColour::White,
            square: (0, 4).into(),
        })
        .id();

    let black_king = world
        .spawn()
        .insert(Piece {
            kind: PieceKind::King,
            colour: PieceColour::Black,
            square: (7, 4).into(),
        })
        .id();

    let kingside_rook = Piece::white(PieceKind::Rook, (0, 7).into());
    let kingside_rook_id = world.spawn().insert(kingside_rook).id();

    let queenside_rook = Piece::white(PieceKind::Rook, (0, 0).into());
    let queenside_rook_id = world.spawn().insert(queenside_rook).id();

    world.overwrite_resource(PlayerTurn(PieceColour::White));
    let mut special_moves = world.get_resource_mut::<SpecialMoveData>().unwrap();
    special_moves.black_castling_data.queenside_rook_moved = true;
    special_moves.black_castling_data.kingside_rook_moved = true;

    stage.run(&mut world);

    let all_valid_moves = world.get_resource::<AllValidMoves>().unwrap();
    assert_eq!(
        all_valid_moves.get(white_king),
        &vec![
            Move::standard((0, 3).into()),
            Move::standard((0, 5).into()),
            Move::standard((1, 3).into()),
            Move::standard((1, 4).into()),
            Move::standard((1, 5).into()),
            Move::queenside_castle((0, 0).into(), queenside_rook_id, queenside_rook),
            Move::kingside_castle((0, 7).into(), kingside_rook_id, kingside_rook)
        ]
    );

    world.move_piece(queenside_rook_id, (1, 0).into());
    stage.run(&mut world);

    world.move_piece(black_king, (7, 5).into());
    stage.run(&mut world);

    let all_valid_moves = world.get_resource::<AllValidMoves>().unwrap();
    assert_eq!(
        all_valid_moves.get(white_king),
        &vec![
            Move::standard((0, 3).into()),
            Move::standard((0, 5).into()),
            Move::standard((1, 3).into()),
            Move::standard((1, 4).into()),
            Move::standard((1, 5).into()),
            Move::kingside_castle((0, 7).into(), kingside_rook_id, kingside_rook)
        ]
    );

    world.move_piece(queenside_rook_id, (0, 0).into());
    stage.run(&mut world);

    world.move_piece(black_king, (7, 4).into());
    stage.run(&mut world);

    let all_valid_moves = world.get_resource::<AllValidMoves>().unwrap();
    assert_eq!(
        all_valid_moves.get(white_king),
        &vec![
            Move::standard((0, 3).into()),
            Move::standard((0, 5).into()),
            Move::standard((1, 3).into()),
            Move::standard((1, 4).into()),
            Move::standard((1, 5).into()),
            Move::kingside_castle((0, 7).into(), kingside_rook_id, kingside_rook)
        ]
    );
}

#[test]
fn it_should_not_be_possible_to_castle_if_the_king_is_in_check() {
    let (mut world, mut stage) = setup();

    let white_king = world
        .spawn()
        .insert(Piece {
            kind: PieceKind::King,
            colour: PieceColour::White,
            square: (0, 4).into(),
        })
        .id();

    world
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
        square: (0, 7).into(),
    });

    world.spawn().insert(Piece {
        kind: PieceKind::Rook,
        colour: PieceColour::White,
        square: (0, 0).into(),
    });

    world.spawn().insert(Piece {
        kind: PieceKind::Knight,
        colour: PieceColour::Black,
        square: (2, 5).into(),
    });

    world.overwrite_resource(PlayerTurn(PieceColour::White));
    stage.run(&mut world);

    let all_valid_moves = world.get_resource::<AllValidMoves>().unwrap();
    assert_eq!(
        all_valid_moves.get(white_king),
        &vec![
            Move::standard((0, 3).into()),
            Move::standard((0, 5).into()),
            Move::standard((1, 4).into()),
            Move::standard((1, 5).into())
        ]
    );
}

#[test]
fn it_should_not_be_possible_to_castle_if_any_square_the_king_would_move_through_is_attacked() {
    let (mut world, mut stage) = setup();

    let white_king = world
        .spawn()
        .insert(Piece {
            kind: PieceKind::King,
            colour: PieceColour::White,
            square: (0, 4).into(),
        })
        .id();

    world
        .spawn()
        .insert(Piece {
            kind: PieceKind::King,
            colour: PieceColour::Black,
            square: (7, 4).into(),
        })
        .id();

    let kingside_rook = Piece::white(PieceKind::Rook, (0, 7).into());
    let kingside_rook_id = world.spawn().insert(kingside_rook).id();

    world.spawn().insert(Piece {
        kind: PieceKind::Rook,
        colour: PieceColour::White,
        square: (0, 0).into(),
    });

    world.spawn().insert(Piece {
        kind: PieceKind::Knight,
        colour: PieceColour::Black,
        square: (2, 2).into(),
    });

    world.overwrite_resource(PlayerTurn(PieceColour::White));
    stage.run(&mut world);

    let all_valid_moves = world.get_resource::<AllValidMoves>().unwrap();
    assert_eq!(
        all_valid_moves.get(white_king),
        &vec![
            Move::standard((0, 5).into()),
            Move::standard((1, 3).into()),
            Move::standard((1, 5).into()),
            Move::kingside_castle((0, 7).into(), kingside_rook_id, kingside_rook)
        ]
    );
}

#[test]
fn it_should_not_be_possible_to_castle_if_the_rook_has_been_taken() {
    let (mut world, mut stage) = setup();

    let white_king = world
        .spawn()
        .insert(Piece {
            kind: PieceKind::King,
            colour: PieceColour::White,
            square: (0, 4).into(),
        })
        .id();

    world
        .spawn()
        .insert(Piece {
            kind: PieceKind::King,
            colour: PieceColour::Black,
            square: (7, 4).into(),
        })
        .id();

    let kingside_rook = Piece::white(PieceKind::Rook, (0, 7).into());
    let kingside_rook_id = world.spawn().insert(kingside_rook).id();

    world.spawn().insert(Piece {
        kind: PieceKind::Rook,
        colour: PieceColour::White,
        square: (0, 0).into(),
    });

    let black_knight = world
        .spawn()
        .insert(Piece {
            kind: PieceKind::Knight,
            colour: PieceColour::Black,
            square: (2, 1).into(),
        })
        .id();

    let mut special_moves = world.get_resource_mut::<SpecialMoveData>().unwrap();
    special_moves.black_castling_data.king_moved = true;

    stage.run(&mut world);

    world.move_piece(black_knight, (0, 0).into());
    stage.run(&mut world);

    let all_valid_moves = world.get_resource::<AllValidMoves>().unwrap();
    assert_eq!(
        all_valid_moves.get(white_king),
        &vec![
            Move::standard((0, 3).into()),
            Move::standard((0, 5).into()),
            Move::standard((1, 3).into()),
            Move::standard((1, 4).into()),
            Move::standard((1, 5).into()),
            Move::kingside_castle((0, 7).into(), kingside_rook_id, kingside_rook)
        ]
    );
}
