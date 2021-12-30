mod board_tests {
    use crate::board::BoardState;
    use crate::pieces::{Piece, PieceColour, PieceKind};

    #[test]
    fn board_state_for_default_board() {
        let back_row = [
            PieceKind::Rook,
            PieceKind::Knight,
            PieceKind::Bishop,
            PieceKind::Queen,
            PieceKind::King,
            PieceKind::Bishop,
            PieceKind::Knight,
            PieceKind::Rook,
        ];
        let front_row = |colour: PieceColour| {
            if colour == PieceColour::White {
                (0..8)
                    .map(|idx| Piece {
                        x: 1,
                        y: idx,
                        kind: PieceKind::Pawn,
                        colour,
                    })
                    .collect::<Vec<_>>()
            } else {
                (0..8)
                    .map(|idx| Piece {
                        x: 6,
                        y: idx,
                        kind: PieceKind::Pawn,
                        colour,
                    })
                    .collect::<Vec<_>>()
            }
        };

        let pieces = back_row
            .iter()
            .enumerate()
            .map(|(idx, kind)| Piece {
                x: 0,
                y: idx as _,
                colour: PieceColour::White,
                kind: *kind,
            })
            .chain(front_row(PieceColour::White).into_iter())
            .chain(front_row(PieceColour::Black).into_iter())
            .chain(back_row.iter().enumerate().map(|(idx, kind)| Piece {
                x: 7,
                y: idx as _,
                colour: PieceColour::Black,
                kind: *kind,
            }))
            .collect::<Vec<_>>();

        let expected = [Some(PieceColour::White); 16]
            .into_iter()
            .chain([None; 32].into_iter())
            .chain([Some(PieceColour::Black); 16].into_iter())
            .collect::<Vec<_>>();
        assert_eq!(BoardState::from(&pieces[..]).squares(), &expected);
    }

    mod checking_for_check {
        use super::*;
        use crate::board::{
            calculate_all_moves, AllValidMoves, CastlingData, GameState, PlayerTurn,
            SpecialMoveData,
        };
        use crate::moves_calculator::Move;
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
                    x: 7,
                    y: 4,
                })
                .id();

            // knight has king in check
            world.spawn().insert(Piece {
                kind: PieceKind::Knight,
                colour: PieceColour::White,
                x: 5,
                y: 3,
            });

            // add other pieces around King to restrict (but not totally block) movement that can't take the knight,
            // to verify that even though they can move, they aren't selectable while the king is in check
            let rook_id = world
                .spawn()
                .insert(Piece {
                    kind: PieceKind::Rook,
                    colour: PieceColour::Black,
                    x: 7,
                    y: 5,
                })
                .id();

            let knight_id = world
                .spawn()
                .insert(Piece {
                    kind: PieceKind::Knight,
                    colour: PieceColour::Black,
                    x: 6,
                    y: 3,
                })
                .id();

            let queen_id = world
                .spawn()
                .insert(Piece {
                    kind: PieceKind::Queen,
                    colour: PieceColour::Black,
                    x: 7,
                    y: 3,
                })
                .id();

            update_stage.run(&mut world);
            let valid_moves = world.get_resource::<AllValidMoves>().unwrap();

            assert_eq!(valid_moves.get(king_id), &vec![Move::standard((6, 4))]);
            assert_eq!(valid_moves.get(rook_id), &vec![]);
            assert_eq!(valid_moves.get(knight_id), &vec![]);
            assert_eq!(valid_moves.get(queen_id), &vec![]);
        }

        #[test]
        fn should_detect_checkmate_when_the_king_cannot_move_and_the_opposing_piece_cannot_be_taken_or_blocked(
        ) {
            let (mut world, mut update_stage) = setup();
            let mut ids = vec![];

            ids.push(
                world
                    .spawn()
                    .insert(Piece {
                        kind: PieceKind::King,
                        colour: PieceColour::Black,
                        x: 7,
                        y: 4,
                    })
                    .id(),
            );

            // knight has king in check
            world.spawn().insert(Piece {
                kind: PieceKind::Knight,
                colour: PieceColour::White,
                x: 5,
                y: 3,
            });

            // pieces blocking the king but tragically unable to take the knight
            let mut spawn_pawn = |x: u8, y: u8| {
                ids.push(
                    world
                        .spawn()
                        .insert(Piece {
                            kind: PieceKind::Pawn,
                            colour: PieceColour::Black,
                            x,
                            y,
                        })
                        .id(),
                );
            };

            spawn_pawn(7, 3);
            spawn_pawn(6, 3);
            spawn_pawn(7, 5);
            spawn_pawn(6, 5);

            // can't place pawn here or it would be able to take the knight
            ids.push(
                world
                    .spawn()
                    .insert(Piece {
                        kind: PieceKind::Rook,
                        colour: PieceColour::Black,
                        x: 6,
                        y: 4,
                    })
                    .id(),
            );

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

            let game_state = world.get_resource::<State<GameState>>().unwrap();
            assert_eq!(
                game_state.current(),
                &GameState::Checkmate(PieceColour::Black)
            );
        }

        #[test]
        fn should_not_detect_checkmate_if_the_king_cannot_move_but_the_opposing_piece_can_be_taken()
        {
            let (mut world, mut update_stage) = setup();
            let mut ids = vec![];

            ids.push(
                world
                    .spawn()
                    .insert(Piece {
                        kind: PieceKind::King,
                        colour: PieceColour::Black,
                        x: 7,
                        y: 4,
                    })
                    .id(),
            );

            // knight has king in check
            world.spawn().insert(Piece {
                kind: PieceKind::Knight,
                colour: PieceColour::White,
                x: 5,
                y: 3,
            });

            let mut spawn_pawn = |x: u8, y: u8| {
                world
                    .spawn()
                    .insert(Piece {
                        kind: PieceKind::Pawn,
                        colour: PieceColour::Black,
                        x,
                        y,
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

            assert_eq!(all_valid_moves.get(pawn_id), &vec![Move::standard((5, 3))]);

            let game_state = world.get_resource::<State<GameState>>().unwrap();
            assert_eq!(game_state.current(), &GameState::NothingSelected);
        }

        #[test]
        fn should_detect_checkmate_if_multiple_pieces_have_the_king_in_check() {
            let (mut world, mut update_stage) = setup();
            let mut ids = vec![];

            ids.push(
                world
                    .spawn()
                    .insert(Piece {
                        kind: PieceKind::King,
                        colour: PieceColour::Black,
                        x: 7,
                        y: 4,
                    })
                    .id(),
            );

            // knight has king in check
            world.spawn().insert(Piece {
                kind: PieceKind::Knight,
                colour: PieceColour::White,
                x: 5,
                y: 3,
            });

            // also has king in check
            world.spawn().insert(Piece {
                kind: PieceKind::Knight,
                colour: PieceColour::White,
                x: 5,
                y: 5,
            });

            let mut spawn_pawn = |x: u8, y: u8| {
                world
                    .spawn()
                    .insert(Piece {
                        kind: PieceKind::Pawn,
                        colour: PieceColour::Black,
                        x,
                        y,
                    })
                    .id()
            };

            ids.push(spawn_pawn(7, 3));
            ids.push(spawn_pawn(6, 3));
            ids.push(spawn_pawn(7, 5));
            ids.push(spawn_pawn(6, 5));
            ids.push(spawn_pawn(6, 4));

            update_stage.run(&mut world);

            let all_valid_moves = world.get_resource::<AllValidMoves>().unwrap();
            ids.into_iter()
                .for_each(|id| assert!(all_valid_moves.get(id).is_empty()));

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
                    x: 7,
                    y: 4,
                })
                .id();

            world.spawn().insert(Piece {
                kind: PieceKind::Rook,
                colour: PieceColour::White,
                x: 0,
                y: 3,
            });

            world.spawn().insert(Piece {
                kind: PieceKind::Rook,
                colour: PieceColour::White,
                x: 0,
                y: 5,
            });

            update_stage.run(&mut world);

            let all_valid_moves = world.get_resource::<AllValidMoves>().unwrap();
            assert_eq!(all_valid_moves.get(king_id), &vec![Move::standard((6, 4))]);
        }

        #[test]
        fn should_not_allow_the_king_to_take_a_piece_if_it_would_leave_the_king_in_check() {
            let (mut world, mut update_stage) = setup();

            let black_king = world
                .spawn()
                .insert(Piece {
                    kind: PieceKind::King,
                    colour: PieceColour::Black,
                    x: 3,
                    y: 3,
                })
                .id();

            world.spawn().insert(Piece {
                kind: PieceKind::Pawn,
                colour: PieceColour::White,
                x: 2,
                y: 4,
            });
            world.spawn().insert(Piece {
                kind: PieceKind::Rook,
                colour: PieceColour::White,
                x: 7,
                y: 4,
            });
            world.spawn().insert(Piece {
                kind: PieceKind::Rook,
                colour: PieceColour::White,
                x: 0,
                y: 2,
            });
            world.spawn().insert(Piece {
                kind: PieceKind::Rook,
                colour: PieceColour::White,
                x: 4,
                y: 0,
            });
            world.spawn().insert(Piece {
                kind: PieceKind::Rook,
                colour: PieceColour::White,
                x: 2,
                y: 0,
            });

            update_stage.run(&mut world);

            let all_valid_moves = world.get_resource::<AllValidMoves>().unwrap();
            assert_eq!(all_valid_moves.get(black_king), &vec![]);
            assert_eq!(
                world.get_resource::<State<GameState>>().unwrap().current(),
                &GameState::Checkmate(PieceColour::Black)
            );
        }

        #[test]
        fn should_detect_checkmate_if_the_king_is_in_check_and_cannot_move_out_of_check() {
            let (mut world, mut update_stage) = setup();
            let mut ids = vec![];

            ids.push(
                world
                    .spawn()
                    .insert(Piece {
                        kind: PieceKind::King,
                        colour: PieceColour::Black,
                        x: 7,
                        y: 4,
                    })
                    .id(),
            );

            // bishop has king in check
            world.spawn().insert(Piece {
                kind: PieceKind::Bishop,
                colour: PieceColour::White,
                x: 5,
                y: 2,
            });

            let mut spawn_pawn = |x: u8, y: u8| {
                ids.push(
                    world
                        .spawn()
                        .insert(Piece {
                            kind: PieceKind::Pawn,
                            colour: PieceColour::Black,
                            x,
                            y,
                        })
                        .id(),
                );
            };

            spawn_pawn(7, 5);
            spawn_pawn(6, 5);
            spawn_pawn(6, 4);

            ids.push(
                world
                    .spawn()
                    .insert(Piece {
                        kind: PieceKind::Bishop,
                        colour: PieceColour::Black,
                        x: 7,
                        y: 3,
                    })
                    .id(),
            );

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
                        x: 7,
                        y: 4,
                    })
                    .id(),
            );

            // bishop has king in check
            world.spawn().insert(Piece {
                kind: PieceKind::Bishop,
                colour: PieceColour::White,
                x: 5,
                y: 2,
            });

            let mut spawn_pawn = |x: u8, y: u8| {
                world
                    .spawn()
                    .insert(Piece {
                        kind: PieceKind::Pawn,
                        colour: PieceColour::Black,
                        x,
                        y,
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
                &vec![Move::standard((6, 3))]
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
                x: 7,
                y: 4,
            });

            // blocked by rook
            world.spawn().insert(Piece {
                kind: PieceKind::Bishop,
                colour: PieceColour::White,
                x: 5,
                y: 2,
            });

            // currently blocking bishop from taking the king
            let rook_id = world
                .spawn()
                .insert(Piece {
                    kind: PieceKind::Rook,
                    colour: PieceColour::Black,
                    x: 6,
                    y: 3,
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
                x: 7,
                y: 4,
            });

            // blocked by black bishop
            world.spawn().insert(Piece {
                kind: PieceKind::Bishop,
                colour: PieceColour::White,
                x: 3,
                y: 0,
            });

            let bishop_id = world
                .spawn()
                .insert(Piece {
                    kind: PieceKind::Bishop,
                    colour: PieceColour::Black,
                    x: 5,
                    y: 2,
                })
                .id();

            update_stage.run(&mut world);

            let all_valid_moves = world.get_resource::<AllValidMoves>().unwrap();
            assert_eq!(
                all_valid_moves.get(bishop_id),
                &vec![
                    Move::standard((6, 3)),
                    Move::standard((4, 1)),
                    Move::standard((3, 0)),
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
                    x: 7,
                    y: 4,
                })
                .id();

            // has the king in check
            world.spawn().insert(Piece {
                kind: PieceKind::Knight,
                colour: PieceColour::White,
                x: 5,
                y: 3,
            });

            // could move to take the knight, but would expose the king to the rook
            let pawn_id = world
                .spawn()
                .insert(Piece {
                    kind: PieceKind::Pawn,
                    colour: PieceColour::Black,
                    x: 6,
                    y: 4,
                })
                .id();

            // blocked by pawn
            world.spawn().insert(Piece {
                kind: PieceKind::Rook,
                colour: PieceColour::White,
                x: 5,
                y: 4,
            });

            update_stage.run(&mut world);

            let all_valid_moves = world.get_resource::<AllValidMoves>().unwrap();
            assert!(all_valid_moves.get(pawn_id).is_empty());
            assert_eq!(
                all_valid_moves.get(king_id),
                &vec![
                    Move::standard((6, 3)),
                    Move::standard((7, 3)),
                    Move::standard((7, 5))
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
                x: 7,
                y: 4,
            });

            let mut spawn_pawn = |x: u8, y: u8| {
                world.spawn().insert(Piece {
                    kind: PieceKind::Pawn,
                    colour: PieceColour::Black,
                    x,
                    y,
                });
            };

            spawn_pawn(7, 3);
            spawn_pawn(6, 3);
            spawn_pawn(7, 5);
            spawn_pawn(6, 5);

            world.spawn().insert(Piece {
                kind: PieceKind::Knight,
                colour: PieceColour::White,
                x: 5,
                y: 3,
            });

            world.spawn().insert(Piece {
                kind: PieceKind::Queen,
                colour: PieceColour::White,
                x: 5,
                y: 4,
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
                    x: 7,
                    y: 4,
                })
                .id();

            world.spawn().insert(Piece {
                kind: PieceKind::Pawn,
                colour: PieceColour::White,
                x: 5,
                y: 4,
            });

            update_stage.run(&mut world);

            let all_valid_moves = world.get_resource::<AllValidMoves>().unwrap();
            assert_eq!(
                all_valid_moves.get(king_id),
                &vec![
                    Move::standard((6, 4)),
                    Move::standard((7, 3)),
                    Move::standard((7, 5))
                ]
            );
        }

        #[test]
        fn the_king_should_be_able_to_move_to_squares_that_are_potentially_attackable_but_blocked()
        {
            let (mut world, mut update_stage) = setup();

            let king_id = world
                .spawn()
                .insert(Piece {
                    kind: PieceKind::King,
                    colour: PieceColour::Black,
                    x: 7,
                    y: 4,
                })
                .id();

            world.spawn().insert(Piece {
                kind: PieceKind::Bishop,
                colour: PieceColour::Black,
                x: 7,
                y: 5,
            });

            world.spawn().insert(Piece {
                kind: PieceKind::Pawn,
                colour: PieceColour::Black,
                x: 6,
                y: 5,
            });

            world.spawn().insert(Piece {
                kind: PieceKind::Pawn,
                colour: PieceColour::Black,
                x: 6,
                y: 4,
            });

            world.spawn().insert(Piece {
                kind: PieceKind::Pawn,
                colour: PieceColour::Black,
                x: 6,
                y: 3,
            });

            world.spawn().insert(Piece {
                kind: PieceKind::Knight,
                colour: PieceColour::White,
                x: 5,
                y: 3,
            });

            world.spawn().insert(Piece {
                kind: PieceKind::Queen,
                colour: PieceColour::White,
                x: 0,
                y: 3,
            });

            update_stage.run(&mut world);

            let all_valid_moves = world.get_resource::<AllValidMoves>().unwrap();
            assert_eq!(all_valid_moves.get(king_id), &vec![Move::standard((7, 3)),]);
        }

        // see bug screenshots 1
        #[test]
        fn fix_bug_1_incorrectly_restricted_move_calculations() {
            let (mut world, mut update_stage) = setup();

            world.spawn().insert(Piece::white(PieceKind::King, 0, 4));
            world.spawn().insert(Piece::white(PieceKind::Pawn, 2, 3));
            world.spawn().insert(Piece::white(PieceKind::Pawn, 1, 4));
            world.spawn().insert(Piece::white(PieceKind::Pawn, 1, 2));

            world.spawn().insert(Piece::black(PieceKind::King, 7, 4));
            world.spawn().insert(Piece::black(PieceKind::Queen, 6, 4));
            world.spawn().insert(Piece::black(PieceKind::Pawn, 5, 4));

            let queen_id = world
                .spawn()
                .insert(Piece::white(PieceKind::Queen, 0, 3))
                .id();

            world.insert_resource(PlayerTurn(PieceColour::White));
            update_stage.run(&mut world);

            let all_valid_moves = world.get_resource::<AllValidMoves>().unwrap();
            assert_eq!(
                all_valid_moves.get(queen_id),
                &vec![
                    Move::standard((1, 3)),
                    Move::standard((0, 2)),
                    Move::standard((0, 1)),
                    Move::standard((0, 0)),
                ]
            );
        }
    }

    mod special_moves {
        use super::*;
        use crate::board::{
            calculate_all_moves, move_piece, AllValidMoves, CastlingData, GameState,
            LastPawnDoubleStep, MovePiece, PlayerTurn, SelectedPiece, SelectedSquare,
            SpecialMoveData, Square, Taken,
        };
        use crate::moves_calculator::Move;
        use bevy::ecs::component::Component;
        use bevy::prelude::*;

        trait WorldTestUtils {
            fn overwrite_resource<T: Component>(&mut self, resource: T);
            fn check_and_overwrite_state(
                &mut self,
                expected_state: GameState,
                new_state: GameState,
            );
            fn square(&mut self, x: u8, y: u8) -> Entity;
            fn move_piece(&mut self, piece_id: Entity, x: u8, y: u8);
        }

        impl WorldTestUtils for World {
            fn overwrite_resource<T: Component>(&mut self, resource: T) {
                *self.get_resource_mut::<T>().unwrap() = resource;
            }

            fn check_and_overwrite_state(
                &mut self,
                expected_state: GameState,
                new_state: GameState,
            ) {
                let mut state = self.get_resource_mut::<State<GameState>>().unwrap();
                assert_eq!(state.current(), &expected_state);
                state.overwrite_set(new_state).unwrap();
            }

            fn square(&mut self, x: u8, y: u8) -> Entity {
                self.query::<(Entity, &Square)>()
                    .iter(self)
                    .find_map(|(entity, square)| (square.x == x && square.y == y).then(|| entity))
                    .unwrap()
            }

            fn move_piece(&mut self, piece_id: Entity, x: u8, y: u8) {
                let all_valid_moves = self.get_resource::<AllValidMoves>().unwrap();
                let piece_moves = all_valid_moves.get(piece_id);
                assert!(
                    all_valid_moves.contains(piece_id, x, y),
                    "({}, {}) is not a valid move; valid moves: {:?}",
                    x,
                    y,
                    piece_moves
                );

                let piece = self.get::<Piece>(piece_id).unwrap();
                let turn = self.get_resource::<PlayerTurn>().unwrap();
                assert_eq!(
                    piece.colour, turn.0,
                    "Moving {:?} piece on {:?}'s turn",
                    piece.colour, turn.0
                );

                self.check_and_overwrite_state(
                    GameState::NothingSelected,
                    GameState::TargetSquareSelected,
                );
                self.overwrite_resource(SelectedPiece(Some(piece_id)));
                let square = self.square(x, y);
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
            world.insert_resource(SpecialMoveData::default());

            (0..8).for_each(|x| {
                (0..8).for_each(|y| {
                    world.spawn().insert(Square { x, y });
                })
            });

            let mut update_stage = SystemStage::parallel();
            update_stage.add_system_set(State::<GameState>::get_driver());
            update_stage.add_system_set(
                SystemSet::on_update(GameState::NothingSelected)
                    .with_system(calculate_all_moves.system()),
            );
            update_stage.add_system_set(
                SystemSet::on_update(GameState::TargetSquareSelected)
                    .with_system(move_piece.system()),
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
                piece.x = move_piece.target_x as u8;
                piece.y = move_piece.target_y as u8;

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
        fn when_a_pawn_makes_a_two_step_move_an_adjacent_pawn_can_take_it_en_passant_on_the_next_turn(
        ) {
            let (mut world, mut stage) = setup();

            world.spawn().insert(Piece {
                kind: PieceKind::King,
                colour: PieceColour::Black,
                x: 7,
                y: 4,
            });

            world.spawn().insert(Piece {
                kind: PieceKind::King,
                colour: PieceColour::White,
                x: 0,
                y: 4,
            });

            let black_pawn = world
                .spawn()
                .insert(Piece {
                    kind: PieceKind::Pawn,
                    colour: PieceColour::Black,
                    x: 6,
                    y: 4,
                })
                .id();

            let white_pawn = world
                .spawn()
                .insert(Piece {
                    kind: PieceKind::Pawn,
                    colour: PieceColour::White,
                    x: 4,
                    y: 3,
                })
                .id();

            let mut special_moves = world.get_resource_mut::<SpecialMoveData>().unwrap();
            special_moves.black_castling_data.king_moved = true;
            special_moves.white_castling_data.king_moved = true;

            stage.run(&mut world);

            world.move_piece(black_pawn, 4, 4);
            stage.run(&mut world);

            let special_moves = world.get_resource::<SpecialMoveData>().unwrap();
            assert_eq!(
                &special_moves.last_pawn_double_step,
                &Some(LastPawnDoubleStep {
                    pawn_id: black_pawn,
                    x: 4,
                    y: 4,
                })
            );

            assert_eq!(
                world.get_resource::<State<GameState>>().unwrap().current(),
                &GameState::NothingSelected
            );

            stage.run(&mut world);

            world.move_piece(white_pawn, 5, 4);
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
                    x: 7,
                    y: 4,
                })
                .id();

            let white_king = world
                .spawn()
                .insert(Piece {
                    kind: PieceKind::King,
                    colour: PieceColour::White,
                    x: 0,
                    y: 4,
                })
                .id();

            let black_pawn = world
                .spawn()
                .insert(Piece {
                    kind: PieceKind::Pawn,
                    colour: PieceColour::Black,
                    x: 6,
                    y: 4,
                })
                .id();

            let white_pawn = world
                .spawn()
                .insert(Piece {
                    kind: PieceKind::Pawn,
                    colour: PieceColour::White,
                    x: 4,
                    y: 3,
                })
                .id();

            let mut special_moves = world.get_resource_mut::<SpecialMoveData>().unwrap();
            special_moves.black_castling_data.king_moved = true;
            special_moves.white_castling_data.king_moved = true;

            stage.run(&mut world);

            // turn 0 move black pawn 2 steps forward
            world.move_piece(black_pawn, 4, 4);
            stage.run(&mut world);

            let special_moves = world.get_resource::<SpecialMoveData>().unwrap();
            assert_eq!(
                &special_moves.last_pawn_double_step,
                &Some(LastPawnDoubleStep {
                    pawn_id: black_pawn,
                    x: 4,
                    y: 4,
                })
            );

            stage.run(&mut world);
            let all_valid_moves = world.get_resource::<AllValidMoves>().unwrap();
            assert_eq!(
                all_valid_moves.get(white_pawn),
                &vec![Move::standard((5, 3)), Move::en_passant(5, 4, black_pawn)]
            );

            world.move_piece(white_king, 1, 4);
            stage.run(&mut world);

            world.move_piece(black_king, 6, 4);
            stage.run(&mut world);

            // check white pawn can't still move en passant
            let all_valid_moves = world.get_resource::<AllValidMoves>().unwrap();
            assert_eq!(
                all_valid_moves.get(white_pawn),
                &vec![Move::standard((5, 3))]
            );
        }

        #[test]
        fn it_should_be_possible_to_take_a_pawn_with_the_king_in_check_using_en_passant() {
            let (mut world, mut stage) = setup();

            world.spawn().insert(Piece {
                kind: PieceKind::King,
                colour: PieceColour::Black,
                x: 7,
                y: 4,
            });

            let white_king = world
                .spawn()
                .insert(Piece {
                    kind: PieceKind::King,
                    colour: PieceColour::White,
                    x: 3,
                    y: 3,
                })
                .id();

            let black_pawn = world
                .spawn()
                .insert(Piece {
                    kind: PieceKind::Pawn,
                    colour: PieceColour::Black,
                    x: 6,
                    y: 4,
                })
                .id();

            let white_pawn = world
                .spawn()
                .insert(Piece {
                    kind: PieceKind::Pawn,
                    colour: PieceColour::White,
                    x: 4,
                    y: 5,
                })
                .id();

            // prevent the white king from being able to move
            world.spawn().insert(Piece {
                kind: PieceKind::Rook,
                colour: PieceColour::Black,
                x: 0,
                y: 4,
            });
            world.spawn().insert(Piece {
                kind: PieceKind::Rook,
                colour: PieceColour::Black,
                x: 0,
                y: 2,
            });
            world.spawn().insert(Piece {
                kind: PieceKind::Rook,
                colour: PieceColour::Black,
                x: 4,
                y: 0,
            });
            world.spawn().insert(Piece {
                kind: PieceKind::Rook,
                colour: PieceColour::Black,
                x: 2,
                y: 0,
            });

            let mut special_moves = world.get_resource_mut::<SpecialMoveData>().unwrap();
            special_moves.black_castling_data.king_moved = true;
            special_moves.white_castling_data.king_moved = true;

            stage.run(&mut world);

            world.move_piece(black_pawn, 4, 4);
            stage.run(&mut world);

            let special_moves = world.get_resource::<SpecialMoveData>().unwrap();
            assert_eq!(
                &special_moves.last_pawn_double_step,
                &Some(LastPawnDoubleStep {
                    pawn_id: black_pawn,
                    x: 4,
                    y: 4,
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
                &vec![Move::en_passant(5, 4, black_pawn)]
            );
        }

        #[test]
        fn it_should_not_be_possible_to_take_a_pawn_en_passant_if_it_would_expose_the_king_to_check(
        ) {
            let (mut world, mut stage) = setup();

            world.spawn().insert(Piece {
                kind: PieceKind::King,
                colour: PieceColour::Black,
                x: 7,
                y: 4,
            });

            world.spawn().insert(Piece {
                kind: PieceKind::King,
                colour: PieceColour::White,
                x: 3,
                y: 3,
            });

            let black_pawn = world
                .spawn()
                .insert(Piece {
                    kind: PieceKind::Pawn,
                    colour: PieceColour::Black,
                    x: 6,
                    y: 4,
                })
                .id();

            world.spawn().insert(Piece {
                kind: PieceKind::Pawn,
                colour: PieceColour::White,
                x: 4,
                y: 3,
            });

            // prevent the white king from being able to move
            world.spawn().insert(Piece {
                kind: PieceKind::Rook,
                colour: PieceColour::Black,
                x: 0,
                y: 4,
            });
            world.spawn().insert(Piece {
                kind: PieceKind::Rook,
                colour: PieceColour::Black,
                x: 0,
                y: 2,
            });
            world.spawn().insert(Piece {
                kind: PieceKind::Rook,
                colour: PieceColour::Black,
                x: 4,
                y: 0,
            });
            world.spawn().insert(Piece {
                kind: PieceKind::Rook,
                colour: PieceColour::Black,
                x: 2,
                y: 0,
            });

            // prevents the pawn from moving
            world.spawn().insert(Piece {
                kind: PieceKind::Rook,
                colour: PieceColour::Black,
                x: 0,
                y: 3,
            });

            let mut special_moves = world.get_resource_mut::<SpecialMoveData>().unwrap();
            special_moves.black_castling_data.king_moved = true;
            special_moves.white_castling_data.king_moved = true;

            stage.run(&mut world);

            world.move_piece(black_pawn, 4, 4);
            stage.run(&mut world);

            let special_moves = world.get_resource::<SpecialMoveData>().unwrap();
            assert_eq!(
                &special_moves.last_pawn_double_step,
                &Some(LastPawnDoubleStep {
                    pawn_id: black_pawn,
                    x: 4,
                    y: 4,
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
                x: 7,
                y: 0,
            });

            world.spawn().insert(Piece {
                kind: PieceKind::King,
                colour: PieceColour::White,
                x: 4,
                y: 2,
            });

            let black_pawn = world
                .spawn()
                .insert(Piece {
                    kind: PieceKind::Pawn,
                    colour: PieceColour::Black,
                    x: 6,
                    y: 4,
                })
                .id();

            world.spawn().insert(Piece {
                kind: PieceKind::Bishop,
                colour: PieceColour::Black,
                x: 7,
                y: 5,
            });

            let white_pawn = world
                .spawn()
                .insert(Piece {
                    kind: PieceKind::Pawn,
                    colour: PieceColour::White,
                    x: 4,
                    y: 3,
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

            world.move_piece(black_pawn, 4, 4);
            stage.run(&mut world);

            let special_moves = world.get_resource::<SpecialMoveData>().unwrap();
            assert_eq!(
                &special_moves.last_pawn_double_step,
                &Some(LastPawnDoubleStep {
                    pawn_id: black_pawn,
                    x: 4,
                    y: 4,
                })
            );

            assert_eq!(
                world.get_resource::<State<GameState>>().unwrap().current(),
                &GameState::NothingSelected
            );

            let all_valid_moves = world.get_resource::<AllValidMoves>().unwrap();
            assert_eq!(
                all_valid_moves.get(white_pawn),
                &vec![Move::standard((5, 3))]
            );
        }

        #[test]
        fn it_should_be_possible_to_castle_queenside_if_neither_the_king_nor_the_rook_have_moved() {
            let (mut world, mut stage) = setup();

            let black_king = world
                .spawn()
                .insert(Piece {
                    kind: PieceKind::King,
                    colour: PieceColour::Black,
                    x: 7,
                    y: 4,
                })
                .id();

            world.spawn().insert(Piece {
                kind: PieceKind::King,
                colour: PieceColour::White,
                x: 0,
                y: 3,
            });

            let black_rook = world
                .spawn()
                .insert(Piece {
                    kind: PieceKind::Rook,
                    colour: PieceColour::Black,
                    x: 7,
                    y: 0,
                })
                .id();

            let mut special_moves = world.get_resource_mut::<SpecialMoveData>().unwrap();
            special_moves.black_castling_data.kingside_rook_moved = true;

            stage.run(&mut world);

            world.move_piece(black_king, 7, 0);
            stage.run(&mut world);

            let black_king = world.get::<Piece>(black_king).unwrap();
            assert_eq!(black_king.x, 7);
            assert_eq!(black_king.y, 2);

            let black_rook = world.get::<Piece>(black_rook).unwrap();
            assert_eq!(black_rook.x, 7);
            assert_eq!(black_rook.y, 3);
        }

        #[test]
        fn it_should_be_possible_to_castle_kingside_if_neither_the_king_nor_the_rook_have_moved() {
            let (mut world, mut stage) = setup();

            let white_king = world
                .spawn()
                .insert(Piece {
                    kind: PieceKind::King,
                    colour: PieceColour::White,
                    x: 0,
                    y: 4,
                })
                .id();

            world.spawn().insert(Piece {
                kind: PieceKind::King,
                colour: PieceColour::Black,
                x: 7,
                y: 4,
            });

            let white_rook = world
                .spawn()
                .insert(Piece {
                    kind: PieceKind::Rook,
                    colour: PieceColour::White,
                    x: 0,
                    y: 7,
                })
                .id();

            let mut special_moves = world.get_resource_mut::<SpecialMoveData>().unwrap();
            special_moves.white_castling_data.queenside_rook_moved = true;
            special_moves.black_castling_data.king_moved = true;

            world.overwrite_resource(PlayerTurn(PieceColour::White));

            stage.run(&mut world);

            world.move_piece(white_king, 0, 7);
            stage.run(&mut world);

            let white_king = world.get::<Piece>(white_king).unwrap();
            assert_eq!(white_king.x, 0);
            assert_eq!(white_king.y, 6);

            let white_rook = world.get::<Piece>(white_rook).unwrap();
            assert_eq!(white_rook.x, 0);
            assert_eq!(white_rook.y, 5);
        }

        #[test]
        fn it_should_not_be_possible_to_castle_if_the_king_has_moved() {
            let (mut world, mut stage) = setup();

            let white_king = world
                .spawn()
                .insert(Piece {
                    kind: PieceKind::King,
                    colour: PieceColour::White,
                    x: 0,
                    y: 4,
                })
                .id();

            let black_king = world
                .spawn()
                .insert(Piece {
                    kind: PieceKind::King,
                    colour: PieceColour::Black,
                    x: 7,
                    y: 4,
                })
                .id();

            let kingside_rook = world
                .spawn()
                .insert(Piece {
                    kind: PieceKind::Rook,
                    colour: PieceColour::White,
                    x: 0,
                    y: 7,
                })
                .id();

            let queenside_rook = world
                .spawn()
                .insert(Piece {
                    kind: PieceKind::Rook,
                    colour: PieceColour::White,
                    x: 0,
                    y: 0,
                })
                .id();

            world.overwrite_resource(PlayerTurn(PieceColour::White));
            let mut special_moves = world.get_resource_mut::<SpecialMoveData>().unwrap();
            special_moves.black_castling_data.queenside_rook_moved = true;
            special_moves.black_castling_data.kingside_rook_moved = true;

            stage.run(&mut world);

            let all_valid_moves = world.get_resource::<AllValidMoves>().unwrap();
            assert_eq!(
                all_valid_moves.get(white_king),
                &vec![
                    Move::standard((0, 3)),
                    Move::standard((0, 5)),
                    Move::standard((1, 3)),
                    Move::standard((1, 4)),
                    Move::standard((1, 5)),
                    Move::queenside_castle(0, 0, queenside_rook),
                    Move::kingside_castle(0, 7, kingside_rook)
                ]
            );

            world.move_piece(white_king, 0, 5);
            stage.run(&mut world);

            world.move_piece(black_king, 7, 5);
            stage.run(&mut world);

            let all_valid_moves = world.get_resource::<AllValidMoves>().unwrap();
            assert_eq!(
                all_valid_moves.get(white_king),
                &vec![
                    Move::standard((0, 4)),
                    Move::standard((0, 6)),
                    Move::standard((1, 4)),
                    Move::standard((1, 5)),
                    Move::standard((1, 6))
                ]
            );

            world.move_piece(white_king, 0, 4);
            stage.run(&mut world);

            world.move_piece(black_king, 7, 4);
            stage.run(&mut world);

            let all_valid_moves = world.get_resource::<AllValidMoves>().unwrap();
            assert_eq!(
                all_valid_moves.get(white_king),
                &vec![
                    Move::standard((0, 3)),
                    Move::standard((0, 5)),
                    Move::standard((1, 3)),
                    Move::standard((1, 4)),
                    Move::standard((1, 5))
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
                    x: 0,
                    y: 4,
                })
                .id();

            let black_king = world
                .spawn()
                .insert(Piece {
                    kind: PieceKind::King,
                    colour: PieceColour::Black,
                    x: 7,
                    y: 4,
                })
                .id();

            let white_kingside_rook = world
                .spawn()
                .insert(Piece {
                    kind: PieceKind::Rook,
                    colour: PieceColour::White,
                    x: 0,
                    y: 7,
                })
                .id();

            let queenside_rook = world
                .spawn()
                .insert(Piece {
                    kind: PieceKind::Rook,
                    colour: PieceColour::White,
                    x: 0,
                    y: 0,
                })
                .id();

            world.overwrite_resource(PlayerTurn(PieceColour::White));
            let mut special_moves = world.get_resource_mut::<SpecialMoveData>().unwrap();
            special_moves.black_castling_data.queenside_rook_moved = true;
            special_moves.black_castling_data.kingside_rook_moved = true;

            stage.run(&mut world);

            let all_valid_moves = world.get_resource::<AllValidMoves>().unwrap();
            assert_eq!(
                all_valid_moves.get(white_king),
                &vec![
                    Move::standard((0, 3)),
                    Move::standard((0, 5)),
                    Move::standard((1, 3)),
                    Move::standard((1, 4)),
                    Move::standard((1, 5)),
                    Move::queenside_castle(0, 0, queenside_rook),
                    Move::kingside_castle(0, 7, white_kingside_rook)
                ]
            );

            world.move_piece(white_kingside_rook, 1, 7);
            stage.run(&mut world);

            world.move_piece(black_king, 7, 5);
            stage.run(&mut world);

            let all_valid_moves = world.get_resource::<AllValidMoves>().unwrap();
            assert_eq!(
                all_valid_moves.get(white_king),
                &vec![
                    Move::standard((0, 3)),
                    Move::standard((0, 5)),
                    Move::standard((1, 3)),
                    Move::standard((1, 4)),
                    Move::standard((1, 5)),
                    Move::queenside_castle(0, 0, queenside_rook)
                ]
            );

            world.move_piece(white_kingside_rook, 0, 7);
            stage.run(&mut world);

            world.move_piece(black_king, 7, 4);
            stage.run(&mut world);

            let all_valid_moves = world.get_resource::<AllValidMoves>().unwrap();
            assert_eq!(
                all_valid_moves.get(white_king),
                &vec![
                    Move::standard((0, 3)),
                    Move::standard((0, 5)),
                    Move::standard((1, 3)),
                    Move::standard((1, 4)),
                    Move::standard((1, 5)),
                    Move::queenside_castle(0, 0, queenside_rook)
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
                    x: 0,
                    y: 4,
                })
                .id();

            let black_king = world
                .spawn()
                .insert(Piece {
                    kind: PieceKind::King,
                    colour: PieceColour::Black,
                    x: 7,
                    y: 4,
                })
                .id();

            let kingside_rook = world
                .spawn()
                .insert(Piece {
                    kind: PieceKind::Rook,
                    colour: PieceColour::White,
                    x: 0,
                    y: 7,
                })
                .id();

            let white_queenside_rook = world
                .spawn()
                .insert(Piece {
                    kind: PieceKind::Rook,
                    colour: PieceColour::White,
                    x: 0,
                    y: 0,
                })
                .id();

            world.overwrite_resource(PlayerTurn(PieceColour::White));
            let mut special_moves = world.get_resource_mut::<SpecialMoveData>().unwrap();
            special_moves.black_castling_data.queenside_rook_moved = true;
            special_moves.black_castling_data.kingside_rook_moved = true;

            stage.run(&mut world);

            let all_valid_moves = world.get_resource::<AllValidMoves>().unwrap();
            assert_eq!(
                all_valid_moves.get(white_king),
                &vec![
                    Move::standard((0, 3)),
                    Move::standard((0, 5)),
                    Move::standard((1, 3)),
                    Move::standard((1, 4)),
                    Move::standard((1, 5)),
                    Move::queenside_castle(0, 0, white_queenside_rook),
                    Move::kingside_castle(0, 7, kingside_rook)
                ]
            );

            world.move_piece(white_queenside_rook, 1, 0);
            stage.run(&mut world);

            world.move_piece(black_king, 7, 5);
            stage.run(&mut world);

            let all_valid_moves = world.get_resource::<AllValidMoves>().unwrap();
            assert_eq!(
                all_valid_moves.get(white_king),
                &vec![
                    Move::standard((0, 3)),
                    Move::standard((0, 5)),
                    Move::standard((1, 3)),
                    Move::standard((1, 4)),
                    Move::standard((1, 5)),
                    Move::kingside_castle(0, 7, kingside_rook)
                ]
            );

            world.move_piece(white_queenside_rook, 0, 0);
            stage.run(&mut world);

            world.move_piece(black_king, 7, 4);
            stage.run(&mut world);

            let all_valid_moves = world.get_resource::<AllValidMoves>().unwrap();
            assert_eq!(
                all_valid_moves.get(white_king),
                &vec![
                    Move::standard((0, 3)),
                    Move::standard((0, 5)),
                    Move::standard((1, 3)),
                    Move::standard((1, 4)),
                    Move::standard((1, 5)),
                    Move::kingside_castle(0, 7, kingside_rook)
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
                    x: 0,
                    y: 4,
                })
                .id();

            world
                .spawn()
                .insert(Piece {
                    kind: PieceKind::King,
                    colour: PieceColour::Black,
                    x: 7,
                    y: 4,
                })
                .id();

            world.spawn().insert(Piece {
                kind: PieceKind::Rook,
                colour: PieceColour::White,
                x: 0,
                y: 7,
            });

            world.spawn().insert(Piece {
                kind: PieceKind::Rook,
                colour: PieceColour::White,
                x: 0,
                y: 0,
            });

            world.spawn().insert(Piece {
                kind: PieceKind::Knight,
                colour: PieceColour::Black,
                x: 2,
                y: 5,
            });

            world.overwrite_resource(PlayerTurn(PieceColour::White));
            stage.run(&mut world);

            let all_valid_moves = world.get_resource::<AllValidMoves>().unwrap();
            assert_eq!(
                all_valid_moves.get(white_king),
                &vec![
                    Move::standard((0, 3)),
                    Move::standard((0, 5)),
                    Move::standard((1, 4)),
                    Move::standard((1, 5))
                ]
            );
        }

        #[test]
        fn it_should_not_be_possible_to_castle_if_any_square_the_king_would_move_through_is_attacked(
        ) {
            let (mut world, mut stage) = setup();

            let white_king = world
                .spawn()
                .insert(Piece {
                    kind: PieceKind::King,
                    colour: PieceColour::White,
                    x: 0,
                    y: 4,
                })
                .id();

            world
                .spawn()
                .insert(Piece {
                    kind: PieceKind::King,
                    colour: PieceColour::Black,
                    x: 7,
                    y: 4,
                })
                .id();

            let kingside_rook = world
                .spawn()
                .insert(Piece {
                    kind: PieceKind::Rook,
                    colour: PieceColour::White,
                    x: 0,
                    y: 7,
                })
                .id();

            world.spawn().insert(Piece {
                kind: PieceKind::Rook,
                colour: PieceColour::White,
                x: 0,
                y: 0,
            });

            world.spawn().insert(Piece {
                kind: PieceKind::Knight,
                colour: PieceColour::Black,
                x: 2,
                y: 2,
            });

            world.overwrite_resource(PlayerTurn(PieceColour::White));
            stage.run(&mut world);

            let all_valid_moves = world.get_resource::<AllValidMoves>().unwrap();
            assert_eq!(
                all_valid_moves.get(white_king),
                &vec![
                    Move::standard((0, 5)),
                    Move::standard((1, 3)),
                    Move::standard((1, 5)),
                    Move::kingside_castle(0, 7, kingside_rook)
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
                    x: 0,
                    y: 4,
                })
                .id();

            world
                .spawn()
                .insert(Piece {
                    kind: PieceKind::King,
                    colour: PieceColour::Black,
                    x: 7,
                    y: 4,
                })
                .id();

            let kingside_rook = world
                .spawn()
                .insert(Piece {
                    kind: PieceKind::Rook,
                    colour: PieceColour::White,
                    x: 0,
                    y: 7,
                })
                .id();

            world.spawn().insert(Piece {
                kind: PieceKind::Rook,
                colour: PieceColour::White,
                x: 0,
                y: 0,
            });

            let black_knight = world
                .spawn()
                .insert(Piece {
                    kind: PieceKind::Knight,
                    colour: PieceColour::Black,
                    x: 2,
                    y: 1,
                })
                .id();

            let mut special_moves = world.get_resource_mut::<SpecialMoveData>().unwrap();
            special_moves.black_castling_data.king_moved = true;

            stage.run(&mut world);

            world.move_piece(black_knight, 0, 0);
            stage.run(&mut world);

            let all_valid_moves = world.get_resource::<AllValidMoves>().unwrap();
            assert_eq!(
                all_valid_moves.get(white_king),
                &vec![
                    Move::standard((0, 3)),
                    Move::standard((0, 5)),
                    Move::standard((1, 3)),
                    Move::standard((1, 4)),
                    Move::standard((1, 5)),
                    Move::kingside_castle(0, 7, kingside_rook)
                ]
            );
        }
    }
}

mod piece_tests {
    use crate::moves_calculator::{Move, PotentialMove};
    use crate::pieces::*;

    fn single_move_path((x, y): (u8, u8), colour: PieceColour) -> PiecePath {
        PiecePath::single(PotentialMove::new(Move::standard((x, y)), None), colour)
    }

    fn unblocked_move((x, y): (u8, u8)) -> PotentialMove {
        PotentialMove::new(Move::standard((x, y)), None)
    }

    fn blocked_move((x, y): (u8, u8), by: PieceColour) -> PotentialMove {
        PotentialMove::new(Move::standard((x, y)), Some(by))
    }

    mod valid_moves_of_a_white_pawn {
        use super::*;

        fn pawn(x: u8, y: u8) -> Piece {
            Piece {
                colour: PieceColour::White,
                kind: PieceKind::Pawn,
                x,
                y,
            }
        }

        mod when_the_board_is_empty {
            use super::*;
            use crate::moves_calculator::Move;

            #[test]
            fn should_only_allow_single_move_forward_after_first_move() {
                let pawn = pawn(2, 0);
                let valid_moves = pawn.valid_moves(&[pawn].into());

                assert_eq!(valid_moves, vec![single_move_path((3, 0), pawn.colour)]);
                assert_eq!(
                    valid_moves[0].legal_path_vec(),
                    vec![Move::standard((3, 0))]
                );
            }

            #[test]
            fn should_allow_two_steps_forward_on_first_move() {
                let pawn = pawn(1, 0);
                let valid_moves = pawn.valid_moves(&[pawn].into());

                let move_ = Move::pawn_double_step(3, 0);
                assert_eq!(
                    valid_moves,
                    vec![
                        single_move_path((2, 0), pawn.colour),
                        PiecePath::single(PotentialMove::new(move_, None), pawn.colour)
                    ]
                );

                assert_eq!(
                    valid_moves[0].legal_path_vec(),
                    vec![Move::standard((2, 0))]
                );
                assert_eq!(valid_moves[1].legal_path_vec(), vec![move_]);
            }

            #[test]
            fn should_not_allow_movement_off_the_board() {
                let pawn = pawn(7, 0);
                let valid_moves = pawn.valid_moves(&[pawn].into());

                assert!(valid_moves.is_empty());
            }
        }

        #[test]
        fn should_allow_diagonal_movement_to_take_a_black_piece() {
            let pawn = pawn(2, 1);
            let pieces = [
                Piece {
                    colour: PieceColour::Black,
                    kind: PieceKind::Pawn,
                    x: 3,
                    y: 2,
                },
                Piece {
                    colour: PieceColour::Black,
                    kind: PieceKind::Pawn,
                    x: 3,
                    y: 0,
                },
                pawn,
            ];

            let valid_moves = pawn.valid_moves(&pieces.into());
            assert_eq!(
                valid_moves,
                vec![
                    single_move_path((3, 1), pawn.colour),
                    single_move_path((3, 0), pawn.colour),
                    single_move_path((3, 2), pawn.colour),
                ]
            );

            assert_eq!(
                valid_moves[0].legal_path_vec(),
                vec![Move::standard((3, 1))]
            );
            assert_eq!(
                valid_moves[1].legal_path_vec(),
                vec![Move::standard((3, 0))]
            );
            assert_eq!(
                valid_moves[2].legal_path_vec(),
                vec![Move::standard((3, 2))]
            );
        }

        #[test]
        fn should_not_allow_forward_movement_to_take_a_black_piece() {
            let pawn = pawn(2, 0);
            let pieces = [
                Piece {
                    colour: PieceColour::Black,
                    kind: PieceKind::Pawn,
                    x: 3,
                    y: 0,
                },
                pawn,
            ];

            let valid_moves = pawn.valid_moves(&pieces.into());
            assert_eq!(valid_moves, vec![]);
        }

        #[test]
        fn should_not_allow_movement_onto_a_piece_of_the_same_colour() {
            let pawn = pawn(2, 0);
            let pieces = [
                Piece {
                    colour: PieceColour::White,
                    kind: PieceKind::Pawn,
                    x: 3,
                    y: 0,
                },
                pawn,
            ];

            let valid_moves = pawn.valid_moves(&pieces.into());
            assert_eq!(valid_moves, vec![]);
        }

        #[test]
        fn should_not_allow_double_movement_if_either_square_is_occupied() {
            let pawn = pawn(1, 0);
            let pieces = [
                Piece {
                    colour: PieceColour::White,
                    kind: PieceKind::Pawn,
                    x: 3,
                    y: 0,
                },
                pawn,
            ];

            let valid_moves = pawn.valid_moves(&pieces.into());
            assert_eq!(valid_moves, vec![single_move_path((2, 0), pawn.colour)]);
            assert_eq!(
                valid_moves[0].legal_path_vec(),
                vec![Move::standard((2, 0))]
            );

            let pieces = [
                Piece {
                    colour: PieceColour::White,
                    kind: PieceKind::Pawn,
                    x: 2,
                    y: 0,
                },
                pawn,
            ];

            let valid_moves = pawn.valid_moves(&pieces.into());
            assert_eq!(valid_moves, vec![]);
        }
    }

    mod valid_moves_of_a_black_pawn {
        use super::*;

        fn pawn(x: u8, y: u8) -> Piece {
            Piece {
                colour: PieceColour::Black,
                kind: PieceKind::Pawn,
                x,
                y,
            }
        }

        mod when_the_board_is_empty {
            use super::*;
            use crate::moves_calculator::Move;

            #[test]
            fn should_only_allow_single_move_forward_after_first_move() {
                let pawn = pawn(5, 0);
                let valid_moves = pawn.valid_moves(&[pawn].into());

                assert_eq!(valid_moves, vec![single_move_path((4, 0), pawn.colour)]);
                assert_eq!(
                    valid_moves[0].legal_path_vec(),
                    vec![Move::standard((4, 0))]
                );
            }

            #[test]
            fn should_allow_two_steps_forward_on_first_move() {
                let pawn = pawn(6, 0);
                let valid_moves = pawn.valid_moves(&[pawn].into());

                assert_eq!(
                    valid_moves,
                    vec![
                        single_move_path((5, 0), pawn.colour),
                        PiecePath::single(
                            PotentialMove::new(Move::pawn_double_step(4, 0), None),
                            pawn.colour
                        )
                    ]
                );

                assert_eq!(
                    valid_moves[0].legal_path_vec(),
                    vec![Move::standard((5, 0))]
                );
                assert_eq!(
                    valid_moves[1].legal_path_vec(),
                    vec![Move::pawn_double_step(4, 0)]
                );
            }

            #[test]
            fn should_not_allow_movement_off_the_board() {
                let pawn = pawn(0, 0);
                let valid_moves = pawn.valid_moves(&[pawn].into());

                assert!(valid_moves.is_empty());
            }
        }

        #[test]
        fn should_allow_diagonal_movement_to_take_a_white_piece() {
            let pawn = pawn(5, 1);
            let pieces = [
                Piece {
                    colour: PieceColour::White,
                    kind: PieceKind::Pawn,
                    x: 4,
                    y: 2,
                },
                Piece {
                    colour: PieceColour::White,
                    kind: PieceKind::Pawn,
                    x: 4,
                    y: 0,
                },
                pawn,
            ];

            let valid_moves = pawn.valid_moves(&pieces.into());
            assert_eq!(
                valid_moves,
                vec![
                    single_move_path((4, 1), pawn.colour),
                    single_move_path((4, 0), pawn.colour),
                    single_move_path((4, 2), pawn.colour),
                ]
            );
            assert_eq!(
                valid_moves[0].legal_path_vec(),
                vec![Move::standard((4, 1))]
            );
            assert_eq!(
                valid_moves[1].legal_path_vec(),
                vec![Move::standard((4, 0))]
            );
            assert_eq!(
                valid_moves[2].legal_path_vec(),
                vec![Move::standard((4, 2))]
            );
        }

        #[test]
        fn should_not_allow_forward_movement_to_take_a_white_piece() {
            let pawn = pawn(5, 0);
            let pieces = [
                Piece {
                    colour: PieceColour::White,
                    kind: PieceKind::Pawn,
                    x: 4,
                    y: 0,
                },
                pawn,
            ];

            let valid_moves = pawn.valid_moves(&pieces.into());
            assert_eq!(valid_moves, vec![]);
        }

        #[test]
        fn should_not_allow_movement_onto_a_piece_of_the_same_colour() {
            let pawn = pawn(5, 0);
            let pieces = [
                Piece {
                    colour: PieceColour::Black,
                    kind: PieceKind::Pawn,
                    x: 4,
                    y: 0,
                },
                pawn,
            ];

            let valid_moves = pawn.valid_moves(&pieces.into());
            assert_eq!(valid_moves, vec![]);
        }

        #[test]
        fn should_not_allow_double_movement_if_either_square_is_occupied() {
            let pawn = pawn(6, 0);
            let pieces = [
                Piece {
                    colour: PieceColour::White,
                    kind: PieceKind::Pawn,
                    x: 4,
                    y: 0,
                },
                pawn,
            ];

            let valid_moves = pawn.valid_moves(&pieces.into());
            assert_eq!(valid_moves, vec![single_move_path((5, 0), pawn.colour)]);
            assert_eq!(
                valid_moves[0].legal_path_vec(),
                vec![Move::standard((5, 0))]
            );

            let pieces = [
                Piece {
                    colour: PieceColour::White,
                    kind: PieceKind::Pawn,
                    x: 5,
                    y: 0,
                },
                pawn,
            ];

            let valid_moves = pawn.valid_moves(&pieces.into());
            assert_eq!(valid_moves, vec![]);
        }
    }

    mod valid_moves_of_a_king {
        use super::*;

        fn king(x: u8, y: u8) -> Piece {
            Piece {
                colour: PieceColour::Black,
                kind: PieceKind::King,
                x,
                y,
            }
        }

        #[test]
        fn should_be_able_to_move_one_square_in_any_direction() {
            let king = king(1, 1);
            let valid_moves = king.valid_moves(&[king].into());
            assert_eq!(
                valid_moves,
                vec![
                    single_move_path((0, 0), king.colour),
                    single_move_path((0, 1), king.colour),
                    single_move_path((0, 2), king.colour),
                    single_move_path((1, 0), king.colour),
                    single_move_path((1, 2), king.colour),
                    single_move_path((2, 0), king.colour),
                    single_move_path((2, 1), king.colour),
                    single_move_path((2, 2), king.colour)
                ]
            );
            assert_eq!(
                valid_moves[0].legal_path_vec(),
                vec![Move::standard((0, 0))]
            );
            assert_eq!(
                valid_moves[1].legal_path_vec(),
                vec![Move::standard((0, 1))]
            );
            assert_eq!(
                valid_moves[2].legal_path_vec(),
                vec![Move::standard((0, 2))]
            );
            assert_eq!(
                valid_moves[3].legal_path_vec(),
                vec![Move::standard((1, 0))]
            );
            assert_eq!(
                valid_moves[4].legal_path_vec(),
                vec![Move::standard((1, 2))]
            );
            assert_eq!(
                valid_moves[5].legal_path_vec(),
                vec![Move::standard((2, 0))]
            );
            assert_eq!(
                valid_moves[6].legal_path_vec(),
                vec![Move::standard((2, 1))]
            );
            assert_eq!(
                valid_moves[7].legal_path_vec(),
                vec![Move::standard((2, 2))]
            );
        }

        #[test]
        fn should_not_be_able_to_move_off_the_board() {
            let king = king(0, 0);
            let valid_moves = king.valid_moves(&[king].into());
            assert_eq!(
                valid_moves,
                vec![
                    single_move_path((0, 1), king.colour),
                    single_move_path((1, 0), king.colour),
                    single_move_path((1, 1), king.colour)
                ]
            );
            assert_eq!(
                valid_moves[0].legal_path_vec(),
                vec![Move::standard((0, 1))]
            );
            assert_eq!(
                valid_moves[1].legal_path_vec(),
                vec![Move::standard((1, 0))]
            );
            assert_eq!(
                valid_moves[2].legal_path_vec(),
                vec![Move::standard((1, 1))]
            );
        }

        #[test]
        fn should_not_be_able_to_move_into_a_piece_of_the_same_colour() {
            let pawn = |x: u8, y: u8| Piece {
                colour: PieceColour::Black,
                kind: PieceKind::Pawn,
                x,
                y,
            };
            let king = king(1, 1);
            let pieces = [king, pawn(2, 1), pawn(2, 2), pawn(1, 2)];

            let valid_moves = king.valid_moves(&pieces.into());
            assert_eq!(
                valid_moves,
                vec![
                    single_move_path((0, 0), king.colour),
                    single_move_path((0, 1), king.colour),
                    single_move_path((0, 2), king.colour),
                    single_move_path((1, 0), king.colour),
                    PiecePath::single(blocked_move((1, 2), PieceColour::Black), king.colour),
                    single_move_path((2, 0), king.colour),
                    PiecePath::single(blocked_move((2, 1), PieceColour::Black), king.colour),
                    PiecePath::single(blocked_move((2, 2), PieceColour::Black), king.colour),
                ]
            );

            assert_eq!(
                valid_moves[0].legal_path_vec(),
                vec![Move::standard((0, 0))]
            );
            assert_eq!(
                valid_moves[1].legal_path_vec(),
                vec![Move::standard((0, 1))]
            );
            assert_eq!(
                valid_moves[2].legal_path_vec(),
                vec![Move::standard((0, 2))]
            );
            assert_eq!(
                valid_moves[3].legal_path_vec(),
                vec![Move::standard((1, 0))]
            );
            assert!(valid_moves[4].legal_path_vec().is_empty());
            assert_eq!(
                valid_moves[5].legal_path_vec(),
                vec![Move::standard((2, 0))]
            );
            assert!(valid_moves[6].legal_path_vec().is_empty());
            assert!(valid_moves[7].legal_path_vec().is_empty());
        }

        #[test]
        fn should_be_able_to_move_into_a_piece_of_the_opposite_colour() {
            let pawn = |x: u8, y: u8| Piece {
                colour: PieceColour::White,
                kind: PieceKind::Pawn,
                x,
                y,
            };
            let king = king(1, 1);
            let pieces = [king, pawn(2, 1), pawn(2, 2), pawn(1, 2)];

            let valid_moves = king.valid_moves(&pieces.into());
            assert_eq!(
                valid_moves,
                vec![
                    single_move_path((0, 0), king.colour),
                    single_move_path((0, 1), king.colour),
                    single_move_path((0, 2), king.colour),
                    single_move_path((1, 0), king.colour),
                    PiecePath::single(blocked_move((1, 2), PieceColour::White), king.colour),
                    single_move_path((2, 0), king.colour),
                    PiecePath::single(blocked_move((2, 1), PieceColour::White), king.colour),
                    PiecePath::single(blocked_move((2, 2), PieceColour::White), king.colour),
                ]
            );

            assert_eq!(
                valid_moves[0].legal_path_vec(),
                vec![Move::standard((0, 0))]
            );
            assert_eq!(
                valid_moves[1].legal_path_vec(),
                vec![Move::standard((0, 1))]
            );
            assert_eq!(
                valid_moves[2].legal_path_vec(),
                vec![Move::standard((0, 2))]
            );
            assert_eq!(
                valid_moves[3].legal_path_vec(),
                vec![Move::standard((1, 0))]
            );
            assert_eq!(
                valid_moves[4].legal_path_vec(),
                vec![Move::standard((1, 2))]
            );
            assert_eq!(
                valid_moves[5].legal_path_vec(),
                vec![Move::standard((2, 0))]
            );
            assert_eq!(
                valid_moves[6].legal_path_vec(),
                vec![Move::standard((2, 1))]
            );
            assert_eq!(
                valid_moves[7].legal_path_vec(),
                vec![Move::standard((2, 2))]
            );
        }
    }

    mod valid_moves_of_a_queen {
        use super::*;

        fn queen(x: u8, y: u8) -> Piece {
            Piece {
                colour: PieceColour::Black,
                kind: PieceKind::Queen,
                x,
                y,
            }
        }

        #[test]
        fn should_be_able_to_move_in_any_direction() {
            let queen = queen(1, 1);
            let valid_moves = queen.valid_moves(&[queen].into());
            assert_eq!(
                valid_moves,
                vec![
                    PiecePath::new(
                        vec![
                            unblocked_move((2, 1)),
                            unblocked_move((3, 1)),
                            unblocked_move((4, 1)),
                            unblocked_move((5, 1)),
                            unblocked_move((6, 1)),
                            unblocked_move((7, 1)),
                        ],
                        queen.colour
                    ),
                    single_move_path((0, 1), queen.colour),
                    single_move_path((1, 0), queen.colour),
                    PiecePath::new(
                        vec![
                            unblocked_move((1, 2)),
                            unblocked_move((1, 3)),
                            unblocked_move((1, 4)),
                            unblocked_move((1, 5)),
                            unblocked_move((1, 6)),
                            unblocked_move((1, 7))
                        ],
                        queen.colour
                    ),
                    single_move_path((2, 0), queen.colour),
                    PiecePath::new(
                        vec![
                            unblocked_move((2, 2)),
                            unblocked_move((3, 3)),
                            unblocked_move((4, 4)),
                            unblocked_move((5, 5)),
                            unblocked_move((6, 6)),
                            unblocked_move((7, 7)),
                        ],
                        queen.colour
                    ),
                    single_move_path((0, 0), queen.colour),
                    single_move_path((0, 2), queen.colour),
                ]
            );

            assert_eq!(
                valid_moves[0].legal_path_vec(),
                vec![
                    Move::standard((2, 1)),
                    Move::standard((3, 1)),
                    Move::standard((4, 1)),
                    Move::standard((5, 1)),
                    Move::standard((6, 1)),
                    Move::standard((7, 1)),
                ]
            );
            assert_eq!(
                valid_moves[1].legal_path_vec(),
                vec![Move::standard((0, 1))]
            );
            assert_eq!(
                valid_moves[2].legal_path_vec(),
                vec![Move::standard((1, 0))]
            );
            assert_eq!(
                valid_moves[3].legal_path_vec(),
                vec![
                    Move::standard((1, 2)),
                    Move::standard((1, 3)),
                    Move::standard((1, 4)),
                    Move::standard((1, 5)),
                    Move::standard((1, 6)),
                    Move::standard((1, 7)),
                ]
            );
            assert_eq!(
                valid_moves[4].legal_path_vec(),
                vec![Move::standard((2, 0))]
            );
            assert_eq!(
                valid_moves[5].legal_path_vec(),
                vec![
                    Move::standard((2, 2)),
                    Move::standard((3, 3)),
                    Move::standard((4, 4)),
                    Move::standard((5, 5)),
                    Move::standard((6, 6)),
                    Move::standard((7, 7)),
                ]
            );
            assert_eq!(
                valid_moves[6].legal_path_vec(),
                vec![Move::standard((0, 0))]
            );
            assert_eq!(
                valid_moves[7].legal_path_vec(),
                vec![Move::standard((0, 2))]
            );
        }

        #[test]
        fn should_not_be_able_to_move_into_a_piece_of_the_same_colour() {
            let pawn = |x: u8, y: u8| Piece {
                colour: PieceColour::Black,
                kind: PieceKind::Pawn,
                x,
                y,
            };
            let queen = queen(1, 1);
            let pieces = [queen, pawn(1, 2), pawn(5, 1), pawn(3, 3)];

            let valid_moves = queen.valid_moves(&pieces.into());
            assert_eq!(
                valid_moves,
                vec![
                    PiecePath::new(
                        vec![
                            unblocked_move((2, 1)),
                            unblocked_move((3, 1)),
                            unblocked_move((4, 1)),
                            blocked_move((5, 1), PieceColour::Black),
                            unblocked_move((6, 1)),
                            unblocked_move((7, 1)),
                        ],
                        queen.colour
                    ),
                    single_move_path((0, 1), queen.colour),
                    single_move_path((1, 0), queen.colour),
                    PiecePath::new(
                        vec![
                            blocked_move((1, 2), PieceColour::Black),
                            unblocked_move((1, 3)),
                            unblocked_move((1, 4)),
                            unblocked_move((1, 5)),
                            unblocked_move((1, 6)),
                            unblocked_move((1, 7))
                        ],
                        queen.colour
                    ),
                    single_move_path((2, 0), queen.colour),
                    PiecePath::new(
                        vec![
                            unblocked_move((2, 2)),
                            blocked_move((3, 3), PieceColour::Black),
                            unblocked_move((4, 4)),
                            unblocked_move((5, 5)),
                            unblocked_move((6, 6)),
                            unblocked_move((7, 7)),
                        ],
                        queen.colour
                    ),
                    single_move_path((0, 0), queen.colour),
                    single_move_path((0, 2), queen.colour),
                ]
            );

            assert_eq!(
                valid_moves[0].legal_path_vec(),
                vec![
                    Move::standard((2, 1)),
                    Move::standard((3, 1)),
                    Move::standard((4, 1)),
                ]
            );
            assert_eq!(
                valid_moves[1].legal_path_vec(),
                vec![Move::standard((0, 1))]
            );
            assert_eq!(
                valid_moves[2].legal_path_vec(),
                vec![Move::standard((1, 0))]
            );
            assert_eq!(valid_moves[3].legal_path_vec(), vec![]);
            assert_eq!(
                valid_moves[4].legal_path_vec(),
                vec![Move::standard((2, 0))]
            );
            assert_eq!(
                valid_moves[5].legal_path_vec(),
                vec![Move::standard((2, 2)),]
            );
            assert_eq!(
                valid_moves[6].legal_path_vec(),
                vec![Move::standard((0, 0))]
            );
            assert_eq!(
                valid_moves[7].legal_path_vec(),
                vec![Move::standard((0, 2))]
            );
        }

        #[test]
        fn should_not_be_able_to_move_past_a_piece_of_a_different_colour() {
            let pawn = |x: u8, y: u8| Piece {
                colour: PieceColour::White,
                kind: PieceKind::Pawn,
                x,
                y,
            };
            let queen = queen(1, 1);
            let pieces = [queen, pawn(1, 2), pawn(5, 1), pawn(3, 3)];

            let valid_moves = queen.valid_moves(&pieces.into());
            assert_eq!(
                valid_moves,
                vec![
                    PiecePath::new(
                        vec![
                            unblocked_move((2, 1)),
                            unblocked_move((3, 1)),
                            unblocked_move((4, 1)),
                            blocked_move((5, 1), PieceColour::White),
                            unblocked_move((6, 1)),
                            unblocked_move((7, 1)),
                        ],
                        queen.colour
                    ),
                    single_move_path((0, 1), queen.colour),
                    single_move_path((1, 0), queen.colour),
                    PiecePath::new(
                        vec![
                            blocked_move((1, 2), PieceColour::White),
                            unblocked_move((1, 3)),
                            unblocked_move((1, 4)),
                            unblocked_move((1, 5)),
                            unblocked_move((1, 6)),
                            unblocked_move((1, 7))
                        ],
                        queen.colour
                    ),
                    single_move_path((2, 0), queen.colour),
                    PiecePath::new(
                        vec![
                            unblocked_move((2, 2)),
                            blocked_move((3, 3), PieceColour::White),
                            unblocked_move((4, 4)),
                            unblocked_move((5, 5)),
                            unblocked_move((6, 6)),
                            unblocked_move((7, 7)),
                        ],
                        queen.colour
                    ),
                    single_move_path((0, 0), queen.colour),
                    single_move_path((0, 2), queen.colour),
                ]
            );

            assert_eq!(
                valid_moves[0].legal_path_vec(),
                vec![
                    Move::standard((2, 1)),
                    Move::standard((3, 1)),
                    Move::standard((4, 1)),
                    Move::standard((5, 1)),
                ]
            );
            assert_eq!(
                valid_moves[1].legal_path_vec(),
                vec![Move::standard((0, 1))]
            );
            assert_eq!(
                valid_moves[2].legal_path_vec(),
                vec![Move::standard((1, 0))]
            );
            assert_eq!(
                valid_moves[3].legal_path_vec(),
                vec![Move::standard((1, 2)),]
            );
            assert_eq!(
                valid_moves[4].legal_path_vec(),
                vec![Move::standard((2, 0))]
            );
            assert_eq!(
                valid_moves[5].legal_path_vec(),
                vec![Move::standard((2, 2)), Move::standard((3, 3)),]
            );
            assert_eq!(
                valid_moves[6].legal_path_vec(),
                vec![Move::standard((0, 0))]
            );
            assert_eq!(
                valid_moves[7].legal_path_vec(),
                vec![Move::standard((0, 2))]
            );
        }

        #[test]
        fn diagonal_movement_should_not_be_blocked_if_the_path_is_empty() {
            let pawn = |x: u8, y: u8| Piece {
                colour: PieceColour::Black,
                kind: PieceKind::Pawn,
                x,
                y,
            };
            let queen = queen(7, 3);
            let pieces = [
                queen,
                pawn(7, 2),
                pawn(7, 4),
                pawn(6, 3),
                pawn(6, 2),
                pawn(5, 4),
                pawn(6, 5),
                pawn(6, 6),
            ];
            let valid_moves = queen.valid_moves(&pieces.into());

            assert_eq!(
                valid_moves,
                vec![
                    PiecePath::new(
                        vec![
                            blocked_move((6, 3), PieceColour::Black),
                            unblocked_move((5, 3)),
                            unblocked_move((4, 3)),
                            unblocked_move((3, 3)),
                            unblocked_move((2, 3)),
                            unblocked_move((1, 3)),
                            unblocked_move((0, 3))
                        ],
                        queen.colour
                    ),
                    PiecePath::new(
                        vec![
                            blocked_move((7, 2), PieceColour::Black),
                            unblocked_move((7, 1)),
                            unblocked_move((7, 0))
                        ],
                        queen.colour
                    ),
                    PiecePath::new(
                        vec![
                            blocked_move((7, 4), PieceColour::Black),
                            unblocked_move((7, 5)),
                            unblocked_move((7, 6)),
                            unblocked_move((7, 7))
                        ],
                        queen.colour
                    ),
                    PiecePath::new(
                        vec![
                            blocked_move((6, 2), PieceColour::Black),
                            unblocked_move((5, 1)),
                            unblocked_move((4, 0))
                        ],
                        queen.colour
                    ),
                    PiecePath::new(
                        vec![
                            unblocked_move((6, 4)),
                            unblocked_move((5, 5)),
                            unblocked_move((4, 6)),
                            unblocked_move((3, 7))
                        ],
                        queen.colour
                    ),
                ]
            );

            assert_eq!(valid_moves[0].legal_path_vec(), vec![]);
            assert_eq!(valid_moves[1].legal_path_vec(), vec![]);
            assert_eq!(valid_moves[2].legal_path_vec(), vec![]);
            assert_eq!(valid_moves[3].legal_path_vec(), vec![]);
            assert_eq!(
                valid_moves[4].legal_path_vec(),
                vec![
                    Move::standard((6, 4)),
                    Move::standard((5, 5)),
                    Move::standard((4, 6)),
                    Move::standard((3, 7)),
                ]
            );
        }
    }

    mod valid_moves_of_a_bishop {
        use super::*;

        fn bishop(x: u8, y: u8) -> Piece {
            Piece {
                colour: PieceColour::Black,
                kind: PieceKind::Bishop,
                x,
                y,
            }
        }

        #[test]
        fn should_be_able_to_move_diagonally() {
            let bishop = bishop(1, 1);
            let valid_moves = bishop.valid_moves(&[bishop].into());
            assert_eq!(
                valid_moves,
                vec![
                    single_move_path((2, 0), bishop.colour),
                    PiecePath::new(
                        vec![
                            unblocked_move((2, 2)),
                            unblocked_move((3, 3)),
                            unblocked_move((4, 4)),
                            unblocked_move((5, 5)),
                            unblocked_move((6, 6)),
                            unblocked_move((7, 7)),
                        ],
                        bishop.colour
                    ),
                    single_move_path((0, 0), bishop.colour),
                    single_move_path((0, 2), bishop.colour),
                ]
            );

            assert_eq!(
                valid_moves[0].legal_path_vec(),
                vec![Move::standard((2, 0))]
            );
            assert_eq!(
                valid_moves[1].legal_path_vec(),
                vec![
                    Move::standard((2, 2)),
                    Move::standard((3, 3)),
                    Move::standard((4, 4)),
                    Move::standard((5, 5)),
                    Move::standard((6, 6)),
                    Move::standard((7, 7)),
                ]
            );
            assert_eq!(
                valid_moves[2].legal_path_vec(),
                vec![Move::standard((0, 0))]
            );
            assert_eq!(
                valid_moves[3].legal_path_vec(),
                vec![Move::standard((0, 2))]
            );
        }

        #[test]
        fn should_not_be_able_to_move_into_a_piece_of_the_same_colour() {
            let pawn = |x: u8, y: u8| Piece {
                colour: PieceColour::Black,
                kind: PieceKind::Pawn,
                x,
                y,
            };
            let bishop = bishop(1, 1);
            let pieces = [bishop, pawn(5, 5)];

            let valid_moves = bishop.valid_moves(&pieces.into());
            assert_eq!(
                valid_moves,
                vec![
                    single_move_path((2, 0), bishop.colour),
                    PiecePath::new(
                        vec![
                            unblocked_move((2, 2)),
                            unblocked_move((3, 3)),
                            unblocked_move((4, 4)),
                            blocked_move((5, 5), PieceColour::Black),
                            unblocked_move((6, 6)),
                            unblocked_move((7, 7)),
                        ],
                        bishop.colour
                    ),
                    single_move_path((0, 0), bishop.colour),
                    single_move_path((0, 2), bishop.colour),
                ]
            );

            assert_eq!(
                valid_moves[0].legal_path_vec(),
                vec![Move::standard((2, 0))]
            );
            assert_eq!(
                valid_moves[1].legal_path_vec(),
                vec![
                    Move::standard((2, 2)),
                    Move::standard((3, 3)),
                    Move::standard((4, 4)),
                ]
            );
            assert_eq!(
                valid_moves[2].legal_path_vec(),
                vec![Move::standard((0, 0))]
            );
            assert_eq!(
                valid_moves[3].legal_path_vec(),
                vec![Move::standard((0, 2))]
            );
        }

        #[test]
        fn should_not_be_able_to_move_past_a_piece_of_a_different_colour() {
            let pawn = |x: u8, y: u8| Piece {
                colour: PieceColour::White,
                kind: PieceKind::Pawn,
                x,
                y,
            };
            let bishop = bishop(1, 1);
            let pieces = [bishop, pawn(5, 5)];

            let valid_moves = bishop.valid_moves(&pieces.into());
            assert_eq!(
                valid_moves,
                vec![
                    single_move_path((2, 0), bishop.colour),
                    PiecePath::new(
                        vec![
                            unblocked_move((2, 2)),
                            unblocked_move((3, 3)),
                            unblocked_move((4, 4)),
                            blocked_move((5, 5), PieceColour::White),
                            unblocked_move((6, 6)),
                            unblocked_move((7, 7)),
                        ],
                        bishop.colour
                    ),
                    single_move_path((0, 0), bishop.colour),
                    single_move_path((0, 2), bishop.colour),
                ]
            );

            assert_eq!(
                valid_moves[0].legal_path_vec(),
                vec![Move::standard((2, 0))]
            );
            assert_eq!(
                valid_moves[1].legal_path_vec(),
                vec![
                    Move::standard((2, 2)),
                    Move::standard((3, 3)),
                    Move::standard((4, 4)),
                    Move::standard((5, 5)),
                ]
            );
            assert_eq!(
                valid_moves[2].legal_path_vec(),
                vec![Move::standard((0, 0))]
            );
            assert_eq!(
                valid_moves[3].legal_path_vec(),
                vec![Move::standard((0, 2))]
            );
        }
    }

    mod valid_moves_of_a_knight {
        use super::*;

        fn knight(x: u8, y: u8) -> Piece {
            Piece {
                colour: PieceColour::Black,
                kind: PieceKind::Knight,
                x,
                y,
            }
        }

        #[test]
        fn should_be_able_to_move_2_squares_in_one_direction_and_1_in_the_other() {
            let knight = knight(2, 2);
            let valid_moves = knight.valid_moves(&[knight].into());
            assert_eq!(
                valid_moves,
                vec![
                    single_move_path((0, 1), knight.colour),
                    single_move_path((0, 3), knight.colour),
                    single_move_path((4, 1), knight.colour),
                    single_move_path((4, 3), knight.colour),
                    single_move_path((1, 0), knight.colour),
                    single_move_path((1, 4), knight.colour),
                    single_move_path((3, 0), knight.colour),
                    single_move_path((3, 4), knight.colour),
                ]
            );

            assert_eq!(
                valid_moves[0].legal_path_vec(),
                vec![Move::standard((0, 1))]
            );
            assert_eq!(
                valid_moves[1].legal_path_vec(),
                vec![Move::standard((0, 3))]
            );
            assert_eq!(
                valid_moves[2].legal_path_vec(),
                vec![Move::standard((4, 1))]
            );
            assert_eq!(
                valid_moves[3].legal_path_vec(),
                vec![Move::standard((4, 3))]
            );
            assert_eq!(
                valid_moves[4].legal_path_vec(),
                vec![Move::standard((1, 0))]
            );
            assert_eq!(
                valid_moves[5].legal_path_vec(),
                vec![Move::standard((1, 4))]
            );
            assert_eq!(
                valid_moves[6].legal_path_vec(),
                vec![Move::standard((3, 0))]
            );
            assert_eq!(
                valid_moves[7].legal_path_vec(),
                vec![Move::standard((3, 4))]
            );
        }

        #[test]
        fn should_not_be_able_to_move_off_the_board() {
            let knight = knight(0, 0);
            let valid_moves = knight.valid_moves(&[knight].into());
            assert_eq!(
                valid_moves,
                vec![
                    single_move_path((2, 1), knight.colour),
                    single_move_path((1, 2), knight.colour),
                ]
            );

            assert_eq!(
                valid_moves[0].legal_path_vec(),
                vec![Move::standard((2, 1))]
            );
            assert_eq!(
                valid_moves[1].legal_path_vec(),
                vec![Move::standard((1, 2))]
            );
        }

        #[test]
        fn should_be_able_to_move_over_other_pieces() {
            let pawn = |x: u8, y: u8| Piece {
                colour: PieceColour::Black,
                kind: PieceKind::Pawn,
                x,
                y,
            };
            let knight = knight(2, 2);
            let pieces = [
                knight,
                pawn(1, 1),
                pawn(2, 1),
                pawn(3, 1),
                pawn(3, 2),
                pawn(3, 3),
                pawn(2, 3),
                pawn(1, 3),
                pawn(1, 2),
            ];

            let valid_moves = knight.valid_moves(&pieces.into());
            assert_eq!(
                valid_moves,
                vec![
                    single_move_path((0, 1), knight.colour),
                    single_move_path((0, 3), knight.colour),
                    single_move_path((4, 1), knight.colour),
                    single_move_path((4, 3), knight.colour),
                    single_move_path((1, 0), knight.colour),
                    single_move_path((1, 4), knight.colour),
                    single_move_path((3, 0), knight.colour),
                    single_move_path((3, 4), knight.colour),
                ]
            );

            assert_eq!(
                valid_moves[0].legal_path_vec(),
                vec![Move::standard((0, 1))]
            );
            assert_eq!(
                valid_moves[1].legal_path_vec(),
                vec![Move::standard((0, 3))]
            );
            assert_eq!(
                valid_moves[2].legal_path_vec(),
                vec![Move::standard((4, 1))]
            );
            assert_eq!(
                valid_moves[3].legal_path_vec(),
                vec![Move::standard((4, 3))]
            );
            assert_eq!(
                valid_moves[4].legal_path_vec(),
                vec![Move::standard((1, 0))]
            );
            assert_eq!(
                valid_moves[5].legal_path_vec(),
                vec![Move::standard((1, 4))]
            );
            assert_eq!(
                valid_moves[6].legal_path_vec(),
                vec![Move::standard((3, 0))]
            );
            assert_eq!(
                valid_moves[7].legal_path_vec(),
                vec![Move::standard((3, 4))]
            );
        }

        #[test]
        fn should_not_be_able_to_move_into_a_piece_of_the_same_colour() {
            let pawn = |x: u8, y: u8| Piece {
                colour: PieceColour::Black,
                kind: PieceKind::Pawn,
                x,
                y,
            };
            let knight = knight(2, 2);
            let pieces = [knight, pawn(0, 1), pawn(4, 1), pawn(3, 0)];

            let valid_moves = knight.valid_moves(&pieces.into());
            assert_eq!(
                valid_moves,
                vec![
                    PiecePath::single(blocked_move((0, 1), PieceColour::Black), knight.colour),
                    single_move_path((0, 3), knight.colour),
                    PiecePath::single(blocked_move((4, 1), PieceColour::Black), knight.colour),
                    single_move_path((4, 3), knight.colour),
                    single_move_path((1, 0), knight.colour),
                    single_move_path((1, 4), knight.colour),
                    PiecePath::single(blocked_move((3, 0), PieceColour::Black), knight.colour),
                    single_move_path((3, 4), knight.colour),
                ]
            );

            assert_eq!(valid_moves[0].legal_path_vec(), vec![]);
            assert_eq!(
                valid_moves[1].legal_path_vec(),
                vec![Move::standard((0, 3))]
            );
            assert_eq!(valid_moves[2].legal_path_vec(), vec![]);
            assert_eq!(
                valid_moves[3].legal_path_vec(),
                vec![Move::standard((4, 3))]
            );
            assert_eq!(
                valid_moves[4].legal_path_vec(),
                vec![Move::standard((1, 0))]
            );
            assert_eq!(
                valid_moves[5].legal_path_vec(),
                vec![Move::standard((1, 4))]
            );
            assert_eq!(valid_moves[6].legal_path_vec(), vec![]);
            assert_eq!(
                valid_moves[7].legal_path_vec(),
                vec![Move::standard((3, 4))]
            );
        }
    }

    mod valid_moves_of_a_rook {
        use super::*;

        fn rook(x: u8, y: u8) -> Piece {
            Piece {
                colour: PieceColour::Black,
                kind: PieceKind::Rook,
                x,
                y,
            }
        }

        #[test]
        fn should_be_able_to_move_vertically_and_horizontally() {
            let rook = rook(1, 1);
            let valid_moves = rook.valid_moves(&[rook].into());
            assert_eq!(
                valid_moves,
                vec![
                    single_move_path((0, 1), rook.colour),
                    PiecePath::new(
                        vec![
                            unblocked_move((2, 1)),
                            unblocked_move((3, 1)),
                            unblocked_move((4, 1)),
                            unblocked_move((5, 1)),
                            unblocked_move((6, 1)),
                            unblocked_move((7, 1)),
                        ],
                        rook.colour
                    ),
                    PiecePath::new(
                        vec![
                            unblocked_move((1, 2)),
                            unblocked_move((1, 3)),
                            unblocked_move((1, 4)),
                            unblocked_move((1, 5)),
                            unblocked_move((1, 6)),
                            unblocked_move((1, 7)),
                        ],
                        rook.colour
                    ),
                    single_move_path((1, 0), rook.colour),
                ]
            );

            assert_eq!(
                valid_moves[0].legal_path_vec(),
                vec![Move::standard((0, 1))]
            );
            assert_eq!(
                valid_moves[1].legal_path_vec(),
                vec![
                    Move::standard((2, 1)),
                    Move::standard((3, 1)),
                    Move::standard((4, 1)),
                    Move::standard((5, 1)),
                    Move::standard((6, 1)),
                    Move::standard((7, 1)),
                ]
            );
            assert_eq!(
                valid_moves[2].legal_path_vec(),
                vec![
                    Move::standard((1, 2)),
                    Move::standard((1, 3)),
                    Move::standard((1, 4)),
                    Move::standard((1, 5)),
                    Move::standard((1, 6)),
                    Move::standard((1, 7)),
                ]
            );
            assert_eq!(
                valid_moves[3].legal_path_vec(),
                vec![Move::standard((1, 0))]
            );
        }

        #[test]
        fn should_not_be_able_to_move_into_a_piece_of_the_same_colour() {
            let pawn = |x: u8, y: u8| Piece {
                colour: PieceColour::Black,
                kind: PieceKind::Pawn,
                x,
                y,
            };
            let rook = rook(4, 4);
            let pieces = [rook, pawn(3, 4), pawn(4, 2)];

            let valid_moves = rook.valid_moves(&pieces.into());
            assert_eq!(
                valid_moves,
                vec![
                    PiecePath::new(
                        vec![
                            blocked_move((3, 4), PieceColour::Black),
                            unblocked_move((2, 4)),
                            unblocked_move((1, 4)),
                            unblocked_move((0, 4)),
                        ],
                        rook.colour
                    ),
                    PiecePath::new(
                        vec![
                            unblocked_move((5, 4)),
                            unblocked_move((6, 4)),
                            unblocked_move((7, 4)),
                        ],
                        rook.colour
                    ),
                    PiecePath::new(
                        vec![
                            unblocked_move((4, 5)),
                            unblocked_move((4, 6)),
                            unblocked_move((4, 7)),
                        ],
                        rook.colour
                    ),
                    PiecePath::new(
                        vec![
                            unblocked_move((4, 3)),
                            blocked_move((4, 2), PieceColour::Black),
                            unblocked_move((4, 1)),
                            unblocked_move((4, 0)),
                        ],
                        rook.colour
                    ),
                ]
            );

            assert_eq!(valid_moves[0].legal_path_vec(), vec![]);
            assert_eq!(
                valid_moves[1].legal_path_vec(),
                vec![
                    Move::standard((5, 4)),
                    Move::standard((6, 4)),
                    Move::standard((7, 4)),
                ]
            );
            assert_eq!(
                valid_moves[2].legal_path_vec(),
                vec![
                    Move::standard((4, 5)),
                    Move::standard((4, 6)),
                    Move::standard((4, 7)),
                ]
            );
            assert_eq!(
                valid_moves[3].legal_path_vec(),
                vec![Move::standard((4, 3))]
            );
        }

        #[test]
        fn should_not_be_able_to_move_past_a_piece_of_a_different_colour() {
            let pawn = |x: u8, y: u8| Piece {
                colour: PieceColour::White,
                kind: PieceKind::Pawn,
                x,
                y,
            };
            let rook = rook(1, 1);
            let pieces = [rook, pawn(5, 1), pawn(1, 3)];

            let valid_moves = rook.valid_moves(&pieces.into());
            assert_eq!(
                valid_moves,
                vec![
                    single_move_path((0, 1), rook.colour),
                    PiecePath::new(
                        vec![
                            unblocked_move((2, 1)),
                            unblocked_move((3, 1)),
                            unblocked_move((4, 1)),
                            blocked_move((5, 1), PieceColour::White),
                            unblocked_move((6, 1)),
                            unblocked_move((7, 1)),
                        ],
                        rook.colour
                    ),
                    PiecePath::new(
                        vec![
                            unblocked_move((1, 2)),
                            blocked_move((1, 3), PieceColour::White),
                            unblocked_move((1, 4)),
                            unblocked_move((1, 5)),
                            unblocked_move((1, 6)),
                            unblocked_move((1, 7)),
                        ],
                        rook.colour
                    ),
                    single_move_path((1, 0), rook.colour),
                ]
            );

            assert_eq!(
                valid_moves[0].legal_path_vec(),
                vec![Move::standard((0, 1))]
            );
            assert_eq!(
                valid_moves[1].legal_path_vec(),
                vec![
                    Move::standard((2, 1)),
                    Move::standard((3, 1)),
                    Move::standard((4, 1)),
                    Move::standard((5, 1)),
                ]
            );
            assert_eq!(
                valid_moves[2].legal_path_vec(),
                vec![Move::standard((1, 2)), Move::standard((1, 3)),]
            );
            assert_eq!(
                valid_moves[3].legal_path_vec(),
                vec![Move::standard((1, 0))]
            );
        }
    }
}
