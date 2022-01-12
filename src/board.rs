use crate::moves_calculator;
use crate::moves_calculator::{Move, MoveKind};
use crate::pieces::{Piece, PieceColour, PieceKind};
use bevy::prelude::*;
use bevy::utils::HashMap;
use bevy_mod_picking::{PickableBundle, PickingCamera};
use std::fmt::Formatter;

pub struct BoardPlugin;
impl Plugin for BoardPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.init_resource::<SquareMaterials>()
            .init_resource::<SelectedSquare>()
            .init_resource::<SelectedPiece>()
            .init_resource::<PromotedPawn>()
            .init_resource::<PlayerTurn>()
            .init_resource::<AllValidMoves>()
            .init_resource::<Option<HighlightedSquare>>()
            .init_resource::<SpecialMoveData>()
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
                    .with_system(despawn_taken_pieces.system())
                    .with_system(reset_selected.system()),
            );
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
            rank: rank,
            file: file,
        }
    }

    pub fn from_translation(translation: Vec3) -> Self {
        let rank = (translation.z + 3.5).round() as u8;
        let file = (translation.x + 3.5).round() as u8;
        Self { rank: rank, file: file }
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

pub struct Taken;

#[derive(Default)]
// TODO is it possible to pass around the actual Square rather than an Entity ID?
pub struct SelectedSquare(pub Option<Entity>);
#[derive(Default)]
pub struct SelectedPiece(pub Option<Entity>);
#[derive(Default)]
pub struct PromotedPawn(pub Option<Entity>);

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

    fn castling_data_mut(&mut self, turn: PieceColour) -> &mut CastlingData {
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

// todo circular dependency between move calculator and board module isn't ideal
#[derive(Default)]
pub struct AllValidMoves(HashMap<Entity, Vec<Move>>);

impl AllValidMoves {
    pub fn get(&self, piece_id: Entity) -> &Vec<Move> {
        self.0
            .get(&piece_id)
            .expect("all pieces should have moves calculated")
    }

    pub fn insert(&mut self, piece_id: Entity, moves: Vec<Move>) {
        self.0.insert(piece_id, moves);
    }

    pub fn contains(&self, piece_id: Entity, square: Square) -> bool {
        self.get(piece_id).iter().any(|m| m.target_square == square)
    }
}

pub struct MovePiece(Vec3);

impl MovePiece {
    pub fn new(square: Square) -> Self {
        Self(square.to_translation())
    }

    pub fn target_translation(&self) -> Vec3 {
        self.0
    }

    pub fn target_square(&self) -> Square {
        Square::from_translation(self.0)
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

fn create_board(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    materials: ResMut<SquareMaterials>,
) {
    let mesh = meshes.add(Mesh::from(shape::Plane { size: 1.0 }));

    (0..8).for_each(|rank| {
        (0..8).for_each(|file| {
            let square = Square { rank: rank, file: file };

            commands
                .spawn_bundle(PbrBundle {
                    mesh: mesh.clone(),
                    material: materials.none.clone(),
                    transform: Transform::from_translation(square.to_translation()),
                    visible: Visible {
                        is_transparent: true,
                        is_visible: true,
                    },
                    ..Default::default()
                })
                .insert_bundle(PickableBundle::default())
                .insert(square);
        })
    })
}

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
            let piece = pieces.iter().find(|(_, piece)| {
                piece.square == *square && piece.colour == turn.0
            });

            if let Some((entity, _)) = piece {
                let valid_moves = valid_moves.get(entity);

                if !valid_moves.is_empty() {
                    *material = materials.valid_selection.clone();
                    return;
                }
            }
        };

        if let Some(promoted) = promoted_pawn.0 {
            let piece = pieces.iter().find(|(entity, piece)| {
                piece.square == *square && promoted == *entity
            });

            if let Some(_) = piece {
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

// TODO maybe implement this by drawing the highlight on top of the overlay
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

    let valid_moves = moves_calculator::calculate_valid_moves(
        player_turn.0,
        &special_move_data,
        player_pieces.as_slice(),
        opposite_pieces.as_slice(),
        board_state,
    );
    *all_moves = valid_moves;

    // FIXME doesn't handle stalemate properly
    if player_pieces
        .into_iter()
        .all(|(entity, _)| all_moves.get(entity).is_empty())
    {
        game_state.set(GameState::Checkmate(player_turn.0)).unwrap();
    };
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
        .find(|(_, piece)| piece.square == *square && piece.colour == turn.0)
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
        let maybe_valid_move = valid_moves
            .into_iter()
            .find(|m| m.target_square == *square);
        if let Some(valid_move) = maybe_valid_move {
            let (_, piece) = pieces.get_mut(piece_id).unwrap();
            let _ = special_move_data.last_pawn_double_step.take();

            if piece.kind == PieceKind::Pawn {
                if let MoveKind::EnPassant { target_id } = valid_move.kind {
                    commands.entity(target_id).insert(Taken);
                } else if valid_move.kind == MoveKind::PawnDoubleStep {
                    let _ = special_move_data
                        .last_pawn_double_step
                        .insert(LastPawnDoubleStep {
                            pawn_id: piece_id,
                            square: *square
                        });
                } else if valid_move.target_square.rank == player_turn.0.final_rank() {
                    promoted_pawn.0 = Some(piece_id);
                }
            } else if piece.kind == PieceKind::King {
                let mut castling_data = special_move_data.castling_data_mut(player_turn.0);
                castling_data.king_moved = true;

                if let MoveKind::Castle {
                    rook_id,
                    king_target_y,
                    rook_target_y,
                    kingside,
                } = valid_move.kind
                {
                    commands.entity(piece_id).insert(MovePiece::new((square.rank, king_target_y).into()));

                    commands.entity(rook_id).insert(MovePiece::new((square.rank, rook_target_y).into()));

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

            pieces
                .iter_mut()
                .find(|(_, other)| other.square == *square)
                .map(|(target_entity, target_piece)| {
                    if target_piece.kind == PieceKind::Rook {
                        let other_player = player_turn.0.opposite();
                        let mut castling_data = special_move_data.castling_data_mut(other_player);

                        if target_piece.square.rank == other_player.starting_back_rank() && target_piece.square.file == 0
                        {
                            castling_data.queenside_rook_moved = true;
                        } else if target_piece.square.rank == other_player.starting_back_rank()
                            && target_piece.square.file == 7
                        {
                            castling_data.kingside_rook_moved = true;
                        }
                    }

                    commands.entity(target_entity).insert(Taken);
                });

            commands.entity(piece_id).insert(MovePiece::new(*square));

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
    mut special_move_data: ResMut<SpecialMoveData>,
) {
    turn.0 = PieceColour::White;
    game_state.set(GameState::NothingSelected).unwrap();
    *special_move_data = Default::default();
}

struct SquareMaterials {
    highlight: Handle<StandardMaterial>,
    selected: Handle<StandardMaterial>,
    valid_selection: Handle<StandardMaterial>,
    none: Handle<StandardMaterial>,
}

impl FromWorld for SquareMaterials {
    fn from_world(world: &mut World) -> Self {
        let assets = world.get_resource::<AssetServer>().unwrap();
        let highlight = assets.load("textures/highlighted.png");
        let selected = assets.load("textures/selected.png");
        let valid_selection = assets.load("textures/valid_selection.png");

        let mut materials = world
            .get_resource_mut::<Assets<StandardMaterial>>()
            .unwrap();
        SquareMaterials {
            highlight: materials.add(highlight.into()),
            selected: materials.add(selected.into()),
            valid_selection: materials.add(valid_selection.into()),
            none: materials.add(Color::NONE.into()),
        }
    }
}
