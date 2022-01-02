use crate::board::{BoardState, GameState, MovePiece, PlayerTurn, PromotedPawn};
use crate::moves_calculator::{Move, PotentialMove};
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
            )
            .add_system_set(
                SystemSet::on_update(GameState::PawnPromotion)
                    .with_system(promote_pawn_at_final_rank.system()),
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

impl Piece {
    pub fn white(kind: PieceKind, x: u8, y: u8) -> Self {
        Piece {
            colour: PieceColour::White,
            kind,
            x,
            y,
        }
    }

    pub fn black(kind: PieceKind, x: u8, y: u8) -> Self {
        Piece {
            colour: PieceColour::Black,
            kind,
            x,
            y,
        }
    }
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

    pub fn pawn_direction(&self) -> i8 {
        if *self == PieceColour::Black {
            -1
        } else {
            1
        }
    }

    pub fn starting_front_row(&self) -> u8 {
        match self {
            PieceColour::White => 1,
            PieceColour::Black => 6,
        }
    }

    pub fn starting_back_row(&self) -> u8 {
        match self {
            PieceColour::White => 0,
            PieceColour::Black => 7,
        }
    }

    pub fn final_row(&self) -> u8 {
        match self {
            PieceColour::White => 7,
            PieceColour::Black => 0,
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

#[derive(Debug, PartialEq)]
pub struct PiecePath {
    potential_moves: Vec<PotentialMove>,
    colour: PieceColour,
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub struct Obstruction {
    pub x: u8,
    pub y: u8,
    pub colour: PieceColour,
}

impl PiecePath {
    pub fn new(potential_moves: Vec<PotentialMove>, colour: PieceColour) -> Self {
        Self {
            potential_moves,
            colour,
        }
    }

    pub fn single(potential_move: PotentialMove, colour: PieceColour) -> Self {
        Self {
            potential_moves: vec![potential_move],
            colour,
        }
    }

    pub fn from_iterator(
        iter: impl Iterator<Item = PotentialMove>,
        colour: PieceColour,
    ) -> Option<Self> {
        let moves = iter.collect::<Vec<_>>();
        if moves.is_empty() {
            None
        } else {
            Some(Self::new(moves, colour))
        }
    }

    pub fn legal_path(&self) -> impl Iterator<Item = Move> + '_ {
        // this needs to return an Iterator (even though it makes this code a bit awkward)
        // otherwise it causes lifetime issues for the call sites in moves_calculator
        self.potential_moves
            .iter()
            .scan(false, |blocked, potential_move| {
                if *blocked {
                    return None;
                };

                if let Some(colour) = potential_move.blocked_by {
                    *blocked = true;
                    (colour == self.colour.opposite()).then(|| potential_move.move_)
                } else {
                    Some(potential_move.move_)
                }
            })
    }

    pub fn legal_path_vec(&self) -> Vec<Move> {
        self.legal_path().collect()
    }

    pub fn obstructions(&self) -> Vec<Obstruction> {
        self.potential_moves
            .iter()
            .filter_map(|potential_move| {
                potential_move.blocked_by.map(|blockage| Obstruction {
                    x: potential_move.move_.x,
                    y: potential_move.move_.y,
                    colour: blockage,
                })
            })
            .collect()
    }

    pub fn contains(&self, x: u8, y: u8) -> bool {
        self.potential_moves
            .iter()
            .any(|potential| potential.move_.x == x && potential.move_.y == y)
    }

    pub fn truncate_to(&self, x: u8, y: u8) -> Option<Self> {
        if self.contains(x, y) {
            Some(PiecePath {
                potential_moves: self
                    .potential_moves
                    .iter()
                    // take_while_and_then_one_more_please
                    .scan(false, |done, p_move| {
                        if *done {
                            return None;
                        };

                        *done = p_move.move_.x == x && p_move.move_.y == y;
                        Some(p_move)
                    })
                    .copied()
                    .collect(),
                colour: self.colour,
            })
        } else {
            None
        }
    }
}

#[derive(Debug)]
pub struct PawnMoves {
    pub attack_left: Option<PotentialMove>,
    pub attack_right: Option<PotentialMove>,
    pub advance_one: Option<PotentialMove>,
    pub advance_two: Option<PotentialMove>,
}

impl Piece {
    pub fn valid_moves(&self, board: &BoardState) -> Vec<PiecePath> {
        let potential_move = |(x, y): (u8, u8)| PotentialMove {
            move_: Move::standard((x, y)),
            blocked_by: *board.get(x, y),
        };

        let up = || {
            PiecePath::from_iterator(
                ((self.x + 1)..8).map(|new_x| potential_move((new_x, self.y))),
                self.colour,
            )
        };

        let down = || {
            PiecePath::from_iterator(
                (0..self.x)
                    .rev()
                    .map(|new_x| potential_move((new_x, self.y))),
                self.colour,
            )
        };

        let left = || {
            PiecePath::from_iterator(
                (0..self.y)
                    .rev()
                    .map(|new_y| potential_move((self.x, new_y))),
                self.colour,
            )
        };

        let right = || {
            PiecePath::from_iterator(
                ((self.y + 1)..8).map(|new_y| potential_move((self.x, new_y))),
                self.colour,
            )
        };

        let up_left = || {
            PiecePath::from_iterator(
                ((self.x + 1)..8)
                    .filter_map(|new_x| {
                        let diff = self.x.abs_diff(new_x);
                        (diff <= self.y).then(|| (new_x, self.y - diff))
                    })
                    .map(potential_move),
                self.colour,
            )
        };

        let up_right = || {
            PiecePath::from_iterator(
                ((self.x + 1)..8)
                    .filter_map(|new_x| {
                        let new_y = self.y + self.x.abs_diff(new_x);
                        (new_y < 8).then(|| (new_x, new_y))
                    })
                    .map(potential_move),
                self.colour,
            )
        };

        let down_left = || {
            PiecePath::from_iterator(
                (0..self.x)
                    .rev()
                    .filter_map(|new_x| {
                        let diff = self.x.abs_diff(new_x);
                        (diff <= self.y).then(|| (new_x, self.y - diff))
                    })
                    .map(potential_move),
                self.colour,
            )
        };

        let down_right = || {
            PiecePath::from_iterator(
                (0..self.x)
                    .rev()
                    .filter_map(|new_x| {
                        let new_y = self.y + self.x.abs_diff(new_x);
                        (new_y < 8).then(|| (new_x, new_y))
                    })
                    .map(potential_move),
                self.colour,
            )
        };

        let (x, y) = (self.x as i8, self.y as i8);

        let is_on_board = |(x, y): (i8, i8)| {
            ((0..8).contains(&x) && (0..8).contains(&y)).then(|| (x as u8, y as u8))
        };

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
            .map(potential_move)
            .map(|move_| PiecePath::single(move_, self.colour))
            .collect(),
            PieceKind::Queen => [
                up(),
                down(),
                left(),
                right(),
                up_left(),
                up_right(),
                down_left(),
                down_right(),
            ]
            .into_iter()
            .flatten()
            .collect(),
            PieceKind::Bishop => [up_left(), up_right(), down_left(), down_right()]
                .into_iter()
                .flatten()
                .collect(),
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
            .map(potential_move)
            .map(|move_| PiecePath::single(move_, self.colour))
            .collect(),
            PieceKind::Rook => [down(), up(), right(), left()]
                .into_iter()
                .flatten()
                .collect(),
            PieceKind::Pawn => {
                let pawn_moves = self.pawn_moves(board, false);

                [
                    pawn_moves.advance_one,
                    pawn_moves.advance_two,
                    pawn_moves.attack_left,
                    pawn_moves.attack_right,
                ]
                .into_iter()
                .flatten()
                .map(|move_| PiecePath::single(move_, self.colour))
                .collect()
            }
        }
    }

    /// set `attack_empty_squares` to `false` when calculating potential moves, and `true` when checking if a move is safe
    pub fn pawn_moves(&self, board: &BoardState, attack_empty_squares: bool) -> PawnMoves {
        if self.kind != PieceKind::Pawn {
            panic!("{:?} is not a pawn", self)
        };

        let x = self.x as i8;
        let y = self.y;
        let direction = self.colour.pawn_direction();
        let starting_row = self.colour.starting_front_row() as i8;
        // TODO once pawn promotion is implemented, a pawn should never start a turn on the final row
        let final_row = starting_row + (direction * 6);

        if x == final_row {
            PawnMoves {
                advance_one: None,
                advance_two: None,
                attack_left: None,
                attack_right: None,
            }
        } else {
            // note: pawns don't really fit into the "PiecePath" model
            let move_one = (x + direction) as u8;
            let move_two = (x + (2 * direction)) as u8;

            let advance_one = board.get(move_one, y).is_none().then_some(PotentialMove {
                move_: Move::standard((move_one, y)),
                blocked_by: None,
            });

            let advance_two = (x == starting_row
                && board.get(move_one, y).is_none()
                && board.get(move_two, y).is_none())
            .then_some(PotentialMove {
                move_: Move::pawn_double_step(move_two, y),
                blocked_by: None,
            });

            let left_diagonal_occupied =
                || board.get(move_one, y - 1).contains(&self.colour.opposite());
            let attack_left =
                (y != 0 && (attack_empty_squares || left_diagonal_occupied())).then(|| {
                    PotentialMove {
                        move_: Move::standard((move_one, y - 1)),
                        blocked_by: None,
                    }
                });

            let right_diagonal_occupied =
                || board.get(move_one, y + 1).contains(&self.colour.opposite());
            let attack_right = (y != 7 && (attack_empty_squares || right_diagonal_occupied()))
                .then(|| PotentialMove {
                    move_: Move::standard((move_one, y + 1)),
                    blocked_by: None,
                });

            PawnMoves {
                advance_one,
                advance_two,
                attack_left,
                attack_right,
            }
        }
    }
}

const VELOCITY: f32 = 7.0;
// TODO acceleration; y movement - Bezier curve maybe?
fn move_pieces(
    mut commands: Commands,
    time: Res<Time>,
    promoted_pawn: Res<PromotedPawn>,
    mut state: ResMut<State<GameState>>,
    mut turn: ResMut<PlayerTurn>,
    mut query: Query<(Entity, &MovePiece, &mut Piece, &mut Transform)>,
) {
    // note: castling moves two pieces on the same turn

    let any_updated =
        query
            .iter_mut()
            .any(|(piece_entity, move_piece, mut piece, mut transform)| {
                let direction = Vec3::new(move_piece.target_x, 0.0, move_piece.target_y)
                    - transform.translation;

                if direction.length() > f32::EPSILON * 2.0 {
                    let delta = VELOCITY * (direction.normalize() * time.delta_seconds());
                    if delta.length() > direction.length() {
                        transform.translation += direction;
                    } else {
                        transform.translation += delta;
                    }

                    true
                } else {
                    piece.x = move_piece.target_x as u8;
                    piece.y = move_piece.target_y as u8;

                    commands.entity(piece_entity).remove::<MovePiece>();

                    false
                }
            });

    if !any_updated {
        if let Some(_) = promoted_pawn.0 {
            state.set(GameState::PawnPromotion).unwrap();
        } else {
            turn.next();
            state.set(GameState::NothingSelected).unwrap();
        }
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
    let back_row = colour.starting_back_row();
    let front_row = colour.starting_front_row();

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
) -> Entity {
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
        })
        .id()
}

// TODO make spawn functions take PieceMaterials and PieceMeshes instead
fn spawn_queen(
    commands: &mut Commands,
    material: Handle<StandardMaterial>,
    queen: Handle<Mesh>,
    colour: PieceColour,
    (x, y): (u8, u8),
) -> Entity {
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
        })
        .id()
}

fn spawn_bishop(
    commands: &mut Commands,
    material: Handle<StandardMaterial>,
    bishop: Handle<Mesh>,
    colour: PieceColour,
    (x, y): (u8, u8),
) -> Entity {
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
        })
        .id()
}

fn spawn_rook(
    commands: &mut Commands,
    material: Handle<StandardMaterial>,
    rook: Handle<Mesh>,
    colour: PieceColour,
    (x, y): (u8, u8),
) -> Entity {
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
        })
        .id()
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
            .expect(&format!(
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

    let (x, y) = (piece.x, piece.y);
    commands.entity(entity).despawn_recursive();

    let material = materials.get(turn.0);
    let new_entity = match new_kind {
        PieceKind::Queen => spawn_queen(
            &mut commands,
            material,
            meshes.queen.clone(),
            turn.0,
            (x, y),
        ),
        PieceKind::Bishop => spawn_bishop(
            &mut commands,
            material,
            meshes.bishop.clone(),
            turn.0,
            (x, y),
        ),
        PieceKind::Knight => spawn_knight(
            &mut commands,
            material,
            meshes.knight_base.clone(),
            meshes.knight.clone(),
            turn.0,
            (x, y),
        ),
        PieceKind::Rook => spawn_rook(&mut commands, material, meshes.rook.clone(), turn.0, (x, y)),
        _ => panic!("promoted to invalid piece kind {:?}", new_kind),
    };
    promoted_pawn.0 = Some(new_entity);
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
