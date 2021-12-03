use crate::board::{AllValidMoves, BoardState, SpecialMoveData};
use crate::pieces::{Piece, PieceColour, PieceKind};
use bevy::prelude::Entity;

pub fn calculate_valid_moves(
    turn: PieceColour,
    special_move_data: &SpecialMoveData,
    player_pieces: &[(Entity, &Piece)],
    opposite_pieces: &[(Entity, &Piece)],
    board_state: BoardState,
) -> AllValidMoves {
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
        king,
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
    king: &'game Piece,
    board_state: BoardState,
}

type Move = (u8, u8);
type Moves = Vec<Move>;
type PieceMoves = (Entity, Moves);
type EnPassantMove = (Entity, Move);

impl<'game> MoveCalculator<'game> {
    fn calculate_valid_moves(self) -> AllValidMoves {
        let mut all_moves = AllValidMoves::default();

        let (en_passant_left, en_passant_right) = self.find_en_passant_pieces();

        self.player_pieces
            .iter()
            .chain(self.opposite_pieces.iter())
            .copied()
            .for_each(|(entity, piece)| {
                let mut valid_moves = piece.valid_moves(&self.board_state);

                if let Some((left, ep_move)) = en_passant_left {
                    if entity == left {
                        valid_moves.push(ep_move);
                    }
                } else if let Some((right, ep_move)) = en_passant_right {
                    if entity == right {
                        valid_moves.push(ep_move);
                    }
                };

                let _ = all_moves.insert(entity, valid_moves);
            });

        let pieces_attacking_king = self.pieces_attacking_king(&all_moves);
        // TODO inline these
        let safe_king_moves = self.calculate_safe_king_moves(&all_moves);
        let safe_player_moves = self.calculate_safe_player_moves(&all_moves);

        if !pieces_attacking_king.is_empty() {
            let counter_moves = self.calculate_check_counter_moves(
                safe_king_moves,
                safe_player_moves,
                pieces_attacking_king,
                en_passant_left,
                en_passant_right,
            );

            counter_moves.into_iter().for_each(|(entity, moves)| {
                let _ = all_moves.insert(entity, moves);
            });
        } else {
            let mut safe_king_moves = safe_king_moves;
            let mut castling_moves = self.calculate_castling_moves(&all_moves);
            safe_king_moves.append(&mut castling_moves);

            let _ = all_moves.insert(self.king_entity, safe_king_moves);
            safe_player_moves.into_iter().for_each(|(entity, moves)| {
                let _ = all_moves.insert(entity, moves);
            });
        }

        all_moves
    }

    fn find_en_passant_pieces(&self) -> (Option<EnPassantMove>, Option<EnPassantMove>) {
        if let Some(pawn_double_step) = &self.special_move_data.last_pawn_double_step {
            let find_pawn_in_column = |offset: i8| {
                self.player_pieces.iter().find_map(|(entity, piece)| {
                    (piece.kind == PieceKind::Pawn
                        && piece.y == (pawn_double_step.y as i8 + offset) as u8
                        && piece.colour == self.turn)
                        .then(|| {
                            let direction = piece.colour.pawn_direction();
                            let ep_move = (
                                (piece.x as i8 + direction) as u8,
                                (piece.y as i8 - offset) as u8,
                            );
                            (*entity, ep_move)
                        })
                })
            };

            (find_pawn_in_column(-1), find_pawn_in_column(1))
        } else {
            (None, None)
        }
    }

    fn calculate_safe_king_moves(&self, all_moves: &AllValidMoves) -> Moves {
        all_moves
            .get(self.king_entity)
            .into_iter()
            .filter(|(x, y)| {
                !self.opposite_pieces.iter().any(|(entity, piece)| {
                    // don't need to check which colour as `valid_moves` already handles same colour pieces
                    if self.board_state.get(*x, *y).is_some() {
                        // awkward logic to check if any piece can move to the square once the current piece is taken
                        piece
                            .path_to_take_piece_at((*x, *y))
                            .into_iter()
                            .all(|(path_x, path_y)| {
                                (path_x == *x && path_y == *y)
                                    || self.board_state.get(path_x, path_y).is_none()
                            })
                    } else {
                        all_moves.get(*entity).contains(&(*x, *y))
                    }
                })
            })
            .copied()
            .collect()
    }

    fn pieces_attacking_king(&self, all_moves: &AllValidMoves) -> Vec<(Entity, &Piece)> {
        self.opposite_pieces
            .iter()
            .filter(|(entity, _)| {
                all_moves
                    .get(*entity)
                    .contains(&(self.king.x, self.king.y))
            })
            .copied()
            .collect::<Vec<_>>()
    }

    fn calculate_safe_player_moves(&self, all_moves: &AllValidMoves) -> Vec<PieceMoves> {
        let potential_threats = self.calculate_potential_threats();

        self.player_pieces
            .iter()
            .filter(|(entity, _)| *entity != self.king_entity)
            .map(|(entity, piece)| {
                let safe_moves = all_moves
                    .get(*entity)
                    .iter()
                    .filter(|(x, y)| {
                        // safe move iff: doesn't open up a path to the king, or stays within the same path, or takes the piece
                        potential_threats.iter().all(|(_, threat)| {
                            let path = threat.path_to_take_piece_at((self.king.x, self.king.y));
                            !path.contains(&(piece.x, piece.y)) || path.contains(&(*x, *y))
                        })
                    })
                    .copied()
                    .collect::<Vec<_>>();
                (*entity, safe_moves)
            })
            .collect()
    }

    fn calculate_potential_threats(&self) -> Vec<(Entity, &'game Piece)> {
        self.opposite_pieces
            .iter()
            .filter(|(_, piece)| {
                let path = piece
                    .path_to_take_piece_at((self.king.x, self.king.y));

                if path.is_empty() {
                    return false;
                }

                let obstructions = path
                    .into_iter()
                    .filter_map(|(x, y)| self.board_state.get(x, y).as_ref())
                    .collect::<Vec<_>>();

                // don't need to worry about pieces that are blocked by pieces of the same colour (as these can't be moved this turn) or pieces that are blocked by multiple pieces
                !obstructions.contains(&&self.turn.opposite()) || obstructions.len() >= 2
            })
            .copied()
            .collect()
    }

    fn calculate_check_counter_moves(
        &self,
        safe_king_moves: Moves,
        safe_moves: Vec<PieceMoves>,
        pieces_attacking_king: Vec<(Entity, &Piece)>,
        en_passant_left: Option<EnPassantMove>,
        en_passant_right: Option<EnPassantMove>,
    ) -> Vec<PieceMoves> {
        std::iter::once((self.king_entity, safe_king_moves))
            .chain(safe_moves.iter().map(|(entity, safe_piece_moves)| {
                // this piece can only move if it can take or block the piece that has the king in check
                let counter_moves = safe_piece_moves
                    .iter()
                    .filter(|(move_x, move_y)| {
                        pieces_attacking_king
                            .iter()
                            .all(|(opposite_entity, opposite_piece)| {
                                // todo extract this or precalculate
                                let is_en_passant_target = self
                                    .special_move_data
                                    .last_pawn_double_step
                                    .as_ref()
                                    .map_or(false, |e| e.pawn_id == *opposite_entity);
                                let is_left_en_passant = en_passant_left
                                    .as_ref()
                                    .map_or(false, |(_, ep_move)| ep_move == &(*move_x, *move_y));
                                let is_right_en_passant = en_passant_right
                                    .as_ref()
                                    .map_or(false, |(_, ep_move)| ep_move == &(*move_x, *move_y));

                                let can_take_en_passant = is_en_passant_target
                                    && (is_left_en_passant || is_right_en_passant);

                                let can_take_directly =
                                    opposite_piece.x == *move_x && opposite_piece.y == *move_y;

                                let blocks_piece = opposite_piece
                                    .path_to_take_piece_at((self.king.x, self.king.y))
                                    .contains(&(*move_x, *move_y));

                                can_take_en_passant || can_take_directly || blocks_piece
                            })
                    })
                    .copied()
                    .collect::<Vec<_>>();

                (*entity, counter_moves)
            }))
            .collect()
    }

    fn calculate_castling_moves(&self, all_moves: &AllValidMoves) -> Moves {
        let mut moves = vec![];
        let castling_data = self.special_move_data.castling_data(self.turn);

        if !castling_data.king_moved {
            if !castling_data.queenside_rook_moved {
                let first_move = (self.king.x, self.king.y - 1);
                let second_move = (self.king.x, self.king.y - 2);
                let passed_through = (self.king.x, self.king.y - 3);

                if self.board_state.get(first_move.0, first_move.1).is_none()
                    && self.board_state.get(second_move.0, second_move.1).is_none()
                    && self
                        .board_state
                        .get(passed_through.0, passed_through.1)
                        .is_none()
                    && self.opposite_pieces.iter().all(|(entity, _)| {
                        let moves = all_moves.get(*entity);
                        !(moves.contains(&first_move) || moves.contains(&second_move))
                    })
                {
                    moves.push((self.king.x, 0));
                }
            }

            if !castling_data.kingside_rook_moved {
                let first_move = (self.king.x, self.king.y + 1);
                let second_move = (self.king.x, self.king.y + 2);

                if self.board_state.get(first_move.0, first_move.1).is_none()
                    && self.board_state.get(second_move.0, second_move.1).is_none()
                    && self.opposite_pieces.iter().all(|(entity, _)| {
                        let moves = all_moves.get(*entity);
                        !(moves.contains(&first_move) || moves.contains(&second_move))
                    })
                {
                    moves.push((self.king.x, 7));
                }
            }
        };

        moves
    }
}
