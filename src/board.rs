use crate::pieces::{Piece, PieceColour, PieceKind};
use bevy::prelude::*;
use bevy::utils::HashMap;
use bevy_mod_picking::{PickableBundle, PickingCamera};
use std::fmt::Formatter;
use std::ops::{Deref, DerefMut};

pub struct BoardPlugin;
impl Plugin for BoardPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.init_resource::<SquareMaterials>()
            .init_resource::<SelectedSquare>()
            .init_resource::<SelectedPiece>()
            .init_resource::<PlayerTurn>()
            .init_resource::<AllValidMoves>()
            .init_resource::<Option<HighlightedSquare>>()
            .init_resource::<Option<EnPassantData>>()
            .init_resource::<WhiteCastlingData>()
            .init_resource::<BlackCastlingData>()
            .add_state(GameState::NewGame)
            .add_startup_system(create_board.system())
            .add_system(highlight_square_on_hover.system())
            .add_system(restart_game.system())
            .add_system_set(
                SystemSet::on_update(GameState::NewGame).with_system(start_new_game.system()),
            )
            .add_system_set(
                SystemSet::on_enter(GameState::NothingSelected)
                    .with_system(reset_selected.system().label("reset_selected"))
                    .with_system(
                        calculate_all_moves
                            .system()
                            .label("calculate_moves")
                            .after("reset_selected"),
                    )
                    .with_system(colour_squares.system().after("calculate_moves")),
            )
            .add_system_set(
                SystemSet::on_update(GameState::NothingSelected)
                    .with_system(select_square.system()),
            )
            .add_system_set(
                SystemSet::on_update(GameState::SquareSelected).with_system(select_piece.system()),
            )
            .add_system_set(
                SystemSet::on_enter(GameState::PieceSelected).with_system(colour_squares.system()),
            )
            .add_system_set(
                SystemSet::on_update(GameState::PieceSelected).with_system(select_square.system()),
            )
            .add_system_set(
                SystemSet::on_update(GameState::TargetSquareSelected)
                    .with_system(move_piece.system()),
            )
            .add_system_set(
                SystemSet::on_exit(GameState::TargetSquareSelected)
                    .with_system(despawn_taken_pieces.system()),
            );
    }
}

#[derive(Debug, Clone)]
pub struct BoardState {
    squares: [Option<PieceColour>; 64],
}

impl BoardState {
    pub fn get(&self, x: u8, y: u8) -> &Option<PieceColour> {
        &self.squares[(x * 8 + y) as usize]
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
            squares[(piece.x * 8 + piece.y) as usize] = Some(piece.colour);
        });

        Self { squares }
    }
}

#[derive(Debug, PartialEq)]
pub struct Square {
    pub x: u8,
    pub y: u8,
}

impl Square {
    fn is_white(&self) -> bool {
        (self.x + self.y + 1) % 2 == 0
    }
}

pub struct Taken;

#[derive(Default)]
pub struct SelectedSquare(pub Option<Entity>);
#[derive(Default)]
pub struct SelectedPiece(pub Option<Entity>);

#[derive(Debug, PartialEq)]
pub struct EnPassantData {
    pub piece_id: Entity,
    pub x: u8,
    pub y: u8,
}

#[derive(Default)]
pub struct CastlingData {
    pub king_moved: bool,
    pub kingside_rook_moved: bool,
    pub queenside_rook_moved: bool,
}

#[derive(Default)]
pub struct WhiteCastlingData(pub CastlingData);
#[derive(Default)]
pub struct BlackCastlingData(pub CastlingData);

impl Deref for WhiteCastlingData {
    type Target = CastlingData;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Deref for BlackCastlingData {
    type Target = CastlingData;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for WhiteCastlingData {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl DerefMut for BlackCastlingData {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Default)]
pub struct AllValidMoves(HashMap<Entity, Vec<(u8, u8)>>);

impl AllValidMoves {
    pub fn get(&self, piece_id: Entity) -> &Vec<(u8, u8)> {
        self.0
            .get(&piece_id)
            .expect("all pieces should have moves calculated")
    }

    pub fn insert(&mut self, piece_id: Entity, moves: Vec<(u8, u8)>) {
        self.0.insert(piece_id, moves);
    }
}

pub struct MovePiece {
    pub target_x: f32,
    pub target_y: f32,
}

struct HighlightedSquare {
    entity_id: Entity,
    previous_material: Handle<StandardMaterial>,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum GameState {
    // only exists to guarantee the "new turn" systems always run after resetting everything
    NewGame,
    NothingSelected,
    SquareSelected,
    PieceSelected,
    TargetSquareSelected,
    MovingPiece,
    Checkmate(PieceColour),
}

impl core::fmt::Display for GameState {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            GameState::NewGame | GameState::NothingSelected | GameState::SquareSelected => {
                write!(f, "Select a piece to move")
            }
            GameState::PieceSelected => write!(f, "Select a target square"),
            GameState::TargetSquareSelected | GameState::MovingPiece => {
                write!(f, "Moving piece to target square")
            }
            GameState::Checkmate(colour) => {
                write!(f, "{}'s King is in checkmate\nPress R to restart", colour)
            }
        }
    }
}

#[derive(Debug)]
pub struct PlayerTurn(pub PieceColour);
impl Default for PlayerTurn {
    fn default() -> Self {
        PlayerTurn(PieceColour::White)
    }
}

impl PlayerTurn {
    pub fn next(&mut self) {
        self.0 = self.0.opposite()
    }
}

fn create_board(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    materials: ResMut<SquareMaterials>,
) {
    let mesh = meshes.add(Mesh::from(shape::Plane { size: 1.0 }));

    (0..8).for_each(|x| {
        (0..8).for_each(|y| {
            let square = Square { x, y };
            commands
                .spawn_bundle(PbrBundle {
                    mesh: mesh.clone(),
                    material: if square.is_white() {
                        materials.white.clone()
                    } else {
                        materials.black.clone()
                    },
                    transform: Transform::from_translation(Vec3::new(x as f32, 0.0, y as f32)),
                    ..Default::default()
                })
                .insert_bundle(PickableBundle::default())
                .insert(square);
        })
    })
}

// fixme this is highlighting the selected piece as well as its valid moves
fn colour_squares(
    turn: Res<PlayerTurn>,
    selected_square: Res<SelectedSquare>,
    valid_moves: Res<AllValidMoves>,
    selected_piece: Res<SelectedPiece>,
    materials: Res<SquareMaterials>,
    pieces: Query<(Entity, &Piece)>,
    squares: Query<(Entity, &Square, &mut Handle<StandardMaterial>)>,
) {
    squares.for_each_mut(|(entity, square, mut material)| {
        if selected_square.0.contains(&entity) {
            *material = materials.selected.clone();
            return;
        };

        if let Some(piece) = selected_piece.0 {
            let valid_moves = valid_moves.get(piece);

            if valid_moves.contains(&(square.x, square.y)) {
                *material = materials.valid_selection.clone();
                return;
            };
        } else {
            let piece = pieces.iter().find(|(_, piece)| {
                piece.x == square.x && piece.y == square.y && piece.colour == turn.0
            });

            if let Some((entity, _)) = piece {
                let valid_moves = valid_moves.get(entity);

                if !valid_moves.is_empty() {
                    *material = materials.valid_selection.clone();
                    return;
                }
            }
        };

        *material = if square.is_white() {
            materials.white.clone()
        } else {
            materials.black.clone()
        };
    });
}

fn highlight_square_on_hover(
    materials: Res<SquareMaterials>,
    mut previous_highlighted_square: ResMut<Option<HighlightedSquare>>,
    pick_state: Query<&PickingCamera>,
    mut squares: Query<&mut Handle<StandardMaterial>, With<Square>>,
) {
    if let Some(previous) = &*previous_highlighted_square {
        let mut material = squares.get_mut(previous.entity_id).unwrap();
        *material = previous.previous_material.clone();
    };

    if let Some(top_entity) = selected_entity(pick_state) {
        if let Ok(mut material) = squares.get_mut(top_entity) {
            *previous_highlighted_square = Some(HighlightedSquare {
                entity_id: top_entity,
                previous_material: material.clone(),
            });

            *material = materials.highlight.clone();
        }
    };
}

pub fn calculate_all_moves(
    player_turn: Res<PlayerTurn>,
    en_passant_data: Res<Option<EnPassantData>>,
    white_castling_data: Res<WhiteCastlingData>,
    black_castling_data: Res<BlackCastlingData>,
    mut all_moves: ResMut<AllValidMoves>,
    mut game_state: ResMut<State<GameState>>,
    pieces: Query<(Entity, &Piece)>,
) {
    let board_state = pieces.iter().map(|(_, piece)| piece).collect();

    let (en_passant_left, en_passant_right) = if let Some(en_passant) = &*en_passant_data {
        let left = pieces.iter().find_map(|(entity, piece)| {
            (piece.kind == PieceKind::Pawn
                && piece.y == en_passant.y - 1
                && piece.colour == player_turn.0)
                .then(|| {
                    let direction = piece.colour.pawn_direction();
                    let ep_move = ((piece.x as i8 + direction) as u8, piece.y + 1);
                    (entity, ep_move)
                })
        });
        let right = pieces.iter().find_map(|(entity, piece)| {
            (piece.kind == PieceKind::Pawn
                && piece.y == en_passant.y + 1
                && piece.colour == player_turn.0)
                .then(|| {
                    let direction = piece.colour.pawn_direction();
                    let ep_move = ((piece.x as i8 + direction) as u8, piece.y - 1);
                    (entity, ep_move)
                })
        });

        (left, right)
    } else {
        (None, None)
    };

    let castling_data = if player_turn.0 == PieceColour::White {
        &**white_castling_data
    } else {
        &**black_castling_data
    };

    // note: this calculates all potential moves for both sides - this makes it easier to check for check(mate)
    pieces.for_each(|(entity, piece)| {
        let mut valid_moves = piece.valid_moves(&board_state);

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

    let (player_pieces, opposite_pieces): (Vec<_>, Vec<_>) = pieces
        .iter()
        .partition(|(_, piece)| piece.colour == player_turn.0);

    let (king_entity, king) = player_pieces
        .iter()
        .find(|(_, piece)| piece.colour == player_turn.0 && piece.kind == PieceKind::King)
        .copied()
        .expect("there should always be 2 kings");

    // find opposite colour pieces that have the king in check
    let pieces_threatening_king = opposite_pieces
        .iter()
        .filter_map(|(entity, _)| {
            all_moves
                .get(*entity)
                .contains(&(king.x, king.y))
                .then(|| pieces.get(*entity).unwrap())
        })
        .collect::<Vec<_>>();

    // moves that the king can make without leaving itself in check
    let safe_king_moves = all_moves
        .get(king_entity)
        .into_iter()
        .filter(|(x, y)| {
            !opposite_pieces.iter().any(|(entity, piece)| {
                // don't need to check which colour as `valid_moves` already handles same colour pieces
                if board_state.get(*x, *y).is_some() {
                    // awkward logic to check if any piece can move to the square once the current piece is taken
                    piece
                        .path_to_take_piece_at((*x, *y))
                        .into_iter()
                        .all(|(path_x, path_y)| {
                            (path_x == *x && path_y == *y)
                                || board_state.get(path_x, path_y).is_none()
                        })
                } else {
                    all_moves.get(*entity).contains(&(*x, *y))
                }
            })
        })
        .copied()
        .collect::<Vec<_>>();

    // pieces that could reach the king but are blocked by a single piece of the player colour
    let potential_threats = opposite_pieces
        .iter()
        .filter(|(_, piece)| {
            let obstructions = piece
                .path_to_take_piece_at((king.x, king.y))
                .into_iter()
                .filter_map(|(x, y)| board_state.get(x, y).as_ref())
                .collect::<Vec<_>>();

            // don't need to worry about pieces that are blocked by pieces of the same colour (as these can't be moved this turn) or pieces that are blocked by multiple pieces
            !obstructions.contains(&&player_turn.0.opposite()) || obstructions.len() >= 2
        })
        .collect::<Vec<_>>();

    // moves that player pieces (excluding the king) can make without exposing the king to check
    let safe_player_moves = player_pieces
        .iter()
        .filter(|(entity, _)| *entity != king_entity)
        .map(|(entity, piece)| {
            let safe_moves = all_moves
                .get(*entity)
                .iter()
                .filter(|(x, y)| {
                    // safe move iff: doesn't open up a path to the king, or stays within the same path, or takes the piece
                    potential_threats.iter().all(|(_, threat)| {
                        let path = threat.path_to_take_piece_at((king.x, king.y));
                        !path.contains(&(piece.x, piece.y)) || path.contains(&(*x, *y))
                    })
                })
                .copied()
                .collect::<Vec<_>>();
            (entity, safe_moves)
        })
        .collect::<Vec<_>>();

    // king is currently in check - only allow moves that protect the king
    if !pieces_threatening_king.is_empty() {
        let counter_moves: Vec<(Entity, Vec<(u8, u8)>)> =
            std::iter::once((king_entity, safe_king_moves))
                .chain(safe_player_moves.iter().map(|(entity, safe_moves)| {
                    // this piece can only move if it can take or block the piece that has the king in check
                    let counter_moves = safe_moves
                        .iter()
                        .filter(|(move_x, move_y)| {
                            pieces_threatening_king.iter().all(
                                |(opposite_entity, opposite_piece)| {
                                    let can_take_en_passant = en_passant_data
                                        .as_ref()
                                        .map_or(false, |e| e.piece_id == *opposite_entity)
                                        && (en_passant_left
                                            .as_ref()
                                            .map_or(false, |(_, ep_move)| {
                                                ep_move == &(*move_x, *move_y)
                                            })
                                            || en_passant_right
                                                .as_ref()
                                                .map_or(false, |(_, ep_move)| {
                                                    ep_move == &(*move_x, *move_y)
                                                }));

                                    let can_take_directly =
                                        opposite_piece.x == *move_x && opposite_piece.y == *move_y;

                                    let blocks_piece = opposite_piece
                                        .path_to_take_piece_at((king.x, king.y))
                                        .contains(&(*move_x, *move_y));

                                    can_take_en_passant || can_take_directly || blocks_piece
                                },
                            )
                        })
                        .copied()
                        .collect::<Vec<_>>();

                    (**entity, counter_moves)
                }))
                .collect();

        if counter_moves.iter().all(|(_, moves)| moves.is_empty()) {
            game_state.set(GameState::Checkmate(player_turn.0)).unwrap();
        }

        counter_moves.into_iter().for_each(|(entity, moves)| {
            let _ = all_moves.insert(entity, moves);
        });
    } else {
        let mut safe_king_moves = safe_king_moves;
        if !castling_data.king_moved {
            if !castling_data.queenside_rook_moved {
                let first_move = (king.x, king.y - 1);
                let second_move = (king.x, king.y - 2);
                let passed_through = (king.x, king.y - 3);

                if board_state.get(first_move.0, first_move.1).is_none()
                    && board_state.get(second_move.0, second_move.1).is_none()
                    && board_state.get(passed_through.0, passed_through.1).is_none()
                    && opposite_pieces.iter().all(|(entity, _)| {
                        let moves = all_moves.get(*entity);
                        !(moves.contains(&first_move) || moves.contains(&second_move))
                    })
                {
                    safe_king_moves.push((king.x, 0));
                }
            }

            if !castling_data.kingside_rook_moved {
                let first_move = (king.x, king.y + 1);
                let second_move = (king.x, king.y + 2);

                if board_state.get(first_move.0, first_move.1).is_none()
                    && board_state.get(second_move.0, second_move.1).is_none()
                    && opposite_pieces.iter().all(|(entity, _)| {
                        let moves = all_moves.get(*entity);
                        !(moves.contains(&first_move)
                            || moves.contains(&second_move))
                    })
                {
                    safe_king_moves.push((king.x, 7));
                }
            }
        }

        let _ = all_moves.insert(king_entity, safe_king_moves);
        safe_player_moves.into_iter().for_each(|(entity, moves)| {
            let _ = all_moves.insert(*entity, moves);
        })
    }
}

fn select_square(
    mut input: ResMut<Input<MouseButton>>,
    mut selected_square: ResMut<SelectedSquare>,
    selected_piece: Res<SelectedPiece>,
    mut game_state: ResMut<State<GameState>>,
    pick_state: Query<&PickingCamera>,
    squares: Query<&Square>,
) {
    if !input.just_pressed(MouseButton::Left) {
        return;
    }

    input.reset(MouseButton::Left);

    if let Some(square_entity) = selected_entity(pick_state) {
        if squares.get(square_entity).is_ok() {
            selected_square.0 = Some(square_entity);
        }

        if selected_piece.0.is_some() {
            game_state.set(GameState::TargetSquareSelected).unwrap();
        } else {
            game_state.set(GameState::SquareSelected).unwrap();
        }
    } else {
        if *game_state.current() != GameState::NothingSelected {
            game_state.set(GameState::NothingSelected).unwrap();
        }
    };
}

fn selected_entity(pick_state: Query<&PickingCamera>) -> Option<Entity> {
    if let Some((entity, _)) = pick_state.single().unwrap().intersect_top() {
        Some(entity)
    } else {
        None
    }
}

fn select_piece(
    mut selected_piece: ResMut<SelectedPiece>,
    selected_square: Res<SelectedSquare>,
    mut game_state: ResMut<State<GameState>>,
    turn: Res<PlayerTurn>,
    squares: Query<&Square>,
    pieces: Query<(Entity, &Piece)>,
) {
    let square = if let Some(entity) = selected_square.0 {
        squares.get(entity).unwrap()
    } else {
        return;
    };

    pieces
        .iter()
        .find(|(_, piece)| piece.x == square.x && piece.y == square.y && piece.colour == turn.0)
        .map(|(entity, _)| {
            selected_piece.0 = Some(entity);
            game_state.set(GameState::PieceSelected).unwrap();
        })
        .unwrap_or_else(|| game_state.set(GameState::NothingSelected).unwrap());
}

pub fn move_piece(
    mut commands: Commands,
    selected_square: Res<SelectedSquare>,
    selected_piece: Res<SelectedPiece>,
    all_valid_moves: Res<AllValidMoves>,
    player_turn: Res<PlayerTurn>,
    mut en_passant_data: ResMut<Option<EnPassantData>>,
    mut game_state: ResMut<State<GameState>>,
    mut white_castling_data: ResMut<WhiteCastlingData>,
    mut black_castling_data: ResMut<BlackCastlingData>,
    squares: Query<&Square>,
    mut pieces: Query<(Entity, &mut Piece)>,
) {
    let square = if let Some(entity) = selected_square.0 {
        squares.get(entity).unwrap()
    } else {
        return;
    };

    if let Some(piece_id) = selected_piece.0 {
        let valid_moves = all_valid_moves.get(piece_id);
        if valid_moves.contains(&(square.x, square.y)) {
            let (_, piece) = pieces.get_mut(piece_id).unwrap();
            let en_passant = en_passant_data.take();

            if piece.kind == PieceKind::Pawn {
                if let Some(en_passant) = en_passant {
                    if piece.x == en_passant.x {
                        commands.entity(en_passant.piece_id).insert(Taken);
                    }
                } else if piece.x.abs_diff(square.x) == 2 {
                    let _ = en_passant_data.insert(EnPassantData {
                        piece_id,
                        x: square.x,
                        y: square.y,
                    });
                }
            } else if piece.kind == PieceKind::King {
                let mut castling_data = if player_turn.0 == PieceColour::White {
                    &mut **white_castling_data
                } else {
                    &mut **black_castling_data
                };
                let king_moved = castling_data.king_moved;

                castling_data.king_moved = true;

                if !king_moved && (square.y == 0 || square.y == 7)
                {
                    let (rook_id, _) = pieces
                        .iter_mut()
                        .find(|(_, other)| other.x == square.x && other.y == square.y)
                        .expect("castling without a rook");

                    // move king by 2 towards rook
                    commands.entity(piece_id).insert(MovePiece {
                        target_x: square.x as f32,
                        target_y: if square.y == 0 { 2.0 } else { 6.0 },
                    });

                    // move rook 1 past king
                    commands.entity(rook_id).insert(MovePiece {
                        target_x: square.x as f32,
                        target_y: if square.y == 0 { 3.0 } else { 5.0 },
                    });

                    if square.y == 0 {
                        castling_data.queenside_rook_moved = true;
                    } else {
                        castling_data.kingside_rook_moved = true;
                    };

                    game_state.set(GameState::MovingPiece).unwrap();
                    return;
                }
            } else if piece.kind == PieceKind::Rook {
                let mut castling_data = if player_turn.0 == PieceColour::White {
                    &mut **white_castling_data
                } else {
                    &mut **black_castling_data
                };

                if !castling_data.queenside_rook_moved && piece.y == 0 {
                    castling_data.queenside_rook_moved = true;
                } else if !castling_data.kingside_rook_moved && piece.y == 7 {
                    castling_data.kingside_rook_moved = true;
                }
            }

            pieces
                .iter_mut()
                .find(|(_, other)| other.x == square.x && other.y == square.y)
                .map(|(other_entity, piece)| {
                    if piece.kind == PieceKind::Rook {
                        // todo might be able to simplify this
                        if player_turn.0 == PieceColour::White && piece.x == 7 {
                            if piece.y == 0 {
                                black_castling_data.queenside_rook_moved = true;
                            } else if piece.y == 7 {
                                black_castling_data.kingside_rook_moved = true;
                            }
                        } else if player_turn.0 == PieceColour::Black && piece.x == 0 {
                            if piece.y == 0 {
                                white_castling_data.queenside_rook_moved = true;
                            } else if piece.y == 7 {
                                white_castling_data.kingside_rook_moved = true;
                            }
                        }
                    }

                    commands.entity(other_entity).insert(Taken);
                });

            commands.entity(piece_id).insert(MovePiece {
                target_x: square.x as f32,
                target_y: square.y as f32,
            });

            game_state.set(GameState::MovingPiece).unwrap();
        } else {
            game_state.set(GameState::NothingSelected).unwrap();
        };
    }
}

fn reset_selected(
    mut selected_square: ResMut<SelectedSquare>,
    mut selected_piece: ResMut<SelectedPiece>,
    mut valid_moves: ResMut<AllValidMoves>,
    mut highlighted: ResMut<Option<HighlightedSquare>>,
) {
    selected_square.0 = None;
    selected_piece.0 = None;
    valid_moves.0.clear();
    *highlighted = None;
}

fn despawn_taken_pieces(
    mut commands: Commands,
    mut state: ResMut<State<GameState>>,
    turn: Res<PlayerTurn>,
    query: Query<(Entity, &Piece, &Taken)>,
) {
    query.for_each(|(entity, piece, _)| {
        if piece.kind == PieceKind::King {
            state.set(GameState::Checkmate(turn.0)).unwrap();
        }

        commands.entity(entity).despawn_recursive();
    })
}

fn restart_game(input: Res<Input<KeyCode>>, mut state: ResMut<State<GameState>>) {
    if input.just_pressed(KeyCode::R) {
        state.set(GameState::NewGame).unwrap();
    }
}

fn start_new_game(
    mut game_state: ResMut<State<GameState>>,
    mut turn: ResMut<PlayerTurn>,
    mut white_castling_data: ResMut<WhiteCastlingData>,
    mut black_castling_data: ResMut<BlackCastlingData>,
) {
    turn.0 = PieceColour::White;
    game_state.set(GameState::NothingSelected).unwrap();
    **white_castling_data = CastlingData::default();
    **black_castling_data = CastlingData::default();
}

struct SquareMaterials {
    highlight: Handle<StandardMaterial>,
    selected: Handle<StandardMaterial>,
    valid_selection: Handle<StandardMaterial>,
    black: Handle<StandardMaterial>,
    white: Handle<StandardMaterial>,
}

impl FromWorld for SquareMaterials {
    fn from_world(world: &mut World) -> Self {
        let mut materials = world
            .get_resource_mut::<Assets<StandardMaterial>>()
            .unwrap();
        SquareMaterials {
            highlight: materials.add(Color::rgb(0.8, 0.3, 0.3).into()),
            selected: materials.add(Color::rgb(0.9, 0.1, 0.1).into()),
            valid_selection: materials.add(Color::rgb(0.4, 0.4, 0.9).into()),
            black: materials.add(Color::rgb(0.0, 0.1, 0.1).into()),
            white: materials.add(Color::rgb(1.0, 0.9, 0.9).into()),
        }
    }
}
