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
}

mod piece_tests {
    use crate::pieces::*;

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

            #[test]
            fn should_only_allow_single_move_forward_after_first_move() {
                let pawn = pawn(2, 0);
                let valid_moves = pawn.valid_moves(&[pawn].into());

                assert_eq!(valid_moves, vec![(3, 0)]);
            }

            #[test]
            fn should_allow_two_steps_forward_on_first_move() {
                let pawn = pawn(1, 0);
                let valid_moves = pawn.valid_moves(&[pawn].into());

                assert_eq!(valid_moves, vec![(2, 0), (3, 0)]);
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
            assert_eq!(valid_moves, vec![(3, 1), (3, 2), (3, 0)]);
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
            assert_eq!(valid_moves, vec![(2, 0)]);

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

            #[test]
            fn should_only_allow_single_move_forward_after_first_move() {
                let pawn = pawn(5, 0);
                let valid_moves = pawn.valid_moves(&[pawn].into());

                assert_eq!(valid_moves, vec![(4, 0)]);
            }

            #[test]
            fn should_allow_two_steps_forward_on_first_move() {
                let pawn = pawn(6, 0);
                let valid_moves = pawn.valid_moves(&[pawn].into());

                assert_eq!(valid_moves, vec![(5, 0), (4, 0)]);
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
            assert_eq!(valid_moves, vec![(4, 1), (4, 2), (4, 0)]);
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
            assert_eq!(valid_moves, vec![(5, 0)]);

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
                    (0, 0),
                    (0, 1),
                    (0, 2),
                    (1, 0),
                    (1, 2),
                    (2, 0),
                    (2, 1),
                    (2, 2)
                ]
            );
        }

        #[test]
        fn should_not_be_able_to_move_off_the_board() {
            let king = king(0, 0);
            let valid_moves = king.valid_moves(&[king].into());
            assert_eq!(valid_moves, vec![(0, 1), (1, 0), (1, 1),]);
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
            assert_eq!(valid_moves, vec![(0, 0), (0, 1), (0, 2), (1, 0), (2, 0),]);
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
                    (0, 0),
                    (2, 0),
                    (2, 2),
                    (0, 2),
                    (3, 3),
                    (4, 4),
                    (5, 5),
                    (6, 6),
                    (7, 7),
                    (0, 1),
                    (2, 1),
                    (3, 1),
                    (4, 1),
                    (5, 1),
                    (6, 1),
                    (7, 1),
                    (1, 0),
                    (1, 2),
                    (1, 3),
                    (1, 4),
                    (1, 5),
                    (1, 6),
                    (1, 7)
                ]
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
                    (0, 0),
                    (2, 0),
                    (2, 2),
                    (0, 2),
                    (0, 1),
                    (2, 1),
                    (3, 1),
                    (4, 1),
                    (1, 0),
                ]
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
                    (0, 0),
                    (2, 0),
                    (2, 2),
                    (0, 2),
                    (3, 3),
                    (0, 1),
                    (2, 1),
                    (3, 1),
                    (4, 1),
                    (5, 1),
                    (1, 0),
                    (1, 2),
                ]
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
                    (6, 4), (5, 5), (4, 6), (3, 7),
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
                    (0, 0),
                    (2, 0),
                    (2, 2),
                    (0, 2),
                    (3, 3),
                    (4, 4),
                    (5, 5),
                    (6, 6),
                    (7, 7),
                ]
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
                vec![(0, 0), (2, 0), (2, 2), (0, 2), (3, 3), (4, 4),]
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
                vec![(0, 0), (2, 0), (2, 2), (0, 2), (3, 3), (4, 4), (5, 5),]
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
                    (0, 1),
                    (0, 3),
                    (4, 1),
                    (4, 3),
                    (1, 0),
                    (1, 4),
                    (3, 0),
                    (3, 4),
                ]
            );
        }

        #[test]
        fn should_not_be_able_to_move_off_the_board() {
            let knight = knight(0, 0);
            let valid_moves = knight.valid_moves(&[knight].into());
            assert_eq!(valid_moves, vec![(2, 1), (1, 2),]);
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
                    (0, 1),
                    (0, 3),
                    (4, 1),
                    (4, 3),
                    (1, 0),
                    (1, 4),
                    (3, 0),
                    (3, 4),
                ]
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
            assert_eq!(valid_moves, vec![(0, 3), (4, 3), (1, 0), (1, 4), (3, 4),]);
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
                    (0, 1),
                    (2, 1),
                    (3, 1),
                    (4, 1),
                    (5, 1),
                    (6, 1),
                    (7, 1),
                    (1, 0),
                    (1, 2),
                    (1, 3),
                    (1, 4),
                    (1, 5),
                    (1, 6),
                    (1, 7),
                ]
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
                vec![(5, 4), (6, 4), (7, 4), (4, 3), (4, 5), (4, 6), (4, 7),]
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
                    (0, 1),
                    (2, 1),
                    (3, 1),
                    (4, 1),
                    (5, 1),
                    (1, 0),
                    (1, 2),
                    (1, 3),
                ]
            );
        }
    }
}
