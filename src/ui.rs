use bevy::prelude::*;
use crate::systems::chess::{GameState, PlayerTurn};

pub struct UiPlugin;
impl Plugin for UiPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_startup_system(initialise.system())
            .add_system(update_next_move.system())
            .add_system(update_prompt.system());
    }
}

fn update_next_move(turn: Res<PlayerTurn>, query: Query<&mut Text, With<NextMoveText>>) {
    if !turn.is_changed() {
        return;
    }

    query.for_each_mut(|mut text| {
        text.sections[1].value = turn.0.to_string()
    })
}

fn update_prompt(game_state: Res<State<GameState>>, query: Query<&mut Text, With<NextMoveText>>) {
    if !game_state.is_changed() {
        return;
    }

    query.for_each_mut(|mut text| {
        text.sections[3].value = game_state.current().to_string()
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
            let style = TextStyle {
                font,
                font_size: 40.0,
                color: Color::rgb(0.8, 0.8, 0.8),
            };
            parent
                .spawn_bundle(TextBundle {
                    text: Text {
                        sections: vec![
                            TextSection {
                                value: "Next move: ".into(),
                                style: style.clone(),
                            },
                            TextSection {
                                value: "White".into(),
                                style: style.clone(),
                            },
                            TextSection {
                                value: "\n".into(),
                                style: style.clone(),
                            },
                            TextSection {
                                value: "Select a piece".into(),
                                style: TextStyle {
                                    font_size: 20.0,
                                    ..style
                                },
                            }
                        ],
                        alignment: TextAlignment::default(),
                    },
                    ..Default::default()
                })
                .insert(NextMoveText);
        });
}

struct NextMoveText;
