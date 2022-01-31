use super::{BoardState, Piece, PieceColour, PieceKind};

#[test]
fn board_state_for_default_board() {
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
                    square: (1, idx).into(),
                    kind: PieceKind::Pawn,
                    colour,
                })
                .collect::<Vec<_>>()
        } else {
            (0..8)
                .map(|idx| Piece {
                    square: (6, idx).into(),
                    kind: PieceKind::Pawn,
                    colour,
                })
                .collect::<Vec<_>>()
        }
    };

    let pieces = back_row
        .iter()
        .enumerate()
        .map(|(idx, kind)| Piece {
            square: (0, idx as u8).into(),
            colour: PieceColour::White,
            kind: *kind,
        })
        .chain(front_row(PieceColour::White).into_iter())
        .chain(front_row(PieceColour::Black).into_iter())
        .chain(back_row.iter().enumerate().map(|(idx, kind)| Piece {
            square: (7, idx as u8).into(),
            colour: PieceColour::Black,
            kind: *kind,
        }))
        .collect::<Vec<_>>();

    let expected = [Some(PieceColour::White); 16]
        .into_iter()
        .chain([None; 32].into_iter())
        .chain([Some(PieceColour::Black); 16].into_iter())
        .collect::<Vec<_>>();
    assert_eq!(BoardState::from(&pieces[..]).squares(), &expected);
}