use std::collections::hash_map::IntoIter;
use std::fmt::Formatter;
use bevy::math::Vec3;
use bevy::prelude::Entity;
use bevy::utils::HashMap;

#[cfg(test)]
mod tests {
    use super::*;

    mod board_tests;
    mod piece_tests;
}

#[derive(Debug, Copy, Clone)]
pub struct Piece {
    pub colour: PieceColour,
    pub kind: PieceKind,
    pub square: Square,
}

impl Piece {
    pub fn white(kind: PieceKind, square: Square) -> Self {
        Piece {
            colour: PieceColour::White,
            kind,
            square,
        }
    }

    pub fn black(kind: PieceKind, square: Square) -> Self {
        Piece {
            colour: PieceColour::Black,
            kind,
            square,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum PieceKind {
    King,
    Queen,
    Bishop,
    Knight,
    Rook,
    Pawn,
}
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum PieceColour {
    White,
    Black,
}

impl PieceColour {
    pub fn opposite(&self) -> Self {
        match self {
            PieceColour::White => PieceColour::Black,
            PieceColour::Black => PieceColour::White,
        }
    }

    pub fn pawn_direction(&self) -> i8 {
        if *self == PieceColour::Black {
            -1
        } else {
            1
        }
    }

    pub fn starting_front_rank(&self) -> u8 {
        match self {
            PieceColour::White => 1,
            PieceColour::Black => 6,
        }
    }

    pub fn starting_back_rank(&self) -> u8 {
        match self {
            PieceColour::White => 0,
            PieceColour::Black => 7,
        }
    }

    pub fn final_rank(&self) -> u8 {
        match self {
            PieceColour::White => 7,
            PieceColour::Black => 0,
        }
    }
}

impl core::fmt::Display for PieceColour {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                PieceColour::White => "White",
                PieceColour::Black => "Black",
            }
        )
    }
}

#[derive(Debug, PartialEq)]
pub struct PiecePath {
    potential_moves: Vec<PotentialMove>,
    colour: PieceColour,
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub struct Obstruction {
    pub square: Square,
    pub colour: PieceColour,
}

impl PiecePath {
    pub fn new(potential_moves: Vec<PotentialMove>, colour: PieceColour) -> Self {
        Self {
            potential_moves,
            colour,
        }
    }

    pub fn single(potential_move: PotentialMove, colour: PieceColour) -> Self {
        Self {
            potential_moves: vec![potential_move],
            colour,
        }
    }

    pub fn from_iterator(
        iter: impl Iterator<Item = PotentialMove>,
        colour: PieceColour,
    ) -> Option<Self> {
        let moves = iter.collect::<Vec<_>>();
        if moves.is_empty() {
            None
        } else {
            Some(Self::new(moves, colour))
        }
    }

    pub fn legal_path(&self) -> impl Iterator<Item = Move> + '_ {
        // this needs to return an Iterator (even though it makes this code a bit awkward)
        // otherwise it causes lifetime issues for the call sites in moves_calculator
        self.potential_moves
            .iter()
            .scan(false, |blocked, potential_move| {
                if *blocked {
                    return None;
                };

                if let Some(colour) = potential_move.blocked_by {
                    *blocked = true;
                    (colour == self.colour.opposite()).then(|| potential_move.to_move())
                } else {
                    Some(potential_move.to_move())
                }
            })
    }

    pub fn legal_path_vec(&self) -> Vec<Move> {
        self.legal_path().collect()
    }

    pub fn obstructions(&self) -> Vec<Obstruction> {
        self.potential_moves
            .iter()
            .filter_map(|potential_move| {
                potential_move.blocked_by.map(|blockage| Obstruction {
                    square: potential_move.target_square,
                    colour: blockage,
                })
            })
            .collect()
    }

    pub fn contains(&self, square: Square) -> bool {
        self.potential_moves
            .iter()
            .any(|potential| potential.target_square == square)
    }

    pub fn truncate_to(&self, square: Square) -> Option<Self> {
        if self.contains(square) {
            Some(PiecePath {
                potential_moves: self
                    .potential_moves
                    .iter()
                    // take_while_and_then_one_more_please
                    .scan(false, |done, p_move| {
                        if *done {
                            return None;
                        };

                        *done = p_move.target_square == square;
                        Some(p_move)
                    })
                    .copied()
                    .collect(),
                colour: self.colour,
            })
        } else {
            None
        }
    }
}

#[derive(Debug)]
pub struct PawnMoves {
    pub attack_left: Option<PotentialMove>,
    pub attack_right: Option<PotentialMove>,
    pub advance_one: Option<PotentialMove>,
    pub advance_two: Option<PotentialMove>,
}

impl Piece {
    pub fn valid_moves(&self, board: &BoardState) -> Vec<PiecePath> {
        let potential_move = |(x, y): (u8, u8)| PotentialMove {
            kind: MoveKind::Standard,
            target_square: (x, y).into(),
            blocked_by: *board.get((x, y).into()),
        };

        let up = || {
            PiecePath::from_iterator(
                ((self.square.rank + 1)..8)
                    .map(|new_rank| potential_move((new_rank, self.square.file))),
                self.colour,
            )
        };

        let down = || {
            PiecePath::from_iterator(
                (0..self.square.rank)
                    .rev()
                    .map(|new_rank| potential_move((new_rank, self.square.file))),
                self.colour,
            )
        };

        let left = || {
            PiecePath::from_iterator(
                (0..self.square.file)
                    .rev()
                    .map(|new_file| potential_move((self.square.rank, new_file))),
                self.colour,
            )
        };

        let right = || {
            PiecePath::from_iterator(
                ((self.square.file + 1)..8)
                    .map(|new_rank| potential_move((self.square.rank, new_rank))),
                self.colour,
            )
        };

        let up_left = || {
            PiecePath::from_iterator(
                ((self.square.rank + 1)..8)
                    .filter_map(|new_rank| {
                        let diff = self.square.rank.abs_diff(new_rank);
                        (diff <= self.square.file).then(|| (new_rank, self.square.file - diff))
                    })
                    .map(potential_move),
                self.colour,
            )
        };

        let up_right = || {
            PiecePath::from_iterator(
                ((self.square.rank + 1)..8)
                    .filter_map(|new_rank| {
                        let new_file = self.square.file + self.square.rank.abs_diff(new_rank);
                        (new_file < 8).then(|| (new_rank, new_file))
                    })
                    .map(potential_move),
                self.colour,
            )
        };

        let down_left = || {
            PiecePath::from_iterator(
                (0..self.square.rank)
                    .rev()
                    .filter_map(|new_rank| {
                        let diff = self.square.rank.abs_diff(new_rank);
                        (diff <= self.square.file).then(|| (new_rank, self.square.file - diff))
                    })
                    .map(potential_move),
                self.colour,
            )
        };

        let down_right = || {
            PiecePath::from_iterator(
                (0..self.square.rank)
                    .rev()
                    .filter_map(|new_rank| {
                        let new_file = self.square.file + self.square.rank.abs_diff(new_rank);
                        (new_file < 8).then(|| (new_rank, new_file))
                    })
                    .map(potential_move),
                self.colour,
            )
        };

        let (rank, file) = (self.square.rank as i8, self.square.file as i8);

        let is_on_board = |(rank, file): (i8, i8)| {
            ((0..8).contains(&rank) && (0..8).contains(&file)).then(|| (rank as u8, file as u8))
        };

        match self.kind {
            PieceKind::King => [
                (rank - 1, file - 1),
                (rank - 1, file),
                (rank - 1, file + 1),
                (rank, file - 1),
                (rank, file + 1),
                (rank + 1, file - 1),
                (rank + 1, file),
                (rank + 1, file + 1),
            ]
            .into_iter()
            .filter_map(is_on_board)
            .map(potential_move)
            .map(|move_| PiecePath::single(move_, self.colour))
            .collect(),
            PieceKind::Queen => [
                up(),
                down(),
                left(),
                right(),
                up_left(),
                up_right(),
                down_left(),
                down_right(),
            ]
            .into_iter()
            .flatten()
            .collect(),
            PieceKind::Bishop => [up_left(), up_right(), down_left(), down_right()]
                .into_iter()
                .flatten()
                .collect(),
            PieceKind::Knight => [
                (rank - 2, file - 1),
                (rank - 2, file + 1),
                (rank + 2, file - 1),
                (rank + 2, file + 1),
                (rank - 1, file - 2),
                (rank - 1, file + 2),
                (rank + 1, file - 2),
                (rank + 1, file + 2),
            ]
            .into_iter()
            .filter_map(is_on_board)
            .map(potential_move)
            .map(|move_| PiecePath::single(move_, self.colour))
            .collect(),
            PieceKind::Rook => [down(), up(), right(), left()]
                .into_iter()
                .flatten()
                .collect(),
            PieceKind::Pawn => {
                let pawn_moves = self.pawn_moves(board, false);

                [
                    pawn_moves.advance_one,
                    pawn_moves.advance_two,
                    pawn_moves.attack_left,
                    pawn_moves.attack_right,
                ]
                .into_iter()
                .flatten()
                .map(|move_| PiecePath::single(move_, self.colour))
                .collect()
            }
        }
    }

    /// set `attack_empty_squares` to `false` when calculating potential moves, and `true` when checking if a move is safe
    pub fn pawn_moves(&self, board: &BoardState, attack_empty_squares: bool) -> PawnMoves {
        if self.kind != PieceKind::Pawn {
            panic!("{:?} is not a pawn", self)
        };

        let rank = self.square.rank as i8;
        let file = self.square.file;
        let direction = self.colour.pawn_direction();

        if self.square.rank == self.colour.final_rank() {
            PawnMoves {
                advance_one: None,
                advance_two: None,
                attack_left: None,
                attack_right: None,
            }
        } else {
            // note: pawns don't really fit into the "PiecePath" model
            let move_one = (rank + direction) as u8;
            let move_two = (rank + (2 * direction)) as u8;

            let advance_one =
                board
                    .get((move_one, file).into())
                    .is_none()
                    .then_some(PotentialMove::new(
                        Move::standard((move_one, file).into()),
                        None,
                    ));

            let advance_two = (self.square.rank == self.colour.starting_front_rank()
                && board.get((move_one, file).into()).is_none()
                && board.get((move_two, file).into()).is_none())
            .then_some(PotentialMove::new(
                Move::pawn_double_step((move_two, file).into()),
                None,
            ));

            let left_diagonal_occupied = || {
                board
                    .get((move_one, file - 1).into())
                    .contains(&self.colour.opposite())
            };
            let attack_left = (file != 0 && (attack_empty_squares || left_diagonal_occupied()))
                .then(|| PotentialMove::new(Move::standard((move_one, file - 1).into()), None));

            let right_diagonal_occupied = || {
                board
                    .get((move_one, file + 1).into())
                    .contains(&self.colour.opposite())
            };
            let attack_right = (file != 7 && (attack_empty_squares || right_diagonal_occupied()))
                .then(|| PotentialMove::new(Move::standard((move_one, file + 1).into()), None));

            PawnMoves {
                advance_one,
                advance_two,
                attack_left,
                attack_right,
            }
        }
    }
}


#[derive(Debug, Clone)]
pub struct BoardState {
    squares: [Option<PieceColour>; 64],
}

impl BoardState {
    pub fn get(&self, square: Square) -> &Option<PieceColour> {
        &self.squares[(square.rank * 8 + square.file) as usize]
    }

    #[cfg(test)]
    pub fn squares(&self) -> &[Option<PieceColour>] {
        &self.squares
    }
}

impl From<&[Piece]> for BoardState {
    fn from(pieces: &[Piece]) -> Self {
        pieces.iter().collect()
    }
}

impl<const N: usize> From<[Piece; N]> for BoardState {
    fn from(pieces: [Piece; N]) -> Self {
        Self::from(&pieces[..])
    }
}

impl<'piece> FromIterator<&'piece Piece> for BoardState {
    fn from_iter<T: IntoIterator<Item = &'piece Piece>>(pieces: T) -> Self {
        let mut squares = [None; 64];
        pieces.into_iter().for_each(|piece| {
            squares[(piece.square.rank * 8 + piece.square.file) as usize] = Some(piece.colour);
        });

        Self { squares }
    }
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub struct Square {
    pub rank: u8,
    pub file: u8,
}

impl Square {
    pub fn new(rank: u8, file: u8) -> Self {
        assert!(rank <= 7 && file <= 7, "({}, {}) is out of bounds", rank, file);

        Self {
            rank,
            file,
        }
    }

    pub fn from_translation(translation: Vec3) -> Self {
        let rank = (translation.z + 3.5).round() as u8;
        let file = (translation.x + 3.5).round() as u8;
        Self { rank, file }
    }

    pub fn to_translation(self) -> Vec3 {
        (self.file as f32 - 3.5, 0.0, self.rank as f32 - 3.5).into()
    }
}

impl From<(u8, u8)> for Square {
    fn from((rank, file): (u8, u8)) -> Self {
        Self::new(rank, file)
    }
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub struct PotentialMove {
    pub kind: MoveKind,
    pub target_square: Square,
    pub blocked_by: Option<PieceColour>,
}

impl PotentialMove {
    pub fn new(move_: Move, blocked_by: Option<PieceColour>) -> Self {
        PotentialMove {
            kind: move_.kind,
            target_square: move_.target_square,
            blocked_by,
        }
    }

    pub fn to_move(&self) -> Move {
        Move {
            kind: self.kind,
            target_square: self.target_square,
        }
    }
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub struct Move {
    pub target_square: Square,
    pub kind: MoveKind,
}

impl Move {
    pub fn standard(square: Square) -> Self {
        Move {
            target_square: square,
            kind: MoveKind::Standard,
        }
    }

    pub fn pawn_double_step(square: Square) -> Self {
        Move {
            target_square: square,
            kind: MoveKind::PawnDoubleStep,
        }
    }

    pub fn en_passant(square: Square, target_id: Entity) -> Self {
        Move {
            target_square: square,
            kind: MoveKind::EnPassant { target_id },
        }
    }

    pub fn kingside_castle(square: Square, rook_id: Entity, rook: Piece) -> Self {
        Move {
            target_square: square,
            kind: MoveKind::Castle {
                rook_id,
                rook_position: rook.square,
                king_target_y: 6,
                rook_target_y: 5,
                kingside: true,
            },
        }
    }

    pub fn queenside_castle(square: Square, rook_id: Entity, rook: Piece) -> Self {
        Move {
            target_square: square,
            kind: MoveKind::Castle {
                rook_id,
                rook_position: rook.square,
                king_target_y: 2,
                rook_target_y: 3,
                kingside: false,
            },
        }
    }
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum MoveKind {
    Standard,
    PawnDoubleStep,
    EnPassant {
        target_id: Entity,
    },
    Castle {
        rook_id: Entity,
        rook_position: Square,
        king_target_y: u8,
        rook_target_y: u8,
        kingside: bool,
    },
}


#[derive(Debug, PartialEq)]
pub struct LastPawnDoubleStep {
    pub pawn_id: Entity,
    pub square: Square,
}

#[derive(Debug, Default)]
pub struct SpecialMoveData {
    pub last_pawn_double_step: Option<LastPawnDoubleStep>,
    pub white_castling_data: CastlingData,
    pub black_castling_data: CastlingData,
}

impl SpecialMoveData {
    pub fn castling_data(&self, turn: PieceColour) -> &CastlingData {
        if turn == PieceColour::White {
            &self.white_castling_data
        } else {
            &self.black_castling_data
        }
    }

    pub(crate) fn castling_data_mut(&mut self, turn: PieceColour) -> &mut CastlingData {
        if turn == PieceColour::White {
            &mut self.white_castling_data
        } else {
            &mut self.black_castling_data
        }
    }
}

#[derive(Debug, Default)]
pub struct CastlingData {
    pub king_moved: bool,
    pub kingside_rook_moved: bool,
    pub queenside_rook_moved: bool,
}

#[derive(Default, Debug)]
pub struct AllValidMoves {
    _0: HashMap<Entity, Vec<Move>>,
}

impl AllValidMoves {
    pub fn get(&self, piece_id: Entity) -> &Vec<Move> {
        self._0
            .get(&piece_id)
            .expect("all pieces should have moves calculated")
    }

    pub fn insert(&mut self, piece_id: Entity, moves: Vec<Move>) {
        self._0.insert(piece_id, moves);
    }

    pub fn contains(&self, piece_id: Entity, square: Square) -> bool {
        self.get(piece_id).iter().any(|m| m.target_square == square)
    }

    pub fn clear(&mut self) {
        self._0.iter_mut().for_each(|(_, moves)| moves.clear())
    }

    pub fn into_iter(self) -> IntoIter<Entity, Vec<Move>> {
        self._0.into_iter()
    }
}