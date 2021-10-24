use crate::board::Reset;
use bevy::prelude::*;
use std::f32::consts::{FRAC_PI_2, PI};
use std::fmt::Formatter;

pub struct PiecePlugin;
impl Plugin for PiecePlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.init_resource::<PieceMeshes>()
            .init_resource::<PieceMaterials>()
            .add_startup_system(create_pieces.system())
            .add_system(move_pieces.system())
            .add_system(reset_pieces.system());
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Piece {
    pub colour: PieceColour,
    pub kind: PieceKind,
    pub x: u8,
    pub y: u8,
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

impl Piece {
    pub fn valid_move(&self, new_position: (u8, u8), pieces: &[Piece]) -> bool {
        let new_x = new_position.0 as i8;
        let new_y = new_position.1 as i8;
        let x = self.x as i8;
        let y = self.y as i8;

        if square_colour(new_position, pieces) == Some(self.colour) {
            return false;
        };

        match self.kind {
            PieceKind::King => {
                // todo can probably simplify this
                let valid_horizontal = (x - new_x).abs() == 1 && y == new_y;
                let valid_vertical = (y - new_y).abs() == 1 && x == new_x;
                let valid_diagonal = (x - new_x).abs() == 1 && (y - new_y).abs() == 1;

                valid_horizontal || valid_vertical || valid_diagonal
            }
            PieceKind::Queen => {
                let valid_diagonal = (x - new_x).abs() == (y - new_y).abs();
                let valid_vertical = x == new_x && y != new_y;
                let valid_horizontal = y == new_y && x != new_x;

                path_empty((self.x, self.y), new_position, pieces)
                    && (valid_diagonal || valid_vertical || valid_horizontal)
            }
            PieceKind::Bishop => {
                path_empty((self.x, self.y), new_position, pieces)
                    && (x - new_x).abs() == (y - new_y).abs()
            }
            PieceKind::Knight => {
                let valid_horizontal = (x - new_x).abs() == 1 && (y - new_y).abs() == 2;
                let valid_vertical = (y - new_y).abs() == 1 && (x - new_x).abs() == 2;

                valid_horizontal || valid_vertical
            }
            PieceKind::Rook => {
                let valid_vertical = x == new_x && y != new_y;
                let valid_horizontal = y == new_y && x != new_x;

                path_empty((self.x, self.y), new_position, pieces)
                    && (valid_vertical || valid_horizontal)
            }
            PieceKind::Pawn if self.colour == PieceColour::White => {
                if new_x - x == 1 && new_y == y {
                    return square_colour(new_position, pieces).is_none();
                }

                if x == 1
                    && new_x == 3
                    && y == new_y
                    && path_empty((self.x, self.y), new_position, pieces)
                {
                    return square_colour(new_position, pieces).is_none();
                }

                if new_x - x == 1 && (new_y - y).abs() == 1 {
                    return square_colour(new_position, pieces) == Some(PieceColour::Black);
                }

                false
            }
            PieceKind::Pawn => {
                if new_x - x == -1 && new_y == y {
                    return square_colour(new_position, pieces).is_none();
                }

                if x == 6
                    && new_x == 4
                    && y == new_y
                    && path_empty((self.x, self.y), new_position, pieces)
                {
                    return square_colour(new_position, pieces).is_none();
                }

                if new_x - x == -1 && (new_y - y).abs() == 1 {
                    return square_colour(new_position, pieces) == Some(PieceColour::White);
                }

                false
            }
        }
    }
}

const VELOCITY: f32 = 7.0;
// TODO acceleration; y movement - Bezier curve maybe?
fn move_pieces(time: Res<Time>, mut query: Query<(&mut Transform, &Piece)>) {
    for (mut transform, piece) in query.iter_mut() {
        let direction = Vec3::new(piece.x as f32, 0.0, piece.y as f32) - transform.translation;

        if direction.length() > f32::EPSILON * 2.0 {
            let delta = VELOCITY * (direction.normalize() * time.delta_seconds());
            if delta.length() > direction.length() {
                transform.translation += direction;
            } else {
                transform.translation += delta;
            }
        }
    }
}

fn reset_pieces(
    mut commands: Commands,
    mut reset_events: EventReader<Reset>,
    meshes: Res<PieceMeshes>,
    materials: Res<PieceMaterials>,
    pieces: Query<Entity, With<Piece>>,
) {
    // awkward way to consume all events (although there should only be 1 or 0) then reset pieces if
    // there were any events received

    if reset_events.iter().count() != 0 {
        pieces.for_each(|entity| commands.entity(entity).despawn_recursive());
        create_pieces(commands, meshes, materials);
    }
}

fn create_pieces(
    mut commands: Commands,
    meshes: Res<PieceMeshes>,
    materials: Res<PieceMaterials>,
) {
    spawn_side(
        &mut commands,
        &meshes,
        materials.white.clone(),
        PieceColour::White,
    );
    spawn_side(
        &mut commands,
        &meshes,
        materials.black.clone(),
        PieceColour::Black,
    );
}

fn path_empty(from: (u8, u8), to: (u8, u8), pieces: &[Piece]) -> bool {
    let (start_x, start_y) = from;
    let (end_x, end_y) = to;

    // same column
    if start_x == end_x {
        return pieces
            .iter()
            .filter(|piece| piece.x == start_x)
            .filter(|piece| piece.y != start_y && piece.y != end_y)
            .all(|piece| {
                // piece is after path ends
                (piece.y > start_y && piece.y > end_y) ||
                    // piece is before path starts
                    (piece.y < start_y && piece.y < end_y)
            });
    };

    // same row
    if start_y == end_y {
        return pieces
            .iter()
            .filter(|piece| piece.y == start_y)
            .filter(|piece| piece.x != start_x && piece.x != end_x)
            .all(|piece| {
                // piece is after path ends
                (piece.x > start_x && piece.x > end_x) ||
                    // piece is before path starts
                    (piece.x < start_x && piece.x < end_x)
            });
    }

    let x_diff = (start_x as i8 - end_x as i8).abs();
    let y_diff = (start_y as i8 - end_y as i8).abs();

    // diagonal
    if x_diff == y_diff {
        return (1..x_diff).into_iter().all(|idx| {
            let idx = idx as u8;
            let (x, y) = if start_x < end_x && start_y < end_y {
                // bottom left -> top right
                (start_x + idx, start_y + idx)
            } else if start_x < end_x {
                // top left -> bottom right
                (start_x + idx, start_y - idx)
            } else if start_y < end_y {
                // bottom right -> top left
                (start_x - idx, start_y + idx)
            } else {
                // top right -> bottom left
                (start_x - idx, start_y - idx)
            };

            square_colour((x, y), pieces).is_none()
        });
    }

    true
}

fn square_colour((x, y): (u8, u8), pieces: &[Piece]) -> Option<PieceColour> {
    pieces
        .iter()
        .find_map(|piece| (piece.x == x && piece.y == y).then(|| piece.colour))
}

fn spawn_side(
    commands: &mut Commands,
    meshes: &PieceMeshes,
    material: Handle<StandardMaterial>,
    colour: PieceColour,
) {
    let back_row = if colour == PieceColour::White { 0 } else { 7 };
    let front_row = if colour == PieceColour::White { 1 } else { 6 };

    spawn_rook(
        commands,
        material.clone(),
        meshes.rook.clone(),
        colour,
        (back_row, 0),
    );
    spawn_knight(
        commands,
        material.clone(),
        meshes.knight_base.clone(),
        meshes.knight.clone(),
        colour,
        (back_row, 1),
    );
    spawn_bishop(
        commands,
        material.clone(),
        meshes.bishop.clone(),
        colour,
        (back_row, 2),
    );
    spawn_queen(commands, material.clone(), meshes.queen.clone(), colour, (back_row, 3));
    spawn_king(
        commands,
        material.clone(),
        meshes.king.clone(),
        meshes.king_cross.clone(),
        colour,
        (back_row, 4),
    );
    spawn_bishop(commands, material.clone(), meshes.bishop.clone(), colour, (back_row, 5));
    spawn_knight(
        commands,
        material.clone(),
        meshes.knight_base.clone(),
        meshes.knight.clone(),
        colour,
        (back_row, 6),
    );
    spawn_rook(commands, material.clone(), meshes.rook.clone(), colour, (back_row, 7));

    (0..=7).into_iter().for_each(|idx| {
        spawn_pawn(
            commands,
            material.clone(),
            meshes.pawn.clone(),
            colour,
            (front_row, idx),
        )
    })
}

fn spawn_king(
    commands: &mut Commands,
    material: Handle<StandardMaterial>,
    king: Handle<Mesh>,
    king_cross: Handle<Mesh>,
    colour: PieceColour,
    (x, y): (u8, u8),
) {
    commands
        .spawn_bundle(PbrBundle {
            transform: place_on_square(colour, x, y),
            ..Default::default()
        })
        .insert(Piece {
            colour,
            kind: PieceKind::King,
            x,
            y,
        })
        .with_children(|parent| {
            parent.spawn_bundle(PbrBundle {
                mesh: king.clone(),
                material: material.clone(),
                transform: {
                    let mut transform = Transform::from_translation(Vec3::new(-0.2, 0.0, -1.9));
                    transform.apply_non_uniform_scale(Vec3::new(0.2, 0.2, 0.2));
                    transform
                },
                ..Default::default()
            });

            parent.spawn_bundle(PbrBundle {
                mesh: king_cross.clone(),
                material,
                transform: {
                    let mut transform = Transform::from_translation(Vec3::new(-0.2, 0.0, -1.9));
                    transform.apply_non_uniform_scale(Vec3::new(0.2, 0.2, 0.2));
                    transform
                },
                ..Default::default()
            });
        });
}

fn spawn_knight(
    commands: &mut Commands,
    material: Handle<StandardMaterial>,
    knight_base: Handle<Mesh>,
    knight: Handle<Mesh>,
    colour: PieceColour,
    (x, y): (u8, u8),
) {
    commands
        .spawn_bundle(PbrBundle {
            transform: place_on_square(colour, x, y),
            ..Default::default()
        })
        .insert(Piece {
            colour,
            kind: PieceKind::Knight,
            x,
            y,
        })
        .with_children(|parent| {
            parent.spawn_bundle(PbrBundle {
                mesh: knight_base,
                material: material.clone(),
                transform: {
                    let mut transform = Transform::from_translation(Vec3::new(-0.2, 0.0, 0.9));
                    transform.apply_non_uniform_scale(Vec3::new(0.2, 0.2, 0.2));
                    transform
                },
                ..Default::default()
            });

            parent.spawn_bundle(PbrBundle {
                mesh: knight,
                material,
                transform: {
                    let mut transform = Transform::from_translation(Vec3::new(-0.2, 0.0, 0.9));
                    transform.apply_non_uniform_scale(Vec3::new(0.2, 0.2, 0.2));
                    transform
                },
                ..Default::default()
            });
        });
}

fn spawn_queen(
    commands: &mut Commands,
    material: Handle<StandardMaterial>,
    queen: Handle<Mesh>,
    colour: PieceColour,
    (x, y): (u8, u8),
) {
    commands
        .spawn_bundle(PbrBundle {
            transform: place_on_square(colour, x, y),
            ..Default::default()
        })
        .insert(Piece {
            colour,
            kind: PieceKind::Queen,
            x,
            y,
        })
        .with_children(|parent| {
            parent.spawn_bundle(PbrBundle {
                mesh: queen,
                material,
                transform: {
                    let mut transform = Transform::from_translation(Vec3::new(-0.2, 0.0, -0.95));
                    transform.apply_non_uniform_scale(Vec3::new(0.2, 0.2, 0.2));
                    transform
                },
                ..Default::default()
            });
        });
}

fn spawn_bishop(
    commands: &mut Commands,
    material: Handle<StandardMaterial>,
    bishop: Handle<Mesh>,
    colour: PieceColour,
    (x, y): (u8, u8),
) {
    commands
        .spawn_bundle(PbrBundle {
            transform: place_on_square(colour, x, y),
            ..Default::default()
        })
        .insert(Piece {
            colour,
            kind: PieceKind::Bishop,
            x,
            y,
        })
        .with_children(|parent| {
            parent.spawn_bundle(PbrBundle {
                mesh: bishop,
                material,
                transform: {
                    // FIXME wrong direction because of rotation (black side)
                    let mut transform = Transform::from_translation(Vec3::new(-0.1, 0.0, 0.0));
                    transform.apply_non_uniform_scale(Vec3::new(0.2, 0.2, 0.2));
                    transform.rotate(Quat::from_rotation_y(-FRAC_PI_2));
                    transform
                },
                ..Default::default()
            });
        });
}

fn spawn_rook(
    commands: &mut Commands,
    material: Handle<StandardMaterial>,
    rook: Handle<Mesh>,
    colour: PieceColour,
    (x, y): (u8, u8),
) {
    commands
        .spawn_bundle(PbrBundle {
            transform: place_on_square(colour, x, y),
            ..Default::default()
        })
        .insert(Piece {
            colour,
            kind: PieceKind::Rook,
            x,
            y,
        })
        .with_children(|parent| {
            parent.spawn_bundle(PbrBundle {
                mesh: rook,
                material,
                transform: {
                    let mut transform = Transform::from_translation(Vec3::new(-0.1, 0.0, 1.8));
                    transform.apply_non_uniform_scale(Vec3::new(0.2, 0.2, 0.2));
                    transform
                },
                ..Default::default()
            });
        });
}

fn spawn_pawn(
    commands: &mut Commands,
    material: Handle<StandardMaterial>,
    pawn: Handle<Mesh>,
    colour: PieceColour,
    (x, y): (u8, u8),
) {
    commands
        .spawn_bundle(PbrBundle {
            transform: place_on_square(colour, x, y),
            ..Default::default()
        })
        .insert(Piece {
            colour,
            kind: PieceKind::Pawn,
            x,
            y,
        })
        .with_children(|parent| {
            parent.spawn_bundle(PbrBundle {
                mesh: pawn,
                material,
                transform: {
                    let mut transform = Transform::from_translation(Vec3::new(-0.2, 0.0, 2.6));
                    transform.apply_non_uniform_scale(Vec3::new(0.2, 0.2, 0.2));
                    transform
                },
                ..Default::default()
            });
        });
}

fn place_on_square(colour: PieceColour, x: u8, y: u8) -> Transform {
    let angle = if colour == PieceColour::Black {
        PI
    } else {
        0.0
    };

    let rotation = Transform::from_rotation(Quat::from_rotation_y(angle));
    let translation = Transform::from_translation(Vec3::new(x as f32, 0.0, y as f32));

    translation * rotation
}

struct PieceMeshes {
    king: Handle<Mesh>,
    king_cross: Handle<Mesh>,
    pawn: Handle<Mesh>,
    knight_base: Handle<Mesh>,
    knight: Handle<Mesh>,
    rook: Handle<Mesh>,
    bishop: Handle<Mesh>,
    queen: Handle<Mesh>,
}

impl FromWorld for PieceMeshes {
    fn from_world(world: &mut World) -> Self {
        let assets = world.get_resource::<AssetServer>().unwrap();
        Self {
            king: assets.load("meshes/pieces.glb#Mesh0/Primitive0"),
            king_cross: assets.load("meshes/pieces.glb#Mesh1/Primitive0"),
            pawn: assets.load("meshes/pieces.glb#Mesh2/Primitive0"),
            knight_base: assets.load("meshes/pieces.glb#Mesh3/Primitive0"),
            knight: assets.load("meshes/pieces.glb#Mesh4/Primitive0"),
            rook: assets.load("meshes/pieces.glb#Mesh5/Primitive0"),
            bishop: assets.load("meshes/pieces.glb#Mesh6/Primitive0"),
            queen: assets.load("meshes/pieces.glb#Mesh7/Primitive0"),
        }
    }
}

struct PieceMaterials {
    white: Handle<StandardMaterial>,
    black: Handle<StandardMaterial>,
}

impl FromWorld for PieceMaterials {
    fn from_world(world: &mut World) -> Self {
        let mut materials = world
            .get_resource_mut::<Assets<StandardMaterial>>()
            .unwrap();
        let black = materials.add(Color::rgb(0.0, 0.2, 0.2).into());
        let white = materials.add(Color::rgb(1.0, 0.8, 0.8).into());

        Self {
            white,
            black
        }
    }
}