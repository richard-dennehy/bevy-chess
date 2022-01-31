use super::*;

fn single_move_path((x, y): (u8, u8), colour: PieceColour) -> PiecePath {
    PiecePath::single(
        PotentialMove::new(Move::standard((x, y).into()), None),
        colour,
    )
}

fn unblocked_move((x, y): (u8, u8)) -> PotentialMove {
    PotentialMove::new(Move::standard((x, y).into()), None)
}

fn blocked_move((x, y): (u8, u8), by: PieceColour) -> PotentialMove {
    PotentialMove::new(Move::standard((x, y).into()), Some(by))
}

mod valid_moves_of_a_white_pawn {
    use super::*;

    fn pawn(x: u8, y: u8) -> Piece {
        Piece {
            colour: PieceColour::White,
            kind: PieceKind::Pawn,
            square: Square::new(x, y),
        }
    }

    mod when_the_board_is_empty {
        use super::*;

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

    fn pawn(x: u8, y: u8) -> Piece {
        Piece {
            colour: PieceColour::Black,
            kind: PieceKind::Pawn,
            square: Square::new(x, y),
        }
    }

    mod when_the_board_is_empty {
        use super::*;

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
