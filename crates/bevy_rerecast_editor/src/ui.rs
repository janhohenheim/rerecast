use bevy::{color::palettes::tailwind, ecs::system::ObserverSystem, prelude::*, ui::Val::*};

use crate::{
    build::BuildNavmesh,
    get_navmesh_input::GetNavmeshInput,
    theme::{
        palette::BEVY_GRAY,
        widget::{button, checkbox},
    },
    visualization::{AvailableGizmos, GizmosToDraw},
};

pub(super) fn plugin(app: &mut App) {
    app.add_systems(Startup, spawn_ui);
    app.add_observer(close_modal);
}

fn spawn_ui(mut commands: Commands) {
    commands.spawn((
        Name::new("Canvas"),
        Node {
            width: Percent(100.0),
            height: Percent(100.0),
            display: Display::Grid,
            grid_template_rows: vec![
                // Menu bar
                RepeatedGridTrack::auto(1),
                // Property panel
                RepeatedGridTrack::fr(1, 1.0),
                // Status bar
                RepeatedGridTrack::auto(1),
            ],
            ..default()
        },
        Pickable::IGNORE,
        children![
            (
                Name::new("Menu Bar"),
                Node {
                    padding: UiRect::axes(Px(10.0), Px(5.0)),
                    column_gap: Val::Px(5.0),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.1, 0.1, 0.1)),
                children![
                    button("Load Scene", spawn_load_scene_modal),
                    button("Build Navmesh", build_navmesh)
                ]
            ),
            (
                Name::new("Property Panel"),
                Node {
                    width: Px(300.0),
                    justify_self: JustifySelf::End,
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(Px(30.0)),
                    ..default()
                },
                children![
                    checkbox("Show Affector", toggle_gizmo(AvailableGizmos::Affector)),
                    checkbox("Show Polygon Mesh", toggle_gizmo(AvailableGizmos::PolyMesh)),
                    checkbox(
                        "Show Detail Mesh",
                        toggle_gizmo(AvailableGizmos::DetailMesh)
                    )
                ],
                BackgroundColor(BEVY_GRAY.with_alpha(0.6)),
            ),
            (
                Name::new("Status Bar"),
                Node {
                    display: Display::Flex,
                    justify_content: JustifyContent::SpaceBetween,
                    padding: UiRect::axes(Px(10.0), Px(5.0)),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.1, 0.1, 0.1)),
                children![
                    status_bar_text("Status Bar"),
                    status_bar_text("Rerecast Editor v0.1.0")
                ],
            )
        ],
    ));
}

#[derive(Component)]
struct LoadSceneModal;

fn build_navmesh(_: Trigger<Pointer<Click>>, mut commands: Commands) {
    commands.trigger(BuildNavmesh);
}

fn spawn_load_scene_modal(_: Trigger<Pointer<Click>>, mut commands: Commands) {
    commands.spawn((
        Name::new("Backdrop"),
        Node {
            width: Percent(100.0),
            height: Percent(100.0),
            display: Display::Grid,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        LoadSceneModal,
        Pickable {
            should_block_lower: true,
            ..default()
        },
        BackgroundColor(Color::srgba(0.1, 0.1, 0.1, 0.5)),
        children![(
            Name::new("Modal"),
            Node {
                min_width: Px(300.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(10.0),
                ..default()
            },
            BackgroundColor(tailwind::GRAY_300.into()),
            BorderRadius::all(Px(10.0)),
            children![
                (
                    Name::new("Title Bar"),
                    Node {
                        column_gap: Val::Px(5.0),
                        align_items: AlignItems::Center,
                        padding: UiRect::axes(Val::Px(10.0), Val::Px(5.0)),
                        ..default()
                    },
                    BackgroundColor(Color::BLACK.with_alpha(0.1)),
                    children![modal_title("Load Scene"), button("x", close_load_scene),],
                ),
                (
                    Name::new("Modal Content"),
                    Node {
                        padding: UiRect::all(Val::Px(10.0)),
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(10.0),
                        ..default()
                    },
                    children![
                        modal_text("http://127.0.0.1:15702"),
                        (
                            Name::new("Load Button"),
                            Node { ..default() },
                            children![button("Load", load_scene)]
                        )
                    ]
                )
            ],
        )],
    ));
}

fn modal_title(text: impl Into<String>) -> impl Bundle {
    (
        Node {
            flex_grow: 1.0,
            ..default()
        },
        Text::new(text),
        TextLayout::new_with_justify(JustifyText::Center),
        TextFont::from_font_size(17.0),
        TextColor(Color::BLACK),
    )
}

fn modal_text(text: impl Into<String>) -> impl Bundle {
    (
        Text::new(text),
        TextFont::from_font_size(15.0),
        TextColor(tailwind::GRAY_800.into()),
    )
}

fn load_scene(_: Trigger<Pointer<Click>>, mut commands: Commands) {
    commands.trigger(CloseModal);
    commands.trigger(GetNavmeshInput);
}

#[derive(Event)]
struct CloseModal;

fn close_modal(
    _: Trigger<CloseModal>,
    mut commands: Commands,
    modal: Single<Entity, With<LoadSceneModal>>,
) {
    commands.entity(*modal).try_despawn();
}

fn close_load_scene(_: Trigger<Pointer<Click>>, mut commands: Commands) {
    commands.trigger(CloseModal);
}

fn status_bar_text(text: impl Into<String>) -> impl Bundle {
    (
        Text::new(text),
        TextFont::from_font_size(15.0),
        TextColor(Color::srgb(0.9, 0.9, 0.9)),
    )
}

fn toggle_gizmo(gizmo: AvailableGizmos) -> impl ObserverSystem<Pointer<Click>, (), ()> {
    IntoSystem::into_system(
        move |_: Trigger<Pointer<Click>>, mut gizmos: ResMut<GizmosToDraw>| {
            gizmos.toggle(gizmo);
        },
    )
}
