use crate::pieces::{Piece, PieceColour, PieceKind};
use bevy::prelude::*;
use bevy_mod_picking::{PickableBundle, PickingCamera};
use std::borrow::Borrow;
use std::fmt::Formatter;

pub struct BoardPlugin;
impl Plugin for BoardPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.init_resource::<SquareMaterials>()
            .init_resource::<SelectedSquare>()
            .init_resource::<SelectedPiece>()
            .init_resource::<PlayerTurn>()
            .init_resource::<ValidMoves>()
            .add_event::<Reset>()
            .add_state(GameState::NothingSelected)
            .add_startup_system(create_board.system())
            .add_system(colour_squares.system())
            .add_system(reset_game.system())
            .add_system_set(
                SystemSet::on_enter(GameState::NothingSelected)
                    .with_system(reset_selected.system()),
            )
            .add_system_set(
                SystemSet::on_update(GameState::NothingSelected)
                    .with_system(select_square.system()),
            )
            .add_system_set(
                SystemSet::on_update(GameState::SquareSelected).with_system(select_piece.system()),
            )
            .add_system_set(
                SystemSet::on_enter(GameState::PieceSelected)
                    .with_system(calculate_valid_moves.system()),
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

#[derive(Debug)]
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
        let mut squares = [None; 64];
        pieces.iter().for_each(|piece| {
            squares[(piece.x * 8 + piece.y) as usize] = Some(piece.colour);
        });

        Self { squares }
    }
}

impl<const N: usize> From<[Piece; N]> for BoardState {
    fn from(pieces: [Piece; N]) -> Self {
        Self::from(&pieces[..])
    }
}

struct Square {
    pub x: u8,
    pub y: u8,
}

impl Square {
    fn is_white(&self) -> bool {
        (self.x + self.y + 1) % 2 == 0
    }
}

struct Taken;
pub struct Reset;

// todo try tagging square/piece entity and using it as a query filter
#[derive(Default)]
struct SelectedSquare(Option<Entity>);
#[derive(Default)]
struct SelectedPiece(Option<Entity>);
#[derive(Default)]
struct ValidMoves(Vec<(u8, u8)>);

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum GameState {
    NothingSelected,
    SquareSelected,
    PieceSelected,
    TargetSquareSelected,
    Checkmate(PieceColour),
}

impl core::fmt::Display for GameState {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            GameState::NothingSelected | GameState::SquareSelected => {
                write!(f, "Select a piece to move")
            }
            GameState::PieceSelected => write!(f, "Select a target square"),
            GameState::TargetSquareSelected => write!(f, "Moving piece to target square"),
            // TODO should stop the game when the King is in checkmate, not when the King has been taken
            GameState::Checkmate(colour) => {
                write!(f, "{}'s King has been captured\nPress R to restart", colour)
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
        self.0 = match self.0 {
            PieceColour::White => PieceColour::Black,
            PieceColour::Black => PieceColour::White,
        }
    }
}

fn create_board(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    materials: ResMut<SquareMaterials>,
) {
    let mesh = meshes.add(Mesh::from(shape::Plane { size: 1.0 }));

    (0..8)
        .into_iter()
        .map(|x| {
            (0..8).into_iter().for_each(|y| {
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
        .collect()
}

fn colour_squares(
    selected_square: Res<SelectedSquare>,
    valid_moves: Res<ValidMoves>,
    materials: Res<SquareMaterials>,
    pick_state: Query<&PickingCamera>,
    mut query: Query<(Entity, &Square, &mut Handle<StandardMaterial>)>,
) {
    let top_entity = selected_entity(pick_state);

    for (entity, square, mut material) in query.iter_mut() {
        *material = if top_entity == Some(entity) {
            materials.highlight.clone()
        } else if Some(entity) == selected_square.0 {
            materials.selected.clone()
        } else if valid_moves.0.contains(&(square.x, square.y)) {
            materials.highlight.clone()
        } else if square.is_white() {
            materials.white.clone()
        } else {
            materials.black.clone()
        }
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
    // FIXME boilerplate
    let square = if let Some(entity) = selected_square.0 {
        if let Ok(square) = squares.get(entity) {
            square
        } else {
            return;
        }
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

fn calculate_valid_moves(
    mut valid_moves: ResMut<ValidMoves>,
    selected_piece: Res<SelectedPiece>,
    pieces: Query<&Piece>,
) {
    let piece = if let Some(entity) = selected_piece.0 {
        if let Ok(piece) = pieces.get(entity) {
            piece
        } else {
            return;
        }
    } else {
        return;
    };

    let pieces = pieces.iter().cloned().collect::<Vec<_>>();
    let moves = piece.valid_moves(&BoardState::from(pieces.borrow()));

    valid_moves.0 = moves;
}

fn move_piece(
    mut commands: Commands,
    selected_square: Res<SelectedSquare>,
    selected_piece: Res<SelectedPiece>,
    mut turn: ResMut<PlayerTurn>,
    mut game_state: ResMut<State<GameState>>,
    squares: Query<&Square>,
    mut pieces: Query<(Entity, &mut Piece)>,
) {
    let square = if let Some(entity) = selected_square.0 {
        if let Ok(square) = squares.get(entity) {
            square
        } else {
            return;
        }
    } else {
        return;
    };

    // FIXME messy
    if let Some(piece) = selected_piece.0 {
        let piece_entities_copy = pieces
            .iter_mut()
            .map(|(entity, piece)| (entity, *piece))
            .collect::<Vec<_>>();
        let pieces_copy = pieces
            .iter_mut()
            .map(|(_, piece)| *piece)
            .collect::<Vec<_>>();

        let mut piece = if let Ok((_, piece)) = pieces.get_mut(piece) {
            piece
        } else {
            return;
        };

        if piece.valid_move((square.x, square.y), &pieces_copy) {
            piece_entities_copy
                .into_iter()
                .filter(|(_, other)| other.x == square.x && other.y == square.y)
                .for_each(|(other_entity, _)| {
                    commands.entity(other_entity).insert(Taken);
                });

            piece.x = square.x;
            piece.y = square.y;

            // TODO don't change turn until movement completed
            game_state.set(GameState::NothingSelected).unwrap();
            turn.next()
        } else {
            game_state.set(GameState::PieceSelected).unwrap();
        };
    }
}

fn reset_selected(
    mut selected_square: ResMut<SelectedSquare>,
    mut selected_piece: ResMut<SelectedPiece>,
    mut valid_moves: ResMut<ValidMoves>,
) {
    selected_square.0 = None;
    selected_piece.0 = None;
    valid_moves.0 = vec![];
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

fn reset_game(
    input: Res<Input<KeyCode>>,
    mut reset_events: EventWriter<Reset>,
    mut state: ResMut<State<GameState>>,
    mut turn: ResMut<PlayerTurn>,
) {
    if input.just_pressed(KeyCode::R) {
        if state.current() != &GameState::NothingSelected {
            state.set(GameState::NothingSelected).unwrap();
        }

        turn.0 = PieceColour::White;
        reset_events.send(Reset);
    }
}

struct SquareMaterials {
    highlight: Handle<StandardMaterial>,
    selected: Handle<StandardMaterial>,
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
            black: materials.add(Color::rgb(0.0, 0.1, 0.1).into()),
            white: materials.add(Color::rgb(1.0, 0.9, 0.9).into()),
        }
    }
}
