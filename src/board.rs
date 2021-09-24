use bevy::prelude::*;
use bevy_mod_picking::{PickableBundle, PickingCamera};
use crate::pieces::Piece;

pub struct BoardPlugin;
impl Plugin for BoardPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.init_resource::<SquareMaterials>()
            .init_resource::<SelectedSquare>()
            .init_resource::<SelectedPiece>()
            .add_startup_system(create_board.system())
            .add_system(colour_squares.system())
            .add_system(select_square.system());
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

#[derive(Default)]
struct SelectedSquare(Option<Entity>);
#[derive(Default)]
struct SelectedPiece(Option<Entity>);

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
    mut pieces: Query<(Entity, &mut Piece)>
) {
    if !input.just_pressed(MouseButton::Left) {
        return;
    }

    if let Some((square_entity, _)) = pick_state.single().unwrap().intersect_top() {
        if let Ok(square) = squares.get(square_entity) {
            selected_square.0 = Some(square_entity);

            if let Some(piece) = selected_piece.0 {
                if let Ok((_, mut piece)) = pieces.get_mut(piece) {
                    piece.x = square.x;
                    piece.y = square.y;
                };
                selected_square.0 = None;
                selected_piece.0 = None;
            } else {
                for (piece_entity, piece) in pieces.iter_mut() {
                    if piece.x == square.x && piece.y == square.y {
                        selected_piece.0 = Some(piece_entity);
                        break;
                    }
                }
            }
        }
    } else {
        selected_square.0 = None;
        selected_piece.0 = None;
    };
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
