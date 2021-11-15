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
            .init_resource::<PlayerTurn>()
            .init_resource::<AllValidMoves>()
            .init_resource::<Option<HighlightedSquare>>()
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

#[derive(Default)]
struct SelectedSquare(Option<Entity>);
#[derive(Default)]
struct SelectedPiece(Option<Entity>);

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
    // todo should split this into once-per-turn state and actual "Nothing selected" state
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
    mut all_moves: ResMut<AllValidMoves>,
    game_state: ResMut<State<GameState>>,
    pieces: Query<(Entity, &Piece)>,
) {
    let board_state = pieces.iter().map(|(_, piece)| piece).collect();

    // note: this calculates all potential moves for both sides - this makes it easier to check for check(mate)
    pieces.iter().for_each(|(entity, piece)| {
        let valid_moves = piece.valid_moves(&board_state);
        let _ = all_moves.insert(entity, valid_moves);
    });

    check_check(player_turn, all_moves, game_state, board_state, pieces);
}

fn check_check(
    player_turn: Res<PlayerTurn>,
    mut all_moves: ResMut<AllValidMoves>,
    mut game_state: ResMut<State<GameState>>,
    board_state: BoardState,
    pieces: Query<(Entity, &Piece)>,
) {
    let (player_pieces, opposite_pieces): (Vec<_>, Vec<_>) = pieces
        .iter()
        .partition(|(_, piece)| piece.colour == player_turn.0);

    let (king_entity, king) = player_pieces
        .iter()
        .find(|(_, piece)| piece.colour == player_turn.0 && piece.kind == PieceKind::King)
        .expect("there should always be 2 kings");

    let entities_threatening_king = all_moves
        .0
        .iter()
        // find opposite colour pieces that will be able to take the king next turn
        .filter_map(|(entity, moves)| moves.contains(&(king.x, king.y)).then(|| entity))
        .collect::<Vec<_>>();

    let safe_king_moves = all_moves
        .get(*king_entity)
        .into_iter()
        .filter(|(x, y)| {
            !opposite_pieces
                .iter()
                .any(|(entity, _)| all_moves.get(*entity).contains(&(*x, *y)))
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

    let safe_player_moves = player_pieces
        .iter()
        .filter(|(entity, _)| entity != king_entity)
        .map(|(entity, piece)| {
            let safe_moves = all_moves
                .get(*entity)
                .iter()
                .filter(|(x, y)| {
                    // safe move iff: doesn't open up a path to the king, or stays within the same path
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

    if !entities_threatening_king.is_empty() {
        let counter_moves: Vec<(Entity, Vec<(u8, u8)>)> =
            std::iter::once((*king_entity, safe_king_moves))
                .chain(
                    safe_player_moves
                        .iter()
                        .filter(|(entity, _)| *entity != king_entity)
                        .map(|(entity, safe_moves)| {
                            let counter_moves = safe_moves
                                .iter()
                                .filter(|(x, y)| {
                                    entities_threatening_king.iter().all(|opposite_entity| {
                                        let opposite_piece = opposite_pieces
                                            .iter()
                                            .find_map(|(entity, piece)| {
                                                (entity == *opposite_entity).then(|| piece)
                                            })
                                            .unwrap();

                                        (opposite_piece.x == *x && opposite_piece.y == *y)
                                            || opposite_piece
                                                .path_to_take_piece_at((king.x, king.y))
                                                .contains(&(*x, *y))
                                    })
                                })
                                .copied()
                                .collect::<Vec<_>>();

                            (**entity, counter_moves)
                        }),
                )
                .collect();

        if counter_moves.iter().all(|(_, moves)| moves.is_empty()) {
            game_state.set(GameState::Checkmate(player_turn.0)).unwrap();
        }

        counter_moves.into_iter().for_each(|(entity, moves)| {
            let _ = all_moves.insert(entity, moves);
        });
    } else {
        let _ = all_moves.insert(*king_entity, safe_king_moves);
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

fn move_piece(
    mut commands: Commands,
    selected_square: Res<SelectedSquare>,
    selected_piece: Res<SelectedPiece>,
    all_valid_moves: Res<AllValidMoves>,
    mut game_state: ResMut<State<GameState>>,
    squares: Query<&Square>,
    mut pieces: Query<(Entity, &mut Piece)>,
) {
    let square = if let Some(entity) = selected_square.0 {
        squares.get(entity).unwrap()
    } else {
        return;
    };

    if let Some(piece) = selected_piece.0 {
        let valid_moves = all_valid_moves.get(piece);
        if valid_moves.contains(&(square.x, square.y)) {
            pieces
                .iter_mut()
                .filter(|(_, other)| other.x == square.x && other.y == square.y)
                .for_each(|(other_entity, _)| {
                    commands.entity(other_entity).insert(Taken);
                });

            commands.entity(piece).insert(MovePiece {
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

fn start_new_game(mut game_state: ResMut<State<GameState>>, mut turn: ResMut<PlayerTurn>) {
    turn.0 = PieceColour::White;

    game_state.set(GameState::NothingSelected).unwrap();
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
