use crate::board::{BoardState, GameState, MovePiece, PlayerTurn, PromotedPawn, Square};
use crate::moves_calculator::{Move, PotentialMove};
use bevy::prelude::*;
use std::f32::consts::PI;
use std::fmt::Formatter;

pub struct PiecePlugin;
impl Plugin for PiecePlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.init_resource::<PieceMeshes>()
            .init_resource::<PieceMaterials>()
            .add_startup_system(create_board.system())
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

    pub fn contains(&self, square: Square) -> bool {
        self.potential_moves
            .iter()
            .any(|potential| potential.move_.x == square.x_rank && potential.move_.y == square.y_file)
    }

    pub fn truncate_to(&self, square: Square) -> Option<Self> {
        if self.contains(square) {
            Some(PiecePath {
                potential_moves: self
                    .potential_moves
                    .iter()
                    // take_while_and_then_one_more_please
                    .scan(false, |done, p_move| {
                        if *done {
                            return None;
                        };

                        *done = p_move.move_.x == square.x_rank && p_move.move_.y == square.y_file;
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
            blocked_by: *board.get((x, y).into()),
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

            let advance_one = board.get((move_one, y).into()).is_none().then_some(PotentialMove {
                move_: Move::standard((move_one, y)),
                blocked_by: None,
            });

            let advance_two = (x == starting_row
                && board.get((move_one, y).into()).is_none()
                && board.get((move_two, y).into()).is_none())
            .then_some(PotentialMove {
                move_: Move::pawn_double_step(move_two, y),
                blocked_by: None,
            });

            let left_diagonal_occupied =
                || board.get((move_one, y - 1).into()).contains(&self.colour.opposite());
            let attack_left =
                (y != 0 && (attack_empty_squares || left_diagonal_occupied())).then(|| {
                    PotentialMove {
                        move_: Move::standard((move_one, y - 1)),
                        blocked_by: None,
                    }
                });

            let right_diagonal_occupied =
                || board.get((move_one, y + 1).into()).contains(&self.colour.opposite());
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
                let direction = move_piece.target_translation()
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
                    let target = move_piece.target_square();
                    piece.x = target.x_rank;
                    piece.y = target.y_file;

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

fn create_board(mut commands: Commands, assets: Res<AssetServer>) {
    let chessboard = assets.load("meshes/chessboard.glb#Scene0");

    let scale_factor = 15.0;
    let scale = Transform::from_scale(Vec3::new(scale_factor, scale_factor, scale_factor));
    let translation = Transform::from_xyz(0.0, -0.062 * scale_factor, 0.0);
    let transform = translation * scale;

    commands
        .spawn_bundle((transform, GlobalTransform::identity()))
        .with_children(|parent| {
            parent.spawn_scene(chessboard);
        });
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

    [
        PieceKind::Rook,
        PieceKind::Knight,
        PieceKind::Bishop,
        PieceKind::Queen,
        PieceKind::King,
        PieceKind::Bishop,
        PieceKind::Knight,
        PieceKind::Rook,
    ].into_iter().enumerate().for_each(|(file, kind)| {
        spawn_piece(
            commands,
            material.clone(),
            meshes.get(kind),
            colour,
            kind,
            (back_row, file as u8),
        );
    });

    (0..=7).for_each(|idx| {
        spawn_piece(
            commands,
            material.clone(),
            meshes.get(PieceKind::Pawn),
            colour,
            PieceKind::Pawn,
            (front_row, idx),
        );
    });
}

fn spawn_piece(
    commands: &mut Commands,
    material: Handle<StandardMaterial>,
    mesh: Handle<Mesh>,
    colour: PieceColour,
    kind: PieceKind,
    (x, y): (u8, u8),
) -> Entity {
    commands
        .spawn_bundle(PbrBundle {
            transform: place_on_square(colour, x, y),
            ..Default::default()
        })
        .insert(Piece { colour, kind, x, y })
        .with_children(|parent| {
            parent.spawn_bundle(PbrBundle {
                mesh,
                material,
                ..Default::default()
            });
        })
        .id()
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

    let new_entity = spawn_piece(
        &mut commands,
        materials.get(turn.0),
        meshes.get(new_kind),
        turn.0,
        new_kind,
        (x, y),
    );
    promoted_pawn.0 = Some(new_entity);
}

fn place_on_square(colour: PieceColour, x: u8, y: u8) -> Transform {
    // field/rank 0..7 -> x/y -3..4
    let (x, y) = (x as i8 - 3, y as i8 - 3);
    let square_size = 1.0;
    let angle = if colour == PieceColour::Black {
        PI
    } else {
        0.0
    };

    let scale = Transform::from_scale(Vec3::new(15.0, 15.0, 15.0));
    let rotation = Transform::from_rotation(Quat::from_rotation_y(angle));

    let base_translation = Transform::from_translation(Vec3::new(-0.5, 0.0, -0.5));
    let translation = Transform::from_translation(Vec3::new(y as f32, 0.0, x as f32) / square_size);

    translation * base_translation * rotation * scale
}

struct PieceMeshes {
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
