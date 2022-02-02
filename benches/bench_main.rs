use bevy::prelude::{IntoSystem, Stage, State, SystemStage, World};
use bevy_chess::model::{AllValidMoves, Piece, PieceColour, PieceKind, Square};
use bevy_chess::systems::chess::{calculate_all_moves, GameState, PlayerTurn};
use criterion::*;

fn calculate_moves_for_default_board(c: &mut Criterion) {
    c.bench_function("calculate moves for default board", |b| {
        let (mut world, mut system) = setup();

        let ids = pieces()
            .into_iter()
            .map(|piece| world.spawn().insert(piece).id())
            .collect::<Vec<_>>();

        b.iter(|| {
            system.run(&mut world);
        });

        let all_moves = world.get_resource::<AllValidMoves>().unwrap();
        ids.into_iter().for_each(|id| {
            all_moves.get(id);
        })
    });
}

fn setup() -> (World, SystemStage) {
    let mut world = World::new();

    world.insert_resource(AllValidMoves::default());
    world.insert_resource(PlayerTurn(PieceColour::Black));
    world.insert_resource(State::new(GameState::NothingSelected));

    let mut update_stage = SystemStage::parallel();
    update_stage.add_system_set(State::<GameState>::get_driver());
    update_stage.add_system(calculate_all_moves.system());

    (world, update_stage)
}

fn pieces() -> Vec<Piece> {
    let back_row = [
        PieceKind::Rook,
        PieceKind::Knight,
        PieceKind::Bishop,
        PieceKind::Queen,
        PieceKind::King,
        PieceKind::Bishop,
        PieceKind::Knight,
        PieceKind::Rook,
    ];
    let front_row = |colour: PieceColour| {
        (0..8)
            .map(|file| Piece {
                square: Square::new(colour.starting_front_rank(), file),
                kind: PieceKind::Pawn,
                colour,
            })
            .collect::<Vec<_>>()
    };

    back_row
        .iter()
        .enumerate()
        .map(|(idx, kind)| Piece::white (
            *kind,
            Square::new(0, idx as _),
        ))
        .chain(front_row(PieceColour::White).into_iter())
        .chain(front_row(PieceColour::Black).into_iter())
        .chain(back_row.iter().enumerate().map(|(idx, kind)| Piece::black (
            *kind,
            Square::new(7, idx as _)
        )))
        .collect::<Vec<_>>()
}

criterion_group! {
    benches,
    calculate_moves_for_default_board,
}

criterion_main!(benches);
