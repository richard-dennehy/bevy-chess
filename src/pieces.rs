use crate::board::{BoardState, GameState, MovePiece, PlayerTurn, PromotedPawn, Square};
use crate::moves_calculator::{Move, MoveKind, PotentialMove};
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

const SCALE_FACTOR: f32 = 15.0;

#[derive(Debug, Copy, Clone)]
pub struct Piece {
    pub colour: PieceColour,
    pub kind: PieceKind,
    pub square: Square,
}

impl Piece {
    pub fn white(kind: PieceKind, square: Square) -> Self {
        Piece {
            colour: PieceColour::White,
            kind,
            square,
        }
    }

    pub fn black(kind: PieceKind, square: Square) -> Self {
        Piece {
            colour: PieceColour::Black,
            kind,
            square,
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

    pub fn starting_front_rank(&self) -> u8 {
        match self {
            PieceColour::White => 1,
            PieceColour::Black => 6,
        }
    }

    pub fn starting_back_rank(&self) -> u8 {
        match self {
            PieceColour::White => 0,
            PieceColour::Black => 7,
        }
    }

    pub fn final_rank(&self) -> u8 {
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
    pub square: Square,
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
                    (colour == self.colour.opposite()).then(|| potential_move.to_move())
                } else {
                    Some(potential_move.to_move())
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
                    square: potential_move.target_square,
                    colour: blockage,
                })
            })
            .collect()
    }

    pub fn contains(&self, square: Square) -> bool {
        self.potential_moves
            .iter()
            .any(|potential| potential.target_square == square)
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

                        *done = p_move.target_square == square;
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
            kind: MoveKind::Standard,
            target_square: (x, y).into(),
            blocked_by: *board.get((x, y).into()),
        };

        let up = || {
            PiecePath::from_iterator(
                ((self.square.rank + 1)..8)
                    .map(|new_rank| potential_move((new_rank, self.square.file))),
                self.colour,
            )
        };

        let down = || {
            PiecePath::from_iterator(
                (0..self.square.rank)
                    .rev()
                    .map(|new_rank| potential_move((new_rank, self.square.file))),
                self.colour,
            )
        };

        let left = || {
            PiecePath::from_iterator(
                (0..self.square.file)
                    .rev()
                    .map(|new_file| potential_move((self.square.rank, new_file))),
                self.colour,
            )
        };

        let right = || {
            PiecePath::from_iterator(
                ((self.square.file + 1)..8)
                    .map(|new_rank| potential_move((self.square.rank, new_rank))),
                self.colour,
            )
        };

        let up_left = || {
            PiecePath::from_iterator(
                ((self.square.rank + 1)..8)
                    .filter_map(|new_rank| {
                        let diff = self.square.rank.abs_diff(new_rank);
                        (diff <= self.square.file).then(|| (new_rank, self.square.file - diff))
                    })
                    .map(potential_move),
                self.colour,
            )
        };

        let up_right = || {
            PiecePath::from_iterator(
                ((self.square.rank + 1)..8)
                    .filter_map(|new_rank| {
                        let new_file = self.square.file + self.square.rank.abs_diff(new_rank);
                        (new_file < 8).then(|| (new_rank, new_file))
                    })
                    .map(potential_move),
                self.colour,
            )
        };

        let down_left = || {
            PiecePath::from_iterator(
                (0..self.square.rank)
                    .rev()
                    .filter_map(|new_rank| {
                        let diff = self.square.rank.abs_diff(new_rank);
                        (diff <= self.square.file).then(|| (new_rank, self.square.file - diff))
                    })
                    .map(potential_move),
                self.colour,
            )
        };

        let down_right = || {
            PiecePath::from_iterator(
                (0..self.square.rank)
                    .rev()
                    .filter_map(|new_rank| {
                        let new_file = self.square.file + self.square.rank.abs_diff(new_rank);
                        (new_file < 8).then(|| (new_rank, new_file))
                    })
                    .map(potential_move),
                self.colour,
            )
        };

        let (rank, file) = (self.square.rank as i8, self.square.file as i8);

        let is_on_board = |(rank, file): (i8, i8)| {
            ((0..8).contains(&rank) && (0..8).contains(&file)).then(|| (rank as u8, file as u8))
        };

        match self.kind {
            PieceKind::King => [
                (rank - 1, file - 1),
                (rank - 1, file),
                (rank - 1, file + 1),
                (rank, file - 1),
                (rank, file + 1),
                (rank + 1, file - 1),
                (rank + 1, file),
                (rank + 1, file + 1),
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
                (rank - 2, file - 1),
                (rank - 2, file + 1),
                (rank + 2, file - 1),
                (rank + 2, file + 1),
                (rank - 1, file - 2),
                (rank - 1, file + 2),
                (rank + 1, file - 2),
                (rank + 1, file + 2),
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

        let rank = self.square.rank as i8;
        let file = self.square.file;
        let direction = self.colour.pawn_direction();

        if self.square.rank == self.colour.final_rank() {
            PawnMoves {
                advance_one: None,
                advance_two: None,
                attack_left: None,
                attack_right: None,
            }
        } else {
            // note: pawns don't really fit into the "PiecePath" model
            let move_one = (rank + direction) as u8;
            let move_two = (rank + (2 * direction)) as u8;

            let advance_one =
                board
                    .get((move_one, file).into())
                    .is_none()
                    .then_some(PotentialMove::new(
                        Move::standard((move_one, file).into()),
                        None,
                    ));

            let advance_two = (self.square.rank == self.colour.starting_front_rank()
                && board.get((move_one, file).into()).is_none()
                && board.get((move_two, file).into()).is_none())
            .then_some(PotentialMove::new(
                Move::pawn_double_step((move_two, file).into()),
                None,
            ));

            let left_diagonal_occupied = || {
                board
                    .get((move_one, file - 1).into())
                    .contains(&self.colour.opposite())
            };
            let attack_left = (file != 0 && (attack_empty_squares || left_diagonal_occupied()))
                .then(|| PotentialMove::new(Move::standard((move_one, file - 1).into()), None));

            let right_diagonal_occupied = || {
                board
                    .get((move_one, file + 1).into())
                    .contains(&self.colour.opposite())
            };
            let attack_right = (file != 7 && (attack_empty_squares || right_diagonal_occupied()))
                .then(|| PotentialMove::new(Move::standard((move_one, file + 1).into()), None));

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
                let direction = move_piece.target_translation() - transform.translation;

                if direction.length() > f32::EPSILON * 2.0 {
                    let delta = VELOCITY * (direction.normalize() * time.delta_seconds());
                    if delta.length() > direction.length() {
                        transform.translation += direction;
                    } else {
                        transform.translation += delta;
                    }

                    true
                } else {
                    piece.square = move_piece.target_square();

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

    let scale = Transform::from_scale(Vec3::splat(SCALE_FACTOR));
    let translation = Transform::from_xyz(0.0, -0.062 * SCALE_FACTOR, 0.0);
    let transform = translation * scale;

    commands
        .spawn_bundle((transform, GlobalTransform::identity()))
        .with_children(|parent| {
            parent.spawn_scene(chessboard);
        });
}

fn create_pieces(mut commands: Commands, meshes: Res<PieceMeshes>, materials: Res<PieceMaterials>) {
    [PieceColour::White, PieceColour::Black].into_iter().for_each(|colour| {
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

fn spawn_piece(
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

    let square = piece.square;
    commands.entity(entity).despawn_recursive();

    let new_entity = spawn_piece(
        &mut commands,
        &materials,
        &meshes,
        turn.0,
        new_kind,
        square,
    );
    promoted_pawn.0 = Some(new_entity);
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
