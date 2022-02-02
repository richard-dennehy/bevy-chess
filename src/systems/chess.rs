use crate::model::{
    AllValidMoves, LastPawnDoubleStep, MoveKind, Piece, PieceColour, PieceKind, SpecialMoveData,
    Square,
};
use crate::moves_calculator::CalculatorResult;
use crate::{easing, moves_calculator};
use bevy::prelude::*;
use bevy_mod_picking::PickingCamera;
use std::fmt::Formatter;

mod game_set_up;
use game_set_up::*;

#[cfg(test)]
mod tests {
    use super::*;

    mod checking_for_check_tests;
    mod special_move_tests;
    mod piece_movement_tests;
}

pub struct ChessPlugin;
impl Plugin for ChessPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_plugin(GameSetUpPlugin)
            .init_resource::<SelectedSquare>()
            .init_resource::<SelectedPiece>()
            .init_resource::<PromotedPawn>()
            .init_resource::<PlayerTurn>()
            .init_resource::<AllValidMoves>()
            .init_resource::<Option<HighlightedSquare>>()
            .init_resource::<SpecialMoveData>()
            .add_state(GameState::NewGame)
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
                    .with_system(apply_piece_move.system()),
            )
            .add_system_set(
                SystemSet::on_exit(GameState::TargetSquareSelected)
                    .with_system(despawn_taken_pieces.system())
                    .with_system(reset_selected.system()),
            )
            .add_system_set(
                SystemSet::on_update(GameState::MovingPiece).with_system(translate_moved_pieces.system()),
            )
            .add_system_set(
                SystemSet::on_update(GameState::PawnPromotion)
                    .with_system(promote_pawn_at_final_rank.system()),
            );
    }
}

pub struct Taken;

#[derive(Default)]
pub struct SelectedSquare(pub Option<Entity>);
#[derive(Default)]
pub struct SelectedPiece(pub Option<Entity>);
#[derive(Default)]
pub struct PromotedPawn(pub Option<Entity>);

pub struct MovePiece {
    pub from: Vec3,
    pub to: Vec3,
    pub elapsed: f32,
}

impl MovePiece {
    pub fn new(from: Square, to: Square) -> Self {
        Self {
            from: from.to_translation(),
            to: to.to_translation(),
            elapsed: 0.0,
        }
    }

    pub fn target_square(&self) -> Square {
        Square::from_translation(self.to)
    }
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
    Stalemate(PieceColour),
    PawnPromotion,
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
            GameState::Stalemate(colour) => {
                write!(
                    f,
                    "Stalemate: {} cannot make any moves\nPress R to restart",
                    colour
                )
            }
            GameState::PawnPromotion => {
                write!(f, "A pawn can be promoted\nPress Left/Right to cycle between options and Enter to confirm promotion")
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

#[allow(clippy::too_many_arguments)]
fn colour_squares(
    mut highlighted_square: ResMut<Option<HighlightedSquare>>,
    turn: Res<PlayerTurn>,
    selected_square: Res<SelectedSquare>,
    valid_moves: Res<AllValidMoves>,
    selected_piece: Res<SelectedPiece>,
    promoted_pawn: Res<PromotedPawn>,
    materials: Res<SquareMaterials>,
    pieces: Query<(Entity, &Piece)>,
    mut squares: Query<(Entity, &Square, &mut Handle<StandardMaterial>)>,
) {
    squares.for_each_mut(|(entity, square, mut material)| {
        if selected_square.0.contains(&entity) {
            *material = materials.selected.clone();
            return;
        };

        if let Some(piece) = selected_piece.0 {
            if valid_moves.contains(piece, *square) {
                *material = materials.valid_selection.clone();
                return;
            };
        } else {
            let piece = pieces
                .iter()
                .find(|(_, piece)| piece.square == *square && piece.colour == turn.0);

            if let Some((entity, _)) = piece {
                let valid_moves = valid_moves.get(entity);

                if !valid_moves.is_empty() {
                    *material = materials.valid_selection.clone();
                    return;
                }
            }
        };

        if let Some(promoted) = promoted_pawn.0 {
            let piece = pieces
                .iter()
                .find(|(entity, piece)| piece.square == *square && promoted == *entity);

            if piece.is_some() {
                *material = materials.selected.clone();
                return;
            }
        }

        *material = materials.none.clone();
    });

    if let Some(highlighted) = &mut *highlighted_square {
        let (_, _, material) = squares
            .get_mut(highlighted.entity_id)
            .expect("highlighted square should always exist");
        highlighted.previous_material = material.clone()
    }
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
    special_move_data: Res<SpecialMoveData>,
    mut all_moves: ResMut<AllValidMoves>,
    mut game_state: ResMut<State<GameState>>,
    pieces: Query<(Entity, &Piece)>,
) {
    let board_state = pieces.iter().map(|(_, piece)| piece).collect();
    let (player_pieces, opposite_pieces): (Vec<_>, Vec<_>) = pieces
        .iter()
        .partition(|(_, piece)| piece.colour == player_turn.0);

    match moves_calculator::calculate_valid_moves(
        player_turn.0,
        &special_move_data,
        player_pieces.as_slice(),
        opposite_pieces.as_slice(),
        board_state,
    ) {
        CalculatorResult::Stalemate => {
            game_state.set(GameState::Stalemate(player_turn.0)).unwrap();
        }
        CalculatorResult::Checkmate => {
            game_state.set(GameState::Checkmate(player_turn.0)).unwrap();
        }
        CalculatorResult::Ok(valid_moves) => {
            valid_moves.into_iter().for_each(|(k, v)| {
                all_moves.insert(k, v);
            });
        }
    }
}

#[allow(clippy::collapsible_else_if)]
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
        .find(|(_, piece)| piece.square == *square && piece.colour == turn.0)
        .map(|(entity, _)| {
            selected_piece.0 = Some(entity);
            game_state.set(GameState::PieceSelected).unwrap();
        })
        .unwrap_or_else(|| game_state.set(GameState::NothingSelected).unwrap());
}

#[allow(clippy::too_many_arguments)]
pub fn apply_piece_move(
    mut commands: Commands,
    selected_square: Res<SelectedSquare>,
    selected_piece: Res<SelectedPiece>,
    all_valid_moves: Res<AllValidMoves>,
    player_turn: Res<PlayerTurn>,
    mut game_state: ResMut<State<GameState>>,
    mut special_move_data: ResMut<SpecialMoveData>,
    mut promoted_pawn: ResMut<PromotedPawn>,
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
        let maybe_valid_move = valid_moves.iter().find(|m| m.target_square == *square);
        if let Some(valid_move) = maybe_valid_move {
            let (_, piece) = pieces.get_mut(piece_id).unwrap();
            let piece = *piece;
            let _ = special_move_data.last_pawn_double_step.take();

            if piece.kind == PieceKind::Pawn {
                if let MoveKind::EnPassant { target_id } = valid_move.kind {
                    commands.entity(target_id).insert(Taken);
                } else if valid_move.kind == MoveKind::PawnDoubleStep {
                    let _ = special_move_data
                        .last_pawn_double_step
                        .insert(LastPawnDoubleStep {
                            pawn_id: piece_id,
                            square: *square,
                        });
                } else if valid_move.target_square.rank == player_turn.0.final_rank() {
                    promoted_pawn.0 = Some(piece_id);
                }
            } else if piece.kind == PieceKind::King {
                let mut castling_data = special_move_data.castling_data_mut(player_turn.0);
                castling_data.king_moved = true;

                if let MoveKind::Castle {
                    rook_id,
                    rook_position,
                    king_target_y,
                    rook_target_y,
                    kingside,
                } = valid_move.kind
                {
                    commands.entity(piece_id).insert(MovePiece::new(
                        piece.square,
                        (square.rank, king_target_y).into(),
                    ));

                    commands.entity(rook_id).insert(MovePiece::new(
                        rook_position,
                        (square.rank, rook_target_y).into(),
                    ));

                    if kingside {
                        castling_data.kingside_rook_moved = true;
                    } else {
                        castling_data.queenside_rook_moved = true;
                    }

                    game_state.set(GameState::MovingPiece).unwrap();
                    return;
                }
            } else if piece.kind == PieceKind::Rook {
                let mut castling_data = special_move_data.castling_data_mut(player_turn.0);

                if piece.square.file == 0 {
                    castling_data.queenside_rook_moved = true;
                } else if piece.square.file == 7 {
                    castling_data.kingside_rook_moved = true;
                }
            }

            if let Some((target_entity, target_piece)) = pieces
                .iter_mut()
                .find(|(_, other)| other.square == *square) {
                if target_piece.kind == PieceKind::Rook {
                    let other_player = player_turn.0.opposite();
                    let mut castling_data = special_move_data.castling_data_mut(other_player);

                    if target_piece.square.rank == other_player.starting_back_rank()
                        && target_piece.square.file == 0
                    {
                        castling_data.queenside_rook_moved = true;
                    } else if target_piece.square.rank == other_player.starting_back_rank()
                        && target_piece.square.file == 7
                    {
                        castling_data.kingside_rook_moved = true;
                    }
                };

                commands.entity(target_entity).insert(Taken);
            }

            commands
                .entity(piece_id)
                .insert(MovePiece::new(piece.square, *square));

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
    valid_moves.clear();
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
    mut special_move_data: ResMut<SpecialMoveData>,
) {
    turn.0 = PieceColour::White;
    game_state.set(GameState::NothingSelected).unwrap();
    *special_move_data = Default::default();
}

fn translate_moved_pieces(
    mut commands: Commands,
    time: Res<Time>,
    promoted_pawn: Res<PromotedPawn>,
    mut state: ResMut<State<GameState>>,
    mut turn: ResMut<PlayerTurn>,
    mut query: Query<(Entity, &mut MovePiece, &mut Piece, &mut Transform)>,
) {
    // note: castling moves two pieces on the same turn

    let average_velocity = 5.0;

    let any_updated =
        query
            .iter_mut()
            .any(|(piece_entity, mut move_piece, mut piece, mut transform)| {
                let direction = move_piece.to - transform.translation;

                if direction.length() > f32::EPSILON {
                    let distance = (move_piece.from - move_piece.to).length();
                    let target_time = distance.sqrt() / average_velocity;

                    move_piece.elapsed += time.delta_seconds();
                    if move_piece.elapsed > target_time {
                        transform.translation = move_piece.to;
                    } else {
                        let t = move_piece.elapsed / target_time;
                        let eased = ease_xz(t);

                        let xz_translation = move_piece.from.lerp(move_piece.to, eased);

                        let max_height = 0.5 * distance.sqrt();
                        let y_translation = Vec3::new(0.0, ease_y(t) * max_height, 0.0);

                        transform.translation = xz_translation + y_translation;
                    }

                    true
                } else {
                    piece.square = move_piece.target_square();

                    commands.entity(piece_entity).remove::<MovePiece>();

                    false
                }
            });

    if !any_updated {
        if promoted_pawn.0.is_some() {
            state.set(GameState::PawnPromotion).unwrap();
        } else {
            turn.next();
            state.set(GameState::NothingSelected).unwrap();
        }
    }
}

/// takes an x value in 0..1, maps into -1..1, applies easing, and maps the result back into 0..1
fn ease_xz(x: f32) -> f32 {
    (easing::sigmoid(-0.1)((x * 2.0) - 1.0) + 1.0) / 2.0
}

/// takes an y value in 0..1, maps into 0..1..0, applies easing, and maps the result back into 0..1
/// such that `ease_y(0.0)` ~= `ease_y(1.0)`
fn ease_y(y: f32) -> f32 {
    easing::sigmoid(-0.2)(2.0 * if y > 0.5 { 1.0 - y } else { y })
}

#[allow(clippy::too_many_arguments)]
fn promote_pawn_at_final_rank(
    mut commands: Commands,
    mut game_state: ResMut<State<GameState>>,
    mut turn: ResMut<PlayerTurn>,
    mut promoted_pawn: ResMut<PromotedPawn>,
    input: Res<Input<KeyCode>>,
    meshes: Res<PieceMeshes>,
    materials: Res<PieceMaterials>,
    pieces: Query<(Entity, &Piece)>,
) {
    let entity = promoted_pawn
        .0
        .expect("should always have a promoted pawn entity when in PawnPromotion state");
    let (_, piece) = pieces
        .get(entity)
        .expect("promoted pawn should always exist");

    if input.just_pressed(KeyCode::Return) && piece.kind != PieceKind::Pawn {
        promoted_pawn.0 = None;
        turn.next();
        game_state.set(GameState::NothingSelected).unwrap();
    };

    let promotions = [
        PieceKind::Knight,
        PieceKind::Bishop,
        PieceKind::Rook,
        PieceKind::Queen,
    ];

    let index_of = |kind: PieceKind| {
        promotions
            .iter()
            .enumerate()
            .find_map(|(idx, k)| (*k == kind).then(|| idx))
            .unwrap_or_else(|| panic!(
                "promoted to unexpected piece kind {:?}",
                piece.kind
            ))
    };

    let new_kind = if piece.kind == PieceKind::Pawn && input.just_pressed(KeyCode::Left) {
        PieceKind::Queen
    } else if piece.kind == PieceKind::Pawn && input.just_pressed(KeyCode::Right) {
        PieceKind::Knight
    } else if input.just_pressed(KeyCode::Left) {
        let index = index_of(piece.kind);
        promotions[(index as isize - 1) as usize % promotions.len()]
    } else if input.just_pressed(KeyCode::Right) {
        let index = index_of(piece.kind);
        promotions[(index + 1) % promotions.len()]
    } else {
        return;
    };

    let square = piece.square;
    commands.entity(entity).despawn_recursive();

    let new_entity =
        game_set_up::spawn_piece(&mut commands, &materials, &meshes, turn.0, new_kind, square);
    promoted_pawn.0 = Some(new_entity);
}
