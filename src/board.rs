use crate::pieces::{Piece, PieceColour, PieceKind};
use bevy::app::AppExit;
use bevy::prelude::*;
use bevy_mod_picking::{PickableBundle, PickingCamera};
use std::fmt::Formatter;

pub struct BoardPlugin;
impl Plugin for BoardPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.init_resource::<SquareMaterials>()
            .init_resource::<SelectedSquare>()
            .init_resource::<SelectedPiece>()
            .init_resource::<PlayerTurn>()
            .add_event::<ResetSelected>()
            .add_startup_system(create_board.system())
            .add_system(colour_squares.system())
            .add_system(select_square.system().label("select_square"))
            .add_system(
                move_piece
                    .system()
                    .after("select_square")
                    .before("select_piece"),
            )
            .add_system(
                select_piece
                    .system()
                    .after("select_square")
                    .label("select_piece"),
            )
            .add_system(reset_selected.system().after("select_square"))
            .add_system(despawn_taken_pieces.system());
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

struct ResetSelected;
struct Taken;

#[derive(Default)]
struct SelectedSquare(Option<Entity>);
#[derive(Default)]
struct SelectedPiece(Option<Entity>);

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

impl core::fmt::Display for PlayerTurn {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "{}",
            match self.0 {
                PieceColour::White => "White",
                PieceColour::Black => "Black",
            }
        )
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
    materials: Res<SquareMaterials>,
    pick_state: Query<&PickingCamera>,
    mut query: Query<(Entity, &Square, &mut Handle<StandardMaterial>)>,
) {
    let top_entity = if let Some((entity, _)) = pick_state.single().unwrap().intersect_top() {
        Some(entity)
    } else {
        None
    };

    for (entity, square, mut material) in query.iter_mut() {
        *material = if top_entity == Some(entity) {
            materials.highlight.clone()
        } else if Some(entity) == selected_square.0 {
            materials.selected.clone()
        } else if square.is_white() {
            materials.white.clone()
        } else {
            materials.black.clone()
        }
    }
}

fn select_square(
    input: Res<Input<MouseButton>>,
    mut selected_square: ResMut<SelectedSquare>,
    mut selected_piece: ResMut<SelectedPiece>,
    pick_state: Query<&PickingCamera>,
    squares: Query<&Square>,
) {
    if !input.just_pressed(MouseButton::Left) {
        return;
    }

    if let Some((square_entity, _)) = pick_state.single().unwrap().intersect_top() {
        if squares.get(square_entity).is_ok() {
            selected_square.0 = Some(square_entity);
        }
    } else {
        selected_square.0 = None;
        selected_piece.0 = None;
    };
}

fn select_piece(
    selected_square: Res<SelectedSquare>,
    mut selected_piece: ResMut<SelectedPiece>,
    turn: Res<PlayerTurn>,
    squares: Query<&Square>,
    pieces: Query<(Entity, &Piece)>,
) {
    if !selected_square.is_changed() {
        return;
    }

    let square = if let Some(entity) = selected_square.0 {
        if let Ok(square) = squares.get(entity) {
            square
        } else {
            return;
        }
    } else {
        return;
    };

    if selected_piece.0.is_none() {
        pieces
            .iter()
            .find(|(_, piece)| piece.x == square.x && piece.y == square.y && piece.colour == turn.0)
            .map(|(entity, _)| selected_piece.0 = Some(entity));
    };
}

fn move_piece(
    mut commands: Commands,
    selected_square: Res<SelectedSquare>,
    selected_piece: Res<SelectedPiece>,
    mut turn: ResMut<PlayerTurn>,
    squares: Query<&Square>,
    mut pieces: Query<(Entity, &mut Piece)>,
    mut reset_selected_events: EventWriter<ResetSelected>,
) {
    if !selected_square.is_changed() {
        return;
    }

    let square = if let Some(entity) = selected_square.0 {
        if let Ok(square) = squares.get(entity) {
            square
        } else {
            return;
        }
    } else {
        return;
    };

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

            turn.next()
        };

        reset_selected_events.send(ResetSelected);
    }
}

fn reset_selected(
    mut reader: EventReader<ResetSelected>,
    mut selected_square: ResMut<SelectedSquare>,
    mut selected_piece: ResMut<SelectedPiece>,
) {
    reader.iter().for_each(|_| {
        selected_square.0 = None;
        selected_piece.0 = None;
    })
}

fn despawn_taken_pieces(
    mut commands: Commands,
    mut exit_events: EventWriter<AppExit>,
    query: Query<(Entity, &Piece, &Taken)>,
) {
    query.for_each(|(entity, piece, _)| {
        if piece.kind == PieceKind::King {
            println!(
                "{} won",
                match piece.colour {
                    PieceColour::White => "Black",
                    PieceColour::Black => "White",
                }
            );
            exit_events.send(AppExit);
        }

        commands.entity(entity).despawn_recursive();
    })
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
