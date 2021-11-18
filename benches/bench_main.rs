use bevy::prelude::{IntoSystem, Stage, State, SystemStage, World};
use bevy_chess::board::{calculate_all_moves, AllValidMoves, GameState, PlayerTurn};
use bevy_chess::pieces::{Piece, PieceColour, PieceKind};

use criterion::*;

fn calculate_moves_for_default_board(c: &mut Criterion) {
    c.bench_function("calculate moves for default board", |b| {
        let (mut world, mut system) = setup();

        let ids = pieces().into_iter().map(|piece| {
            world.spawn().insert(piece).id()
        }).collect::<Vec<_>>();

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
        if colour == PieceColour::White {
            (0..8)
                .map(|idx| Piece {
                    x: 1,
                    y: idx,
                    kind: PieceKind::Pawn,
                    colour,
                })
                .collect::<Vec<_>>()
        } else {
            (0..8)
                .map(|idx| Piece {
                    x: 6,
                    y: idx,
                    kind: PieceKind::Pawn,
                    colour,
                })
                .collect::<Vec<_>>()
        }
    };

    back_row
        .iter()
        .enumerate()
        .map(|(idx, kind)| Piece {
            x: 0,
            y: idx as _,
            colour: PieceColour::White,
            kind: *kind,
        })
        .chain(front_row(PieceColour::White).into_iter())
        .chain(front_row(PieceColour::Black).into_iter())
        .chain(back_row.iter().enumerate().map(|(idx, kind)| Piece {
            x: 7,
            y: idx as _,
            colour: PieceColour::Black,
            kind: *kind,
        }))
        .collect::<Vec<_>>()
}

criterion_group! {
    benches,
    calculate_moves_for_default_board,
}

criterion_main!(benches);
