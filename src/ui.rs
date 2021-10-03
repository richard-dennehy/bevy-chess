use crate::board::PlayerTurn;
use crate::pieces::PieceColour;
use bevy::prelude::*;

pub struct UiPlugin;
impl Plugin for UiPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_startup_system(initialise.system())
            .add_system(update_next_move.system());
    }
}

fn update_next_move(
    turn: Res<PlayerTurn>,
    query: Query<(&mut Text, &NextMoveText)>,
) {
    if !turn.is_changed() {
        return;
    }

    query.for_each_mut(|(mut text, _)| {
        // fixme can probably use multiple text sections instead and just update section[1]
        text.sections[0].value = format!(
            "Next move: {}",
            match turn.0 {
                PieceColour::White => "White",
                PieceColour::Black => "Black",
            }
        )
    })
}

fn initialise(
    mut commands: Commands,
    asset_server: ResMut<AssetServer>,
    mut colour_materials: ResMut<Assets<ColorMaterial>>,
) {
    let font = asset_server.load("fonts/FiraSans-Bold.ttf");
    let material = colour_materials.add(Color::NONE.into());

    commands.spawn_bundle(UiCameraBundle::default());
    commands
        .spawn_bundle(NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                position: Rect {
                    left: Val::Px(10.0),
                    top: Val::Px(10.0),
                    ..Default::default()
                },
                ..Default::default()
            },
            material,
            ..Default::default()
        })
        .with_children(|parent| {
            parent
                .spawn_bundle(TextBundle {
                    text: Text::with_section(
                        "Next move: White",
                        TextStyle {
                            font,
                            font_size: 40.0,
                            color: Color::rgb(0.8, 0.8, 0.8),
                        },
                        TextAlignment::default(),
                    ),
                    ..Default::default()
                })
                .insert(NextMoveText);
        });
}

struct NextMoveText;
