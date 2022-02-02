use crate::model::{Piece, PieceColour, PieceKind, Square};
use super::GameState;
use bevy::prelude::*;
use std::f32::consts::PI;
use bevy_mod_picking::PickableBundle;

pub struct GameSetUpPlugin;
impl Plugin for GameSetUpPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SquareMaterials>()
            .init_resource::<PieceMeshes>()
            .init_resource::<PieceMaterials>()
            .add_startup_system(create_board)
            .add_startup_system(create_floor_plane)
            .add_startup_system(create_pieces)
            .add_system_set(
                SystemSet::on_update(GameState::NewGame).with_system(reset_pieces),
            );
    }
}

fn reset_pieces(
    mut commands: Commands,
    meshes: Res<PieceMeshes>,
    materials: Res<PieceMaterials>,
    pieces: Query<Entity, With<Piece>>,
) {
    pieces.for_each(|entity| commands.entity(entity).despawn_recursive());
    create_pieces(commands, meshes, materials);
}

const SCALE_FACTOR: f32 = 15.0;

fn create_board(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    assets: Res<AssetServer>,
    materials: ResMut<SquareMaterials>,
) {
    let chessboard = assets.load("meshes/chessboard.glb#Scene0");

    let scale = Transform::from_scale(Vec3::splat(SCALE_FACTOR));
    let translation = Transform::from_xyz(0.0, -0.062 * SCALE_FACTOR, 0.0);
    let transform = translation * scale;

    commands
        .spawn_bundle((transform, GlobalTransform::identity()))
        .with_children(|parent| {
            parent.spawn_scene(chessboard);
        });

    let mesh = meshes.add(Mesh::from(shape::Plane { size: 1.0 }));

    (0..8).for_each(|rank| {
        (0..8).for_each(|file| {
            let square = Square { rank, file };

            // FIXME transparency
            commands
                .spawn_bundle(PbrBundle {
                    mesh: mesh.clone(),
                    material: materials.none.clone(),
                    transform: Transform::from_translation(square.to_translation()),
                    ..Default::default()
                })
                .insert_bundle(PickableBundle::default())
                .insert(square);
        })
    })
}

fn create_floor_plane(mut commands: Commands, assets: Res<AssetServer>) {
    // doesn't appear to support instancing
    let plane = assets.load("meshes/floor.glb#Scene0");
    let plane_size = 90.0;

    for x in -1..=1 {
        for y in -1..=1 {
            let translation =
                Transform::from_xyz(x as f32 * plane_size, -40.0, y as f32 * plane_size);

            commands
                .spawn_bundle((translation, GlobalTransform::identity()))
                .with_children(|parent| {
                    parent.spawn_scene(plane.clone());
                });
        }
    }
}

fn create_pieces(mut commands: Commands, meshes: Res<PieceMeshes>, materials: Res<PieceMaterials>) {
    [PieceColour::White, PieceColour::Black]
        .into_iter()
        .for_each(|colour| {
            let back_row = colour.starting_back_rank();
            let front_row = colour.starting_front_rank();

            [
                PieceKind::Rook,
                PieceKind::Knight,
                PieceKind::Bishop,
                PieceKind::Queen,
                PieceKind::King,
                PieceKind::Bishop,
                PieceKind::Knight,
                PieceKind::Rook,
            ]
                .into_iter()
                .enumerate()
                .for_each(|(file, kind)| {
                    spawn_piece(
                        &mut commands,
                        &materials,
                        &meshes,
                        colour,
                        kind,
                        (back_row, file as u8).into(),
                    );
                });

            (0..=7).for_each(|file| {
                spawn_piece(
                    &mut commands,
                    &materials,
                    &meshes,
                    colour,
                    PieceKind::Pawn,
                    (front_row, file).into(),
                );
            });
        });
}

pub fn spawn_piece(
    commands: &mut Commands,
    materials: &PieceMaterials,
    meshes: &PieceMeshes,
    colour: PieceColour,
    kind: PieceKind,
    square: Square,
) -> Entity {
    commands
        .spawn_bundle((place_on_square(colour, square), GlobalTransform::identity()))
        .insert(Piece {
            colour,
            kind,
            square,
        })
        .with_children(|parent| {
            parent.spawn_bundle(PbrBundle {
                mesh: meshes.get(kind),
                material: materials.get(colour),
                ..Default::default()
            });
        })
        .id()
}

fn place_on_square(colour: PieceColour, square: Square) -> Transform {
    let angle = if colour == PieceColour::Black {
        PI
    } else {
        0.0
    };

    let scale = Transform::from_scale(Vec3::splat(SCALE_FACTOR));
    let rotation = Transform::from_rotation(Quat::from_rotation_y(angle));

    let translation = Transform::from_translation(square.to_translation());

    translation * rotation * scale
}

pub struct SquareMaterials {
    pub highlight: Handle<StandardMaterial>,
    pub selected: Handle<StandardMaterial>,
    pub valid_selection: Handle<StandardMaterial>,
    pub none: Handle<StandardMaterial>,
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

pub struct PieceMeshes {
    king: Handle<Mesh>,
    pawn: Handle<Mesh>,
    knight: Handle<Mesh>,
    rook: Handle<Mesh>,
    bishop: Handle<Mesh>,
    queen: Handle<Mesh>,
}

impl PieceMeshes {
    pub fn get(&self, kind: PieceKind) -> Handle<Mesh> {
        match kind {
            PieceKind::King => self.king.clone(),
            PieceKind::Queen => self.queen.clone(),
            PieceKind::Bishop => self.bishop.clone(),
            PieceKind::Knight => self.knight.clone(),
            PieceKind::Rook => self.rook.clone(),
            PieceKind::Pawn => self.pawn.clone(),
        }
    }
}

impl FromWorld for PieceMeshes {
    fn from_world(world: &mut World) -> Self {
        let assets = world.get_resource::<AssetServer>().unwrap();
        Self {
            king: assets.load("meshes/chess pieces.glb#Mesh0/Primitive0"),
            pawn: assets.load("meshes/chess pieces.glb#Mesh1/Primitive0"),
            knight: assets.load("meshes/chess pieces.glb#Mesh2/Primitive0"),
            rook: assets.load("meshes/chess pieces.glb#Mesh3/Primitive0"),
            bishop: assets.load("meshes/chess pieces.glb#Mesh4/Primitive0"),
            queen: assets.load("meshes/chess pieces.glb#Mesh5/Primitive0"),
        }
    }
}

pub struct PieceMaterials {
    pub white: Handle<StandardMaterial>,
    pub black: Handle<StandardMaterial>,
}

impl PieceMaterials {
    pub fn get(&self, piece_colour: PieceColour) -> Handle<StandardMaterial> {
        match piece_colour {
            PieceColour::White => self.white.clone(),
            PieceColour::Black => self.black.clone(),
        }
    }
}

impl FromWorld for PieceMaterials {
    fn from_world(world: &mut World) -> Self {
        let mut materials = world
            .get_resource_mut::<Assets<StandardMaterial>>()
            .unwrap();
        let black = materials.add(Color::rgb(0.0, 0.2, 0.2).into());
        let white = materials.add(Color::rgb(1.0, 0.8, 0.8).into());

        Self { white, black }
    }
}
