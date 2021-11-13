use crate::board::{BoardState, GameState, MovePiece, PlayerTurn};
use bevy::prelude::*;
use std::f32::consts::{FRAC_PI_2, PI};
use std::fmt::Formatter;

pub struct PiecePlugin;
impl Plugin for PiecePlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.init_resource::<PieceMeshes>()
            .init_resource::<PieceMaterials>()
            .add_startup_system(create_pieces.system())
            .add_system_set(
                SystemSet::on_update(GameState::NewGame).with_system(reset_pieces.system()),
            )
            .add_system_set(
                SystemSet::on_update(GameState::MovingPiece).with_system(move_pieces.system()),
            );
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

impl PieceColour {
    pub fn opposite(&self) -> Self {
        match self {
            PieceColour::White => PieceColour::Black,
            PieceColour::Black => PieceColour::White,
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

impl Piece {
    pub fn valid_moves(&self, board: &BoardState) -> Vec<(u8, u8)> {
        let (x, y) = (self.x as i8, self.y as i8);

        let is_on_board = |(x, y): (i8, i8)| {
            ((0..8).contains(&x) && (0..8).contains(&y)).then(|| (x as u8, y as u8))
        };

        let path_clear = |(x, y): &(u8, u8)| self.path_empty((*x, *y), board);
        let not_occupied_by_same_colour =
            |(x, y): &(u8, u8)| board.get(*x, *y) != &Some(self.colour);

        let diagonals = (-7..=7)
            .filter(|adj| *adj != 0)
            .flat_map(|adj| [(x + adj, y + adj), (x - adj, y + adj)].into_iter())
            .filter_map(is_on_board)
            .filter(not_occupied_by_same_colour)
            .filter(path_clear);

        let straight_lines = (-7..=7)
            .filter(|adj| *adj != 0)
            .map(|adj| (x + adj, y))
            .chain((-7..=7).filter(|adj| *adj != 0).map(|adj| (x, y + adj)))
            .filter_map(is_on_board)
            .filter(not_occupied_by_same_colour)
            .filter(path_clear);

        match self.kind {
            PieceKind::King => [
                (x - 1, y - 1),
                (x - 1, y),
                (x - 1, y + 1),
                (x, y - 1),
                (x, y + 1),
                (x + 1, y - 1),
                (x + 1, y),
                (x + 1, y + 1),
            ]
            .into_iter()
            .filter_map(is_on_board)
            .filter(not_occupied_by_same_colour)
            .collect(),
            PieceKind::Queen => diagonals.chain(straight_lines).collect(),
            PieceKind::Bishop => diagonals.collect(),
            PieceKind::Knight => [
                (x - 2, y - 1),
                (x - 2, y + 1),
                (x + 2, y - 1),
                (x + 2, y + 1),
                (x - 1, y - 2),
                (x - 1, y + 2),
                (x + 1, y - 2),
                (x + 1, y + 2),
            ]
            .into_iter()
            .filter_map(is_on_board)
            .filter(not_occupied_by_same_colour)
            .collect(),
            PieceKind::Rook => straight_lines.collect(),
            PieceKind::Pawn => {
                let (starting_row, final_row, direction) = if self.colour == PieceColour::White {
                    (1, 7, 1)
                } else {
                    (6, 0, -1)
                };

                if x == final_row {
                    vec![]
                } else {
                    let mut moves = Vec::with_capacity(4);

                    let move_one = (x + direction) as u8;
                    let move_two = (x + (2 * direction)) as u8;
                    let y = y as u8;

                    if board.get(move_one, y).is_none() {
                        moves.push((move_one, y));

                        if x == starting_row && board.get(move_two, y).is_none() {
                            moves.push((move_two, y));
                        }
                    };

                    let opposite_colour = if self.colour == PieceColour::White {
                        PieceColour::Black
                    } else {
                        PieceColour::White
                    };

                    if y != 7 && board.get(move_one, y + 1).contains(&opposite_colour) {
                        moves.push((move_one, y + 1));
                    };

                    if y != 0 && board.get(move_one, y - 1).contains(&opposite_colour) {
                        moves.push((move_one, y - 1));
                    };

                    moves
                }
            }
        }
    }

    fn path_empty(&self, to: (u8, u8), board: &BoardState) -> bool {
        let (start_x, start_y) = (self.x, self.y);
        let (end_x, end_y) = to;

        // same column
        if start_x == end_x {
            let range = if start_y > end_y {
                end_y..start_y
            } else {
                start_y..end_y
            };

            return range.skip(1).all(|y| board.get(start_x, y).is_none());
        }

        // same row
        if start_y == end_y {
            let range = if start_x > end_x {
                end_x..start_x
            } else {
                start_x..end_x
            };

            return range.skip(1).all(|x| board.get(x, start_y).is_none());
        }

        let x_diff = (start_x as i8 - end_x as i8).abs();
        let y_diff = (start_y as i8 - end_y as i8).abs();

        // diagonal - this condition should always be true if it is reached
        if x_diff == y_diff {
            return (1..x_diff).all(|idx| {
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

                board.get(x, y).is_none()
            });
        }

        false
    }

    /// Array of squares this piece would move through to take a piece at `target`,
    /// ignoring all other pieces on the board. This includes `target` if it is reachable.
    /// This assumes that `target` is not the same position as `(self.x, self.y)`.
    ///
    /// Used to calculate if moving a piece would block or unblock this piece's movement to the king
    pub fn path_to_take_piece_at(&self, target: (u8, u8)) -> Vec<(u8, u8)> {
        let (x, y) = (self.x as i8, self.y as i8);
        let (t_x, t_y) = (target.0 as i8, target.1 as i8);

        match self.kind {
            PieceKind::King => {
                if (x - t_x).abs() == 1 || (y - t_y).abs() == 1 {
                    vec![target]
                } else {
                    vec![]
                }
            }
            PieceKind::Queen => {
                if x == t_x {
                    if t_y > y {
                        // can't use ..= because `RangeInclusive` != `Range`
                        y+1..t_y+1
                    } else {
                        t_y..y
                    }.map(|y| (self.x, y as u8)).collect()
                } else if y == t_y {
                    if t_x > x {
                        x+1..t_x+1
                    } else {
                        t_x..x
                    }.map(|x| (x as u8, self.y)).collect()
                } else if (x - t_x).abs() == (y - t_y).abs() {
                    if t_x > x {
                        1..=(t_x - x)
                    } else {
                        (t_x - x)..=-1
                    }.map(|adj| ((x + adj) as u8, (y + adj) as u8)).collect()
                } else {
                    vec![]
                }
            }
            PieceKind::Bishop => {
                if (x - t_x).abs() == (y - t_y).abs() {
                    if t_x > x {
                        1..=(t_x - x)
                    } else {
                        (t_x - x)..=-1
                    }.map(|adj| ((x + adj) as u8, (y + adj) as u8)).collect()
                } else {
                    vec![]
                }
            }
            PieceKind::Knight => {
                if ((x - t_x).abs() == 2 && (y - t_y).abs() == 1)
                    || ((x - t_x).abs() == 1 && (y - t_y).abs() == 2)
                {
                    vec![target]
                } else {
                    vec![]
                }
            }
            PieceKind::Rook => {
                if x == t_x {
                    if t_y > y {
                        y+1..t_y+1
                    } else {
                        t_y..y
                    }.map(|y| (self.x, y as u8)).collect()
                } else if y == t_y {
                    if t_x > x {
                        x+1..t_x+1
                    } else {
                        t_x..x
                    }.map(|x| (x as u8, self.y)).collect()
                } else {
                    vec![]
                }
            }
            PieceKind::Pawn => {
                if ((self.colour == PieceColour::White && t_x == x + 1)
                    || (self.colour == PieceColour::Black && t_x == x - 1))
                    && (y - t_y).abs() == 1
                {
                    vec![target]
                } else {
                    vec![]
                }
            }
        }
    }
}

const VELOCITY: f32 = 7.0;
// TODO acceleration; y movement - Bezier curve maybe?
fn move_pieces(
    mut commands: Commands,
    time: Res<Time>,
    mut state: ResMut<State<GameState>>,
    mut turn: ResMut<PlayerTurn>,
    query: Query<(Entity, &MovePiece, &mut Piece, &mut Transform)>,
) {
    query.for_each_mut(|(piece_entity, move_piece, mut piece, mut transform)| {
        let direction =
            Vec3::new(move_piece.target_x, 0.0, move_piece.target_y) - transform.translation;

        if direction.length() > f32::EPSILON * 2.0 {
            let delta = VELOCITY * (direction.normalize() * time.delta_seconds());
            if delta.length() > direction.length() {
                transform.translation += direction;
            } else {
                transform.translation += delta;
            }
        } else {
            piece.x = move_piece.target_x as u8;
            piece.y = move_piece.target_y as u8;

            commands.entity(piece_entity).remove::<MovePiece>();
            turn.next();
            state.set(GameState::NothingSelected).unwrap();
        }
    });
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

fn create_pieces(mut commands: Commands, meshes: Res<PieceMeshes>, materials: Res<PieceMaterials>) {
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
    spawn_queen(
        commands,
        material.clone(),
        meshes.queen.clone(),
        colour,
        (back_row, 3),
    );
    spawn_king(
        commands,
        material.clone(),
        meshes.king.clone(),
        meshes.king_cross.clone(),
        colour,
        (back_row, 4),
    );
    spawn_bishop(
        commands,
        material.clone(),
        meshes.bishop.clone(),
        colour,
        (back_row, 5),
    );
    spawn_knight(
        commands,
        material.clone(),
        meshes.knight_base.clone(),
        meshes.knight.clone(),
        colour,
        (back_row, 6),
    );
    spawn_rook(
        commands,
        material.clone(),
        meshes.rook.clone(),
        colour,
        (back_row, 7),
    );

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

        Self { white, black }
    }
}
