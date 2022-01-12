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
                        square: (1, idx).into(),
                        kind: PieceKind::Pawn,
                        colour,
                    })
                    .collect::<Vec<_>>()
            } else {
                (0..8)
                    .map(|idx| Piece {
                        square: (6, idx).into(),
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
                square: (0, idx as u8).into(),
                colour: PieceColour::White,
                kind: *kind,
            })
            .chain(front_row(PieceColour::White).into_iter())
            .chain(front_row(PieceColour::Black).into_iter())
            .chain(back_row.iter().enumerate().map(|(idx, kind)| Piece {
                square: (7, idx as u8).into(),
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
            SpecialMoveData, Square,
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

            assert_eq!(valid_moves.get(king_id), &vec![Move::standard((6, 4).into())]);
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

            // pieces blocking the king but tragically unable to take the knight
            let mut spawn_pawn = |x: u8, y: u8| {
                ids.push(
                    world
                        .spawn()
                        .insert(Piece {
                            kind: PieceKind::Pawn,
                            colour: PieceColour::Black,
                            square: Square::new(x, y),
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
                        square: (6, 4).into(),
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

            assert_eq!(all_valid_moves.get(pawn_id), &vec![Move::standard((5, 3).into())]);

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

            // also has king in check
            world.spawn().insert(Piece {
                kind: PieceKind::Knight,
                colour: PieceColour::White,
                square: (5, 5).into(),
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
            assert_eq!(all_valid_moves.get(king_id), &vec![Move::standard((6, 4).into())]);
        }

        #[test]
        fn should_not_allow_the_king_to_take_a_piece_if_it_would_leave_the_king_in_check() {
            let (mut world, mut update_stage) = setup();

            let black_king = world
                .spawn()
                .insert(Piece {
                    kind: PieceKind::King,
                    colour: PieceColour::Black,
                    square: (3, 3).into(),
                })
                .id();

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
                ids.push(
                    world
                        .spawn()
                        .insert(Piece {
                            kind: PieceKind::Pawn,
                            colour: PieceColour::Black,
                            square: Square::new(x, y),
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
                        square: (7, 3).into(),
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
        fn the_king_should_be_able_to_move_to_squares_that_are_potentially_attackable_but_blocked()
        {
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
            assert_eq!(all_valid_moves.get(king_id), &vec![Move::standard((7, 3).into()),]);
        }

        // see bug screenshots 1
        #[test]
        fn fix_bug_1_incorrectly_restricted_move_calculations() {
            let (mut world, mut update_stage) = setup();

            world.spawn().insert(Piece::white(PieceKind::King, Square::new(0, 4)));
            world.spawn().insert(Piece::white(PieceKind::Pawn, Square::new(2, 3)));
            world.spawn().insert(Piece::white(PieceKind::Pawn, Square::new(1, 4)));
            world.spawn().insert(Piece::white(PieceKind::Pawn, Square::new(1, 2)));

            world.spawn().insert(Piece::black(PieceKind::King, Square::new(7, 4)));
            world.spawn().insert(Piece::black(PieceKind::Queen, Square::new(6, 4)));
            world.spawn().insert(Piece::black(PieceKind::Pawn, Square::new(5, 4)));

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
            world.spawn().insert(Piece::white(PieceKind::Knight, Square::new(4, 4)));
            world.spawn().insert(Piece::black(PieceKind::Rook, Square::new(7, 4)));

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
            world.spawn().insert(Piece::black(PieceKind::Queen, Square::new(0, 1)));

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
    }

    mod special_moves {
        use super::*;
        use crate::board::{
            calculate_all_moves, move_piece, AllValidMoves, CastlingData, GameState,
            LastPawnDoubleStep, MovePiece, PlayerTurn, PromotedPawn, SelectedPiece, SelectedSquare,
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
            fn move_piece(&mut self, piece_id: Entity, square: Square);
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

                self.check_and_overwrite_state(
                    GameState::NothingSelected,
                    GameState::TargetSquareSelected,
                );
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
                    world.spawn().insert(Square {
                        rank: x,
                        file: y,
                    });
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
        fn when_a_pawn_makes_a_two_step_move_an_adjacent_pawn_can_take_it_en_passant_on_the_next_turn(
        ) {
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
                &vec![Move::standard((5, 3).into()), Move::en_passant((5, 4).into(), black_pawn)]
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
        fn it_should_not_be_possible_to_take_a_pawn_en_passant_if_it_would_expose_the_king_to_check(
        ) {
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

            world.spawn().insert(Piece::black(PieceKind::King, Square::new(7, 4)));
            world.spawn().insert(Piece::white(PieceKind::King, Square::new(0, 4)));

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
                &vec![Move::standard((2, 3).into()), Move::pawn_double_step((3, 3).into())]
            );
        }

        #[test]
        fn a_pawn_on_an_edge_of_the_board_double_stepping_should_not_cause_overflow_when_checking_for_en_passant(
        ) {
            let (mut world, mut stage) = setup();

            world.spawn().insert(Piece::black(PieceKind::King, Square::new(7, 4)));
            world.spawn().insert(Piece::white(PieceKind::King, Square::new(0, 4)));

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

            let kingside_rook = world
                .spawn()
                .insert(Piece {
                    kind: PieceKind::Rook,
                    colour: PieceColour::White,
                    square: (0, 7).into(),
                })
                .id();

            let queenside_rook = world
                .spawn()
                .insert(Piece {
                    kind: PieceKind::Rook,
                    colour: PieceColour::White,
                    square: (0, 0).into(),
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
                    Move::standard((0, 3).into()),
                    Move::standard((0, 5).into()),
                    Move::standard((1, 3).into()),
                    Move::standard((1, 4).into()),
                    Move::standard((1, 5).into()),
                    Move::queenside_castle((0, 0).into(), queenside_rook),
                    Move::kingside_castle((0, 7).into(), kingside_rook)
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

            let white_kingside_rook = world
                .spawn()
                .insert(Piece {
                    kind: PieceKind::Rook,
                    colour: PieceColour::White,
                    square: (0, 7).into(),
                })
                .id();

            let queenside_rook = world
                .spawn()
                .insert(Piece {
                    kind: PieceKind::Rook,
                    colour: PieceColour::White,
                    square: (0, 0).into(),
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
                    Move::standard((0, 3).into()),
                    Move::standard((0, 5).into()),
                    Move::standard((1, 3).into()),
                    Move::standard((1, 4).into()),
                    Move::standard((1, 5).into()),
                    Move::queenside_castle((0, 0).into(), queenside_rook),
                    Move::kingside_castle((0, 7).into(), white_kingside_rook)
                ]
            );

            world.move_piece(white_kingside_rook, (1, 7).into());
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
                    Move::queenside_castle((0, 0).into(), queenside_rook)
                ]
            );

            world.move_piece(white_kingside_rook, (0, 7).into());
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
                    Move::queenside_castle((0, 0).into(), queenside_rook)
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

            let kingside_rook = world
                .spawn()
                .insert(Piece {
                    kind: PieceKind::Rook,
                    colour: PieceColour::White,
                    square: (0, 7).into(),
                })
                .id();

            let white_queenside_rook = world
                .spawn()
                .insert(Piece {
                    kind: PieceKind::Rook,
                    colour: PieceColour::White,
                    square: (0, 0).into(),
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
                    Move::standard((0, 3).into()),
                    Move::standard((0, 5).into()),
                    Move::standard((1, 3).into()),
                    Move::standard((1, 4).into()),
                    Move::standard((1, 5).into()),
                    Move::queenside_castle((0, 0).into(), white_queenside_rook),
                    Move::kingside_castle((0, 7).into(), kingside_rook)
                ]
            );

            world.move_piece(white_queenside_rook, (1, 0).into());
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
                    Move::kingside_castle((0, 7).into(), kingside_rook)
                ]
            );

            world.move_piece(white_queenside_rook, (0, 0).into());
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
                    Move::kingside_castle((0, 7).into(), kingside_rook)
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
        fn it_should_not_be_possible_to_castle_if_any_square_the_king_would_move_through_is_attacked(
        ) {
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

            let kingside_rook = world
                .spawn()
                .insert(Piece {
                    kind: PieceKind::Rook,
                    colour: PieceColour::White,
                    square: (0, 7).into(),
                })
                .id();

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
                    Move::kingside_castle((0, 7).into(), kingside_rook)
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

            let kingside_rook = world
                .spawn()
                .insert(Piece {
                    kind: PieceKind::Rook,
                    colour: PieceColour::White,
                    square: (0, 7).into(),
                })
                .id();

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
                    Move::kingside_castle((0, 7).into(), kingside_rook)
                ]
            );
        }
    }
}

mod piece_tests {
    use crate::moves_calculator::{Move, PotentialMove};
    use crate::pieces::*;

    fn single_move_path((x, y): (u8, u8), colour: PieceColour) -> PiecePath {
        PiecePath::single(PotentialMove::new(Move::standard((x, y).into()), None), colour)
    }

    fn unblocked_move((x, y): (u8, u8)) -> PotentialMove {
        PotentialMove::new(Move::standard((x, y).into()), None)
    }

    fn blocked_move((x, y): (u8, u8), by: PieceColour) -> PotentialMove {
        PotentialMove::new(Move::standard((x, y).into()), Some(by))
    }

    mod valid_moves_of_a_white_pawn {
        use super::*;
        use crate::board::Square;

        fn pawn(x: u8, y: u8) -> Piece {
            Piece {
                colour: PieceColour::White,
                kind: PieceKind::Pawn,
                square: Square::new(x, y),
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
                    vec![Move::standard((3, 0).into())]
                );
            }

            #[test]
            fn should_allow_two_steps_forward_on_first_move() {
                let pawn = pawn(1, 0);
                let valid_moves = pawn.valid_moves(&[pawn].into());

                let move_ = Move::pawn_double_step((3, 0).into());
                assert_eq!(
                    valid_moves,
                    vec![
                        single_move_path((2, 0), pawn.colour),
                        PiecePath::single(PotentialMove::new(move_, None), pawn.colour)
                    ]
                );

                assert_eq!(
                    valid_moves[0].legal_path_vec(),
                    vec![Move::standard((2, 0).into())]
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
                    square: (3, 2).into(),
                },
                Piece {
                    colour: PieceColour::Black,
                    kind: PieceKind::Pawn,
                    square: (3, 0).into(),
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
                vec![Move::standard((3, 1).into())]
            );
            assert_eq!(
                valid_moves[1].legal_path_vec(),
                vec![Move::standard((3, 0).into())]
            );
            assert_eq!(
                valid_moves[2].legal_path_vec(),
                vec![Move::standard((3, 2).into())]
            );
        }

        #[test]
        fn should_not_allow_forward_movement_to_take_a_black_piece() {
            let pawn = pawn(2, 0);
            let pieces = [
                Piece {
                    colour: PieceColour::Black,
                    kind: PieceKind::Pawn,
                    square: (3, 0).into(),
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
                    square: (3, 0).into(),
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
                    square: (3, 0).into(),
                },
                pawn,
            ];

            let valid_moves = pawn.valid_moves(&pieces.into());
            assert_eq!(valid_moves, vec![single_move_path((2, 0), pawn.colour)]);
            assert_eq!(
                valid_moves[0].legal_path_vec(),
                vec![Move::standard((2, 0).into())]
            );

            let pieces = [
                Piece {
                    colour: PieceColour::White,
                    kind: PieceKind::Pawn,
                    square: (2, 0).into(),
                },
                pawn,
            ];

            let valid_moves = pawn.valid_moves(&pieces.into());
            assert_eq!(valid_moves, vec![]);
        }
    }

    mod valid_moves_of_a_black_pawn {
        use super::*;
        use crate::board::Square;

        fn pawn(x: u8, y: u8) -> Piece {
            Piece {
                colour: PieceColour::Black,
                kind: PieceKind::Pawn,
                square: Square::new(x, y),
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
                    vec![Move::standard((4, 0).into())]
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
                            PotentialMove::new(Move::pawn_double_step((4, 0).into()), None),
                            pawn.colour
                        )
                    ]
                );

                assert_eq!(
                    valid_moves[0].legal_path_vec(),
                    vec![Move::standard((5, 0).into())]
                );
                assert_eq!(
                    valid_moves[1].legal_path_vec(),
                    vec![Move::pawn_double_step((4, 0).into())]
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
                    square: (4, 2).into(),
                },
                Piece {
                    colour: PieceColour::White,
                    kind: PieceKind::Pawn,
                    square: (4, 0).into(),
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
                vec![Move::standard((4, 1).into())]
            );
            assert_eq!(
                valid_moves[1].legal_path_vec(),
                vec![Move::standard((4, 0).into())]
            );
            assert_eq!(
                valid_moves[2].legal_path_vec(),
                vec![Move::standard((4, 2).into())]
            );
        }

        #[test]
        fn should_not_allow_forward_movement_to_take_a_white_piece() {
            let pawn = pawn(5, 0);
            let pieces = [
                Piece {
                    colour: PieceColour::White,
                    kind: PieceKind::Pawn,
                    square: (4, 0).into(),
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
                    square: (4, 0).into(),
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
                    square: (4, 0).into(),
                },
                pawn,
            ];

            let valid_moves = pawn.valid_moves(&pieces.into());
            assert_eq!(valid_moves, vec![single_move_path((5, 0), pawn.colour)]);
            assert_eq!(
                valid_moves[0].legal_path_vec(),
                vec![Move::standard((5, 0).into())]
            );

            let pieces = [
                Piece {
                    colour: PieceColour::White,
                    kind: PieceKind::Pawn,
                    square: (5, 0).into(),
                },
                pawn,
            ];

            let valid_moves = pawn.valid_moves(&pieces.into());
            assert_eq!(valid_moves, vec![]);
        }
    }

    mod valid_moves_of_a_king {
        use super::*;
        use crate::board::Square;

        fn king(x: u8, y: u8) -> Piece {
            Piece {
                colour: PieceColour::Black,
                kind: PieceKind::King,
                square: Square::new(x, y),
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
                vec![Move::standard((0, 0).into())]
            );
            assert_eq!(
                valid_moves[1].legal_path_vec(),
                vec![Move::standard((0, 1).into())]
            );
            assert_eq!(
                valid_moves[2].legal_path_vec(),
                vec![Move::standard((0, 2).into())]
            );
            assert_eq!(
                valid_moves[3].legal_path_vec(),
                vec![Move::standard((1, 0).into())]
            );
            assert_eq!(
                valid_moves[4].legal_path_vec(),
                vec![Move::standard((1, 2).into())]
            );
            assert_eq!(
                valid_moves[5].legal_path_vec(),
                vec![Move::standard((2, 0).into())]
            );
            assert_eq!(
                valid_moves[6].legal_path_vec(),
                vec![Move::standard((2, 1).into())]
            );
            assert_eq!(
                valid_moves[7].legal_path_vec(),
                vec![Move::standard((2, 2).into())]
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
                vec![Move::standard((0, 1).into())]
            );
            assert_eq!(
                valid_moves[1].legal_path_vec(),
                vec![Move::standard((1, 0).into())]
            );
            assert_eq!(
                valid_moves[2].legal_path_vec(),
                vec![Move::standard((1, 1).into())]
            );
        }

        #[test]
        fn should_not_be_able_to_move_into_a_piece_of_the_same_colour() {
            let pawn = |x: u8, y: u8| Piece {
                colour: PieceColour::Black,
                kind: PieceKind::Pawn,
                square: Square::new(x, y),
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
                vec![Move::standard((0, 0).into())]
            );
            assert_eq!(
                valid_moves[1].legal_path_vec(),
                vec![Move::standard((0, 1).into())]
            );
            assert_eq!(
                valid_moves[2].legal_path_vec(),
                vec![Move::standard((0, 2).into())]
            );
            assert_eq!(
                valid_moves[3].legal_path_vec(),
                vec![Move::standard((1, 0).into())]
            );
            assert!(valid_moves[4].legal_path_vec().is_empty());
            assert_eq!(
                valid_moves[5].legal_path_vec(),
                vec![Move::standard((2, 0).into())]
            );
            assert!(valid_moves[6].legal_path_vec().is_empty());
            assert!(valid_moves[7].legal_path_vec().is_empty());
        }

        #[test]
        fn should_be_able_to_move_into_a_piece_of_the_opposite_colour() {
            let pawn = |x: u8, y: u8| Piece {
                colour: PieceColour::White,
                kind: PieceKind::Pawn,
                square: Square::new(x, y),
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
                vec![Move::standard((0, 0).into())]
            );
            assert_eq!(
                valid_moves[1].legal_path_vec(),
                vec![Move::standard((0, 1).into())]
            );
            assert_eq!(
                valid_moves[2].legal_path_vec(),
                vec![Move::standard((0, 2).into())]
            );
            assert_eq!(
                valid_moves[3].legal_path_vec(),
                vec![Move::standard((1, 0).into())]
            );
            assert_eq!(
                valid_moves[4].legal_path_vec(),
                vec![Move::standard((1, 2).into())]
            );
            assert_eq!(
                valid_moves[5].legal_path_vec(),
                vec![Move::standard((2, 0).into())]
            );
            assert_eq!(
                valid_moves[6].legal_path_vec(),
                vec![Move::standard((2, 1).into())]
            );
            assert_eq!(
                valid_moves[7].legal_path_vec(),
                vec![Move::standard((2, 2).into())]
            );
        }
    }

    mod valid_moves_of_a_queen {
        use super::*;
        use crate::board::Square;

        fn queen(x: u8, y: u8) -> Piece {
            Piece {
                colour: PieceColour::Black,
                kind: PieceKind::Queen,
                square: Square::new(x, y),
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
                    Move::standard((2, 1).into()),
                    Move::standard((3, 1).into()),
                    Move::standard((4, 1).into()),
                    Move::standard((5, 1).into()),
                    Move::standard((6, 1).into()),
                    Move::standard((7, 1).into()),
                ]
            );
            assert_eq!(
                valid_moves[1].legal_path_vec(),
                vec![Move::standard((0, 1).into())]
            );
            assert_eq!(
                valid_moves[2].legal_path_vec(),
                vec![Move::standard((1, 0).into())]
            );
            assert_eq!(
                valid_moves[3].legal_path_vec(),
                vec![
                    Move::standard((1, 2).into()),
                    Move::standard((1, 3).into()),
                    Move::standard((1, 4).into()),
                    Move::standard((1, 5).into()),
                    Move::standard((1, 6).into()),
                    Move::standard((1, 7).into()),
                ]
            );
            assert_eq!(
                valid_moves[4].legal_path_vec(),
                vec![Move::standard((2, 0).into())]
            );
            assert_eq!(
                valid_moves[5].legal_path_vec(),
                vec![
                    Move::standard((2, 2).into()),
                    Move::standard((3, 3).into()),
                    Move::standard((4, 4).into()),
                    Move::standard((5, 5).into()),
                    Move::standard((6, 6).into()),
                    Move::standard((7, 7).into()),
                ]
            );
            assert_eq!(
                valid_moves[6].legal_path_vec(),
                vec![Move::standard((0, 0).into())]
            );
            assert_eq!(
                valid_moves[7].legal_path_vec(),
                vec![Move::standard((0, 2).into())]
            );
        }

        #[test]
        fn should_not_be_able_to_move_into_a_piece_of_the_same_colour() {
            let pawn = |x: u8, y: u8| Piece {
                colour: PieceColour::Black,
                kind: PieceKind::Pawn,
                square: Square::new(x, y),
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
                    Move::standard((2, 1).into()),
                    Move::standard((3, 1).into()),
                    Move::standard((4, 1).into()),
                ]
            );
            assert_eq!(
                valid_moves[1].legal_path_vec(),
                vec![Move::standard((0, 1).into())]
            );
            assert_eq!(
                valid_moves[2].legal_path_vec(),
                vec![Move::standard((1, 0).into())]
            );
            assert_eq!(valid_moves[3].legal_path_vec(), vec![]);
            assert_eq!(
                valid_moves[4].legal_path_vec(),
                vec![Move::standard((2, 0).into())]
            );
            assert_eq!(
                valid_moves[5].legal_path_vec(),
                vec![Move::standard((2, 2).into()),]
            );
            assert_eq!(
                valid_moves[6].legal_path_vec(),
                vec![Move::standard((0, 0).into())]
            );
            assert_eq!(
                valid_moves[7].legal_path_vec(),
                vec![Move::standard((0, 2).into())]
            );
        }

        #[test]
        fn should_not_be_able_to_move_past_a_piece_of_a_different_colour() {
            let pawn = |x: u8, y: u8| Piece {
                colour: PieceColour::White,
                kind: PieceKind::Pawn,
                square: Square::new(x, y),
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
                    Move::standard((2, 1).into()),
                    Move::standard((3, 1).into()),
                    Move::standard((4, 1).into()),
                    Move::standard((5, 1).into()),
                ]
            );
            assert_eq!(
                valid_moves[1].legal_path_vec(),
                vec![Move::standard((0, 1).into())]
            );
            assert_eq!(
                valid_moves[2].legal_path_vec(),
                vec![Move::standard((1, 0).into())]
            );
            assert_eq!(
                valid_moves[3].legal_path_vec(),
                vec![Move::standard((1, 2).into()),]
            );
            assert_eq!(
                valid_moves[4].legal_path_vec(),
                vec![Move::standard((2, 0).into())]
            );
            assert_eq!(
                valid_moves[5].legal_path_vec(),
                vec![Move::standard((2, 2).into()), Move::standard((3, 3).into()),]
            );
            assert_eq!(
                valid_moves[6].legal_path_vec(),
                vec![Move::standard((0, 0).into())]
            );
            assert_eq!(
                valid_moves[7].legal_path_vec(),
                vec![Move::standard((0, 2).into())]
            );
        }

        #[test]
        fn diagonal_movement_should_not_be_blocked_if_the_path_is_empty() {
            let pawn = |x: u8, y: u8| Piece {
                colour: PieceColour::Black,
                kind: PieceKind::Pawn,
                square: Square::new(x, y),
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
                    Move::standard((6, 4).into()),
                    Move::standard((5, 5).into()),
                    Move::standard((4, 6).into()),
                    Move::standard((3, 7).into()),
                ]
            );
        }
    }

    mod valid_moves_of_a_bishop {
        use super::*;
        use crate::board::Square;

        fn bishop(x: u8, y: u8) -> Piece {
            Piece {
                colour: PieceColour::Black,
                kind: PieceKind::Bishop,
                square: Square::new(x, y),
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
                vec![Move::standard((2, 0).into())]
            );
            assert_eq!(
                valid_moves[1].legal_path_vec(),
                vec![
                    Move::standard((2, 2).into()),
                    Move::standard((3, 3).into()),
                    Move::standard((4, 4).into()),
                    Move::standard((5, 5).into()),
                    Move::standard((6, 6).into()),
                    Move::standard((7, 7).into()),
                ]
            );
            assert_eq!(
                valid_moves[2].legal_path_vec(),
                vec![Move::standard((0, 0).into())]
            );
            assert_eq!(
                valid_moves[3].legal_path_vec(),
                vec![Move::standard((0, 2).into())]
            );
        }

        #[test]
        fn should_not_be_able_to_move_into_a_piece_of_the_same_colour() {
            let pawn = |x: u8, y: u8| Piece {
                colour: PieceColour::Black,
                kind: PieceKind::Pawn,
                square: Square::new(x, y),
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
                vec![Move::standard((2, 0).into())]
            );
            assert_eq!(
                valid_moves[1].legal_path_vec(),
                vec![
                    Move::standard((2, 2).into()),
                    Move::standard((3, 3).into()),
                    Move::standard((4, 4).into()),
                ]
            );
            assert_eq!(
                valid_moves[2].legal_path_vec(),
                vec![Move::standard((0, 0).into())]
            );
            assert_eq!(
                valid_moves[3].legal_path_vec(),
                vec![Move::standard((0, 2).into())]
            );
        }

        #[test]
        fn should_not_be_able_to_move_past_a_piece_of_a_different_colour() {
            let pawn = |x: u8, y: u8| Piece {
                colour: PieceColour::White,
                kind: PieceKind::Pawn,
                square: Square::new(x, y),
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
                vec![Move::standard((2, 0).into())]
            );
            assert_eq!(
                valid_moves[1].legal_path_vec(),
                vec![
                    Move::standard((2, 2).into()),
                    Move::standard((3, 3).into()),
                    Move::standard((4, 4).into()),
                    Move::standard((5, 5).into()),
                ]
            );
            assert_eq!(
                valid_moves[2].legal_path_vec(),
                vec![Move::standard((0, 0).into())]
            );
            assert_eq!(
                valid_moves[3].legal_path_vec(),
                vec![Move::standard((0, 2).into())]
            );
        }
    }

    mod valid_moves_of_a_knight {
        use super::*;
        use crate::board::Square;

        fn knight(x: u8, y: u8) -> Piece {
            Piece {
                colour: PieceColour::Black,
                kind: PieceKind::Knight,
                square: Square::new(x, y),
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
                vec![Move::standard((0, 1).into())]
            );
            assert_eq!(
                valid_moves[1].legal_path_vec(),
                vec![Move::standard((0, 3).into())]
            );
            assert_eq!(
                valid_moves[2].legal_path_vec(),
                vec![Move::standard((4, 1).into())]
            );
            assert_eq!(
                valid_moves[3].legal_path_vec(),
                vec![Move::standard((4, 3).into())]
            );
            assert_eq!(
                valid_moves[4].legal_path_vec(),
                vec![Move::standard((1, 0).into())]
            );
            assert_eq!(
                valid_moves[5].legal_path_vec(),
                vec![Move::standard((1, 4).into())]
            );
            assert_eq!(
                valid_moves[6].legal_path_vec(),
                vec![Move::standard((3, 0).into())]
            );
            assert_eq!(
                valid_moves[7].legal_path_vec(),
                vec![Move::standard((3, 4).into())]
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
                vec![Move::standard((2, 1).into())]
            );
            assert_eq!(
                valid_moves[1].legal_path_vec(),
                vec![Move::standard((1, 2).into())]
            );
        }

        #[test]
        fn should_be_able_to_move_over_other_pieces() {
            let pawn = |x: u8, y: u8| Piece {
                colour: PieceColour::Black,
                kind: PieceKind::Pawn,
                square: Square::new(x, y),
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
                vec![Move::standard((0, 1).into())]
            );
            assert_eq!(
                valid_moves[1].legal_path_vec(),
                vec![Move::standard((0, 3).into())]
            );
            assert_eq!(
                valid_moves[2].legal_path_vec(),
                vec![Move::standard((4, 1).into())]
            );
            assert_eq!(
                valid_moves[3].legal_path_vec(),
                vec![Move::standard((4, 3).into())]
            );
            assert_eq!(
                valid_moves[4].legal_path_vec(),
                vec![Move::standard((1, 0).into())]
            );
            assert_eq!(
                valid_moves[5].legal_path_vec(),
                vec![Move::standard((1, 4).into())]
            );
            assert_eq!(
                valid_moves[6].legal_path_vec(),
                vec![Move::standard((3, 0).into())]
            );
            assert_eq!(
                valid_moves[7].legal_path_vec(),
                vec![Move::standard((3, 4).into())]
            );
        }

        #[test]
        fn should_not_be_able_to_move_into_a_piece_of_the_same_colour() {
            let pawn = |x: u8, y: u8| Piece {
                colour: PieceColour::Black,
                kind: PieceKind::Pawn,
                square: Square::new(x, y),
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
                vec![Move::standard((0, 3).into())]
            );
            assert_eq!(valid_moves[2].legal_path_vec(), vec![]);
            assert_eq!(
                valid_moves[3].legal_path_vec(),
                vec![Move::standard((4, 3).into())]
            );
            assert_eq!(
                valid_moves[4].legal_path_vec(),
                vec![Move::standard((1, 0).into())]
            );
            assert_eq!(
                valid_moves[5].legal_path_vec(),
                vec![Move::standard((1, 4).into())]
            );
            assert_eq!(valid_moves[6].legal_path_vec(), vec![]);
            assert_eq!(
                valid_moves[7].legal_path_vec(),
                vec![Move::standard((3, 4).into())]
            );
        }
    }

    mod valid_moves_of_a_rook {
        use super::*;
        use crate::board::Square;

        fn rook(x: u8, y: u8) -> Piece {
            Piece {
                colour: PieceColour::Black,
                kind: PieceKind::Rook,
                square: Square::new(x, y),
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
                vec![Move::standard((0, 1).into())]
            );
            assert_eq!(
                valid_moves[1].legal_path_vec(),
                vec![
                    Move::standard((2, 1).into()),
                    Move::standard((3, 1).into()),
                    Move::standard((4, 1).into()),
                    Move::standard((5, 1).into()),
                    Move::standard((6, 1).into()),
                    Move::standard((7, 1).into()),
                ]
            );
            assert_eq!(
                valid_moves[2].legal_path_vec(),
                vec![
                    Move::standard((1, 2).into()),
                    Move::standard((1, 3).into()),
                    Move::standard((1, 4).into()),
                    Move::standard((1, 5).into()),
                    Move::standard((1, 6).into()),
                    Move::standard((1, 7).into()),
                ]
            );
            assert_eq!(
                valid_moves[3].legal_path_vec(),
                vec![Move::standard((1, 0).into())]
            );
        }

        #[test]
        fn should_not_be_able_to_move_into_a_piece_of_the_same_colour() {
            let pawn = |x: u8, y: u8| Piece {
                colour: PieceColour::Black,
                kind: PieceKind::Pawn,
                square: Square::new(x, y),
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
                    Move::standard((5, 4).into()),
                    Move::standard((6, 4).into()),
                    Move::standard((7, 4).into()),
                ]
            );
            assert_eq!(
                valid_moves[2].legal_path_vec(),
                vec![
                    Move::standard((4, 5).into()),
                    Move::standard((4, 6).into()),
                    Move::standard((4, 7).into()),
                ]
            );
            assert_eq!(
                valid_moves[3].legal_path_vec(),
                vec![Move::standard((4, 3).into())]
            );
        }

        #[test]
        fn should_not_be_able_to_move_past_a_piece_of_a_different_colour() {
            let pawn = |x: u8, y: u8| Piece {
                colour: PieceColour::White,
                kind: PieceKind::Pawn,
                square: Square::new(x, y),
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
                vec![Move::standard((0, 1).into())]
            );
            assert_eq!(
                valid_moves[1].legal_path_vec(),
                vec![
                    Move::standard((2, 1).into()),
                    Move::standard((3, 1).into()),
                    Move::standard((4, 1).into()),
                    Move::standard((5, 1).into()),
                ]
            );
            assert_eq!(
                valid_moves[2].legal_path_vec(),
                vec![Move::standard((1, 2).into()), Move::standard((1, 3).into()),]
            );
            assert_eq!(
                valid_moves[3].legal_path_vec(),
                vec![Move::standard((1, 0).into())]
            );
        }
    }
}
