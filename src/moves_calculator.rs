use crate::model::{
    AllValidMoves, BoardState, Move, MoveKind, Piece, PieceColour, PieceKind, PiecePath,
    PotentialMove, SpecialMoveData, Square,
};
use bevy::prelude::Entity;
use bevy::utils::HashMap;

pub enum CalculatorResult {
    Stalemate,
    Checkmate,
    Ok(AllValidMoves),
}

pub fn calculate_valid_moves(
    turn: PieceColour,
    special_move_data: &SpecialMoveData,
    player_pieces: &[(Entity, &Piece)],
    opposite_pieces: &[(Entity, &Piece)],
    board_state: BoardState,
) -> CalculatorResult {
    let (king_entity, king) = player_pieces
        .iter()
        .find(|(_, piece)| piece.kind == PieceKind::King)
        .copied()
        .expect("there should always be two kings");

    let calculator = MoveCalculator {
        turn,
        special_move_data,
        player_pieces,
        opposite_pieces,
        king_entity,
        king_square: king.square,
        board_state,
    };

    calculator.calculate_valid_moves()
}

struct MoveCalculator<'game> {
    turn: PieceColour,
    special_move_data: &'game SpecialMoveData,
    player_pieces: &'game [(Entity, &'game Piece)],
    opposite_pieces: &'game [(Entity, &'game Piece)],
    king_entity: Entity,
    king_square: Square,
    board_state: BoardState,
}

type Moves = Vec<Move>;
type PieceMoves = (Entity, Moves);
type EnPassantMove = (Entity, PiecePath);

#[derive(Debug)]
struct AllPotentialMoves(HashMap<Entity, Vec<PiecePath>>);

impl AllPotentialMoves {
    fn new() -> Self {
        Self(HashMap::default())
    }

    fn get(&self, entity: Entity) -> &[PiecePath] {
        self.0
            .get(&entity)
            .expect("missing move calculation")
            .as_slice()
    }

    fn insert(&mut self, entity: Entity, potential_path: Vec<PiecePath>) {
        let _ = self.0.insert(entity, potential_path);
    }

    fn can_reach(&self, entity: Entity, square: Square) -> bool {
        self.potential_path_to(entity, square)
            .map(|path| path.obstructions().is_empty())
            .unwrap_or(false)
    }

    fn potential_path_to(&self, entity: Entity, square: Square) -> Option<PiecePath> {
        self.get(entity)
            .iter()
            .find_map(|path| path.truncate_to(square))
    }
}

impl<'game> MoveCalculator<'game> {
    fn calculate_valid_moves(self) -> CalculatorResult {
        let mut all_potential_moves = AllPotentialMoves::new();

        let (mut en_passant_left, mut en_passant_right) = self.find_en_passant_pieces();

        self.player_pieces
            .iter()
            .chain(self.opposite_pieces.iter())
            .copied()
            .for_each(|(entity, piece)| {
                let mut valid_moves = piece.valid_moves(&self.board_state);

                if let Some((left, _)) = &en_passant_left {
                    if entity == *left {
                        valid_moves.push(en_passant_left.take().unwrap().1);
                    }
                } else if let Some((right, _)) = &en_passant_right {
                    if entity == *right {
                        valid_moves.push(en_passant_right.take().unwrap().1);
                    }
                };

                all_potential_moves.insert(entity, valid_moves);
            });

        let pieces_attacking_king = self.pieces_attacking_king(&all_potential_moves);

        if !pieces_attacking_king.is_empty() {
            let counter_moves =
                self.calculate_check_counter_moves(pieces_attacking_king, &all_potential_moves);

            if counter_moves.iter().all(|(_, moves)| moves.is_empty()) {
                CalculatorResult::Checkmate
            } else {
                let mut all_moves = AllValidMoves::default();
                counter_moves.into_iter().for_each(|(entity, moves)| {
                    let _ = all_moves.insert(entity, moves);
                });

                CalculatorResult::Ok(all_moves)
            }
        } else {
            let safe_player_moves = self.calculate_safe_player_moves(&all_potential_moves);

            let mut safe_king_moves = self.calculate_safe_king_moves(&all_potential_moves);
            let mut castling_moves = self.calculate_castling_moves(&all_potential_moves);
            safe_king_moves.append(&mut castling_moves);

            if safe_player_moves.iter().all(|(_, moves)| moves.is_empty())
                && safe_king_moves.is_empty()
            {
                return CalculatorResult::Stalemate;
            }

            let mut all_moves = AllValidMoves::default();

            let _ = all_moves.insert(self.king_entity, safe_king_moves);
            safe_player_moves.into_iter().for_each(|(entity, moves)| {
                let _ = all_moves.insert(entity, moves);
            });

            CalculatorResult::Ok(all_moves)
        }
    }

    #[allow(clippy::manual_range_contains)]
    fn find_en_passant_pieces(&self) -> (Option<EnPassantMove>, Option<EnPassantMove>) {
        if let Some(pawn_double_step) = &self.special_move_data.last_pawn_double_step {
            let find_pawn_in_column = |offset: i8| {
                let expected_y = pawn_double_step.square.file as i8 + offset;
                if expected_y < 0 || expected_y > 7 {
                    return None;
                };

                self.player_pieces.iter().find_map(|(entity, piece)| {
                    let expected_square =
                        Square::new(pawn_double_step.square.rank, expected_y as u8);
                    (piece.kind == PieceKind::Pawn
                        && piece.square == expected_square
                        && piece.colour == self.turn)
                        .then(|| {
                            let direction = piece.colour.pawn_direction();
                            let ep_move = Move::en_passant(
                                (
                                    (piece.square.rank as i8 + direction) as u8,
                                    (piece.square.file as i8 - offset) as u8,
                                )
                                    .into(),
                                pawn_double_step.pawn_id,
                            );
                            // note: this move can't be blocked, because if there was a piece in the way,
                            // then the enemy pawn wouldn't have been able to double step over it
                            (
                                *entity,
                                PiecePath::single(PotentialMove::new(ep_move, None), piece.colour),
                            )
                        })
                })
            };

            (find_pawn_in_column(-1), find_pawn_in_column(1))
        } else {
            (None, None)
        }
    }

    fn calculate_safe_king_moves(&self, potential_moves: &AllPotentialMoves) -> Moves {
        potential_moves
            .get(self.king_entity)
            .iter()
            .flat_map(PiecePath::legal_path)
            .filter(|king_move| {
                let attacked = self.opposite_pieces.iter().any(|(entity, piece)| {
                    // check that taking the piece on the square doesn't put the king in check
                    if self.board_state.get(king_move.target_square).is_some() {
                        potential_moves.get(*entity).iter().any(|path| {
                            path.obstructions()
                                .first()
                                .map(|obstruction| obstruction.square == king_move.target_square)
                                .unwrap_or(false)
                        })
                    } else if piece.kind == PieceKind::Pawn {
                        // pawn behaviour is very different to other pieces, and it's easier to handle
                        // the interactions here than try to get PotentialMove/PiecePath to handle it properly
                        let will_attack_king = |move_: &Option<PotentialMove>| {
                            let Some(potential_move) = move_ else { return false };
                            potential_move.target_square == king_move.target_square
                        };
                        let pawn_moves = piece.pawn_moves(&self.board_state, true);

                        will_attack_king(&pawn_moves.attack_left)
                            || will_attack_king(&pawn_moves.attack_right)
                    } else {
                        // check that the square isn't directly attacked, or that the king isn't currently blocking that square from being attacked
                        let Some(path) = potential_moves.potential_path_to(*entity, king_move.target_square) else { return false };
                        path.obstructions().is_empty() || (path.obstructions().len() == 1 && path.obstructions()[0].square == self.king_square)
                    }
                });

                !attacked
            })
            .collect()
    }

    fn pieces_attacking_king(
        &self,
        potential_moves: &AllPotentialMoves,
    ) -> Vec<(Entity, &Piece, Moves)> {
        self.opposite_pieces
            .iter()
            .filter_map(|(entity, piece)| {
                let legal_path = potential_moves
                    .potential_path_to(
                        *entity,
                        (self.king_square.rank, self.king_square.file).into(),
                    )?
                    .legal_path_vec();

                legal_path
                    .contains(&Move::standard(self.king_square))
                    .then(|| (*entity, *piece, legal_path))
            })
            .collect::<Vec<_>>()
    }

    fn calculate_safe_player_moves(&self, potential_moves: &AllPotentialMoves) -> Vec<PieceMoves> {
        let potential_threats = self.calculate_potential_threats_to_king(potential_moves);

        self.player_pieces
            .iter()
            .filter(|(entity, _)| *entity != self.king_entity)
            .map(|(entity, piece)| {
                let safe_moves = potential_moves
                    .get(*entity)
                    .iter()
                    .flat_map(PiecePath::legal_path)
                    .filter(|piece_move| {
                        // safe move iff: doesn't open up a path to the king, or stays within the same path, or takes the piece
                        potential_threats.iter().all(|(threat, path_to_king)| {
                            // note: at this point, can assume that the path has exactly one obstruction,
                            // and if this piece is in the path, it is the obstruction
                            let currently_in_path = path_to_king.contains(piece.square);
                            let stays_in_path = path_to_king.contains(piece_move.target_square);
                            let captures_threat = piece_move.target_square == threat.square;

                            captures_threat || !currently_in_path || stays_in_path
                        })
                    })
                    .collect::<Vec<_>>();
                (*entity, safe_moves)
            })
            .collect()
    }

    fn calculate_potential_threats_to_king(
        &self,
        potential_moves: &AllPotentialMoves,
    ) -> Vec<(&'game Piece, PiecePath)> {
        self.opposite_pieces
            .iter()
            .filter_map(|(entity, piece)| {
                let path = potential_moves.potential_path_to(
                    *entity,
                    (self.king_square.rank, self.king_square.file).into(),
                )?;

                let obstructions = path
                    .obstructions()
                    .into_iter()
                    .filter(|obs| obs.square != self.king_square)
                    .collect::<Vec<_>>();
                // if the path is blocked by 2+ pieces _excluding the king_, or by a piece of the same colour, it can't put the king in check during this turn
                let blocked = obstructions.len() >= 2
                    || obstructions
                        .into_iter()
                        .any(|obs| obs.colour == self.turn.opposite());

                (!blocked).then(|| (*piece, path))
            })
            .collect()
    }

    fn calculate_check_counter_moves(
        &self,
        pieces_attacking_king: Vec<(Entity, &Piece, Moves)>,
        potential_moves: &AllPotentialMoves,
    ) -> Vec<PieceMoves> {
        let safe_king_moves = self.calculate_safe_king_moves(potential_moves);
        let safe_moves = self.calculate_safe_player_moves(potential_moves);

        std::iter::once((self.king_entity, safe_king_moves))
            .chain(safe_moves.iter().map(|(entity, safe_piece_moves)| {
                // this piece can only move if it can take or block the piece that has the king in check
                let counter_moves = safe_piece_moves
                    .iter()
                    .filter(|piece_move| {
                        pieces_attacking_king.iter().all(
                            |(opposite_entity, opposite_piece, path_to_king)| {
                                let can_take_en_passant =
                                    if let MoveKind::EnPassant { target_id } = piece_move.kind {
                                        target_id == *opposite_entity
                                    } else {
                                        false
                                    };

                                let can_take_directly =
                                    opposite_piece.square == piece_move.target_square;

                                let blocks_piece = path_to_king
                                    .contains(&Move::standard(piece_move.target_square));

                                can_take_en_passant || can_take_directly || blocks_piece
                            },
                        )
                    })
                    .copied()
                    .collect::<Vec<_>>();

                (*entity, counter_moves)
            }))
            .collect()
    }

    fn calculate_castling_moves(&self, potential_moves: &AllPotentialMoves) -> Moves {
        let king_does_not_pass_through_attacked_square = |dir: i8| {
            let first_move = Square::new(
                self.king_square.rank,
                ((self.king_square.file as i8) + dir) as u8,
            );
            let second_move = Square::new(
                self.king_square.rank,
                ((self.king_square.file as i8) + (dir * 2)) as u8,
            );

            self.board_state.get(first_move).is_none()
                && self.board_state.get(second_move).is_none()
                && self.opposite_pieces.iter().all(|(entity, _)| {
                    !(potential_moves.can_reach(*entity, first_move)
                        || potential_moves.can_reach(*entity, second_move))
                })
        };

        let mut moves = vec![];
        let castling_data = self.special_move_data.castling_data(self.turn);

        if !castling_data.king_moved {
            if !castling_data.queenside_rook_moved {
                let passed_through = Square::new(self.king_square.rank, self.king_square.file - 3);

                if king_does_not_pass_through_attacked_square(-1)
                    && self.board_state.get(passed_through).is_none()
                {
                    let (rook_id, rook) = self
                        .player_pieces
                        .iter()
                        .find(|(_, piece)| {
                            piece.square.rank == self.king_square.rank && piece.square.file == 0
                        })
                        .expect("queenside castling without a rook");
                    moves.push(Move::queenside_castle(
                        (self.king_square.rank, 0).into(),
                        *rook_id,
                        **rook,
                    ));
                }
            }

            if !castling_data.kingside_rook_moved && king_does_not_pass_through_attacked_square(1) {
                let (rook_id, rook) = self
                    .player_pieces
                    .iter()
                    .find(|(_, piece)| {
                        piece.square.rank == self.king_square.rank && piece.square.file == 7
                    })
                    .expect("kingside castling without a rook");

                moves.push(Move::kingside_castle(
                    (self.king_square.rank, 7).into(),
                    *rook_id,
                    **rook,
                ));
            }
        };

        moves
    }
}
