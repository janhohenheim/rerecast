#![expect(
    deprecated,
    reason = "bevy_feathers bundle templates; migrate to @FeathersButton BSN when ui uses bsn!"
)]

use bevy::{
    ecs::{
        prelude::*,
        system::{IntoObserverSystem, ObserverSystem},
    },
    feathers::{
        controls::{ButtonBundleProps, ButtonVariant, button_bundle, checkbox_bundle},
        font_styles::InheritableFont,
        theme::{ThemeBackgroundColor, ThemedText},
        tokens,
    },
    input_focus::tab_navigation::{TabGroup, TabIndex},
    prelude::*,
    tasks::prelude::*,
    text::{EditableText, EditableTextFilter, FontSize, TextCursorStyle},
    ui::{Checked, InteractionDisabled, Val::*},
    ui_widgets::{Activate, ValueChange, observe},
    window::{PrimaryWindow, RawHandleWrapper},
};
use bevy_rerecast::prelude::*;

use rfd::AsyncFileDialog;

use crate::{
    backend::{BuildNavmesh, GlobalNavmeshSettings},
    get_navmesh_input::GetNavmeshInput,
    load::LoadTask,
    save::SaveTask,
    visualization::{AvailableGizmos, GizmosToDraw, ObstacleGizmo},
};

pub(super) fn plugin(app: &mut App) {
    app.add_systems(Startup, spawn_ui);
    app.add_systems(Update, read_config_inputs);
    app.add_observer(update_primary_buttons_when_obstacle_added);
    app.add_observer(update_primary_buttons_when_obstacle_removed);
    app.add_observer(set_ui_size);
    app.add_observer(set_font_size);
}

fn spawn_ui(mut commands: Commands) {
    let ui = ui_bundle();
    commands.spawn(ui);
}

fn ui_bundle() -> impl Bundle {
    (
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
        TabGroup::default(),
        children![
            (
                Name::new("Menu Bar"),
                Node {
                    padding: UiRect::axes(Px(10.0), Px(5.0)),
                    column_gap: Val::Px(5.0),
                    ..default()
                },
                ThemeBackgroundColor(tokens::WINDOW_BG),
                children![
                    editable_text_field(
                        "http://127.0.0.1:15702",
                        0,
                        16.0,
                        Node {
                            width: Val::Px(250.),
                            height: percent(100),
                            top: px(2),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        ConnectionInput,
                    ),
                    menu_button((
                        button_bundle(
                            ButtonBundleProps::default(),
                            (),
                            Spawn((Text::new("Load Scene"), ThemedText))
                        ),
                        observe(|_: On<Activate>, mut commands: Commands| {
                            commands.trigger(GetNavmeshInput);
                        }),
                        LoadSceneButton
                    )),
                    hspace(px(20)),
                    menu_button((
                        button_bundle(
                            ButtonBundleProps::default(),
                            InteractionDisabled,
                            Spawn((Text::new("Build"), ThemedText))
                        ),
                        observe(|_: On<Activate>, mut commands: Commands| {
                            commands.trigger(BuildNavmesh);
                        }),
                        BuildNavmeshButton
                    )),
                    menu_button((
                        button_bundle(
                            ButtonBundleProps::default(),
                            InteractionDisabled,
                            Spawn((Text::new("Save"), ThemedText))
                        ),
                        observe(save_navmesh),
                        SaveNavmeshButton
                    )),
                    menu_button((
                        button_bundle(
                            ButtonBundleProps::default(),
                            InteractionDisabled,
                            Spawn((Text::new("Load"), ThemedText))
                        ),
                        observe(load_navmesh),
                        LoadNavmeshButton
                    )),
                ]
            ),
            (
                Name::new("Property Panel"),
                ThemeBackgroundColor(tokens::WINDOW_BG),
                Node {
                    width: px(280),
                    justify_self: JustifySelf::End,
                    flex_direction: FlexDirection::Column,
                    column_gap: px(8),
                    padding: UiRect::all(Px(30.0)),
                    align_content: AlignContent::Start,
                    ..default()
                },
                children![
                    (
                        Node {
                            display: Display::Grid,
                            grid_template_columns: vec![
                                RepeatedGridTrack::percent(1, 80.),
                                RepeatedGridTrack::percent(1, 20.)
                            ],
                            column_gap: px(8),
                            row_gap: px(5),
                            ..default()
                        },
                        InheritableFont {
                            font_size: FontSize::Px(FONT_SIZE),
                            ..default()
                        },
                        children![
                            decimal_option_label("Cell Size Fraction"),
                            decimal_option_input(
                                1,
                                CellSizeInput,
                                GlobalNavmeshSettings::default().cell_size_fraction
                            ),
                            decimal_option_label("Cell Height Fraction"),
                            decimal_option_input(
                                2,
                                CellHeightInput,
                                GlobalNavmeshSettings::default().cell_height_fraction
                            ),
                            decimal_option_label("Agent Radius"),
                            decimal_option_input(
                                3,
                                AgentRadiusInput,
                                GlobalNavmeshSettings::default().agent_radius
                            ),
                            decimal_option_label("Agent Height"),
                            decimal_option_input(
                                4,
                                AgentHeightInput,
                                GlobalNavmeshSettings::default().agent_height
                            ),
                            decimal_option_label("Agent Walkable Climb"),
                            decimal_option_input(
                                5,
                                WalkableClimbInput,
                                GlobalNavmeshSettings::default().walkable_climb
                            ),
                            decimal_option_label("Max Slope (degrees)"),
                            decimal_option_input(
                                6,
                                MaxSlopeInput,
                                GlobalNavmeshSettings::default()
                                    .walkable_slope_angle
                                    .to_degrees()
                            ),
                        ],
                    ),
                    vspace(px(50)),
                    (
                        Node {
                            flex_direction: FlexDirection::Column,
                            left: percent(10),
                            row_gap: px(5),
                            ..default()
                        },
                        children![
                            (
                                checkbox_bundle(
                                    Checked,
                                    Spawn((Text::new("Show Visual"), ThemedText))
                                ),
                                observe(set_gizmo(AvailableGizmos::Visual))
                            ),
                            (
                                checkbox_bundle(
                                    (),
                                    Spawn((Text::new("Show Obstacles"), ThemedText))
                                ),
                                observe(set_gizmo(AvailableGizmos::Obstacles))
                            ),
                            (
                                checkbox_bundle(
                                    Checked,
                                    Spawn((Text::new("Show Detail Mesh"), ThemedText))
                                ),
                                observe(set_gizmo(AvailableGizmos::DetailMesh))
                            ),
                            (
                                checkbox_bundle(
                                    (),
                                    Spawn((Text::new("Show Polygon Mesh"), ThemedText))
                                ),
                                observe(set_gizmo(AvailableGizmos::PolyMesh))
                            )
                        ],
                    ),
                ]
            ),
            (
                Name::new("Status Bar"),
                Node {
                    display: Display::Flex,
                    justify_content: JustifyContent::SpaceBetween,
                    padding: UiRect::axes(Px(10.0), Px(5.0)),
                    ..default()
                },
                ThemeBackgroundColor(tokens::WINDOW_BG),
                children![(StatusText, label("")), label("Rerecast Editor v0.4.2")],
            )
        ],
    )
}

#[derive(Component)]
struct CellSizeInput;

#[derive(Component)]
struct CellHeightInput;

#[derive(Component)]
struct AgentHeightInput;

#[derive(Component)]
struct AgentRadiusInput;

#[derive(Component)]
struct WalkableClimbInput;

#[derive(Component)]
struct MaxSlopeInput;

fn read_config_inputs(
    mut settings: ResMut<GlobalNavmeshSettings>,
    cell_size: Single<&EditableText, With<CellSizeInput>>,
    cell_height: Single<&EditableText, With<CellHeightInput>>,
    agent_height: Single<&EditableText, With<AgentHeightInput>>,
    agent_radius: Single<&EditableText, With<AgentRadiusInput>>,
    walkable_climb: Single<&EditableText, With<WalkableClimbInput>>,
    max_slope: Single<&EditableText, With<MaxSlopeInput>>,
) {
    let d = NavmeshSettings::default();
    settings.0 = NavmeshSettings {
        cell_size_fraction: cell_size
            .value()
            .to_string()
            .parse()
            .unwrap_or(d.cell_size_fraction),
        cell_height_fraction: cell_height
            .value()
            .to_string()
            .parse()
            .unwrap_or(d.cell_height_fraction),
        walkable_slope_angle: max_slope
            .value()
            .to_string()
            .parse()
            .unwrap_or(d.walkable_slope_angle.to_degrees())
            .to_radians(),
        agent_height: agent_height
            .value()
            .to_string()
            .parse()
            .unwrap_or(d.agent_height),
        walkable_climb: walkable_climb
            .value()
            .to_string()
            .parse()
            .unwrap_or(d.walkable_climb),
        agent_radius: agent_radius
            .value()
            .to_string()
            .parse()
            .unwrap_or(d.agent_radius),
        min_region_size: d.min_region_size,
        merge_region_size: d.merge_region_size,
        detail_sample_max_error: d.detail_sample_max_error,
        tile_size: d.tile_size,
        aabb: d.aabb,
        contour_flags: d.contour_flags,
        tiling: d.tiling,
        area_volumes: d.area_volumes.clone(),
        edge_max_len_factor: d.edge_max_len_factor,
        max_simplification_error: d.max_simplification_error,
        max_vertices_per_polygon: d.max_vertices_per_polygon,
        detail_sample_dist: d.detail_sample_dist,
        up: d.up,
        filter: None,
    };
}

fn save_navmesh(
    _: On<Activate>,
    mut commands: Commands,
    maybe_task: Option<Res<SaveTask>>,
    window_handle: Single<&RawHandleWrapper, With<PrimaryWindow>>,
) {
    if maybe_task.is_some() {
        // Already saving, do nothing
        return;
    }

    // Safety: we're on the main thread, so this is fine??? I think??
    let window_handle = unsafe { window_handle.get_handle() };
    let thread_pool = AsyncComputeTaskPool::get();
    let future = AsyncFileDialog::new()
        .add_filter("Navmesh", &["nav"])
        .add_filter("All files", &["*"])
        .set_title("Save Navmesh")
        .set_file_name("navmesh.nav")
        .set_parent(&window_handle)
        .set_can_create_directories(true)
        .save_file();
    let task = thread_pool.spawn(future);
    commands.insert_resource(SaveTask(task));
}

fn load_navmesh(
    _: On<Activate>,
    mut commands: Commands,
    maybe_task: Option<Res<LoadTask>>,
    window_handle: Single<&RawHandleWrapper, With<PrimaryWindow>>,
) {
    if maybe_task.is_some() {
        // Already saving, do nothing
        return;
    }

    // Safety: we're on the main thread, so this is fine??? I think??
    let window_handle = unsafe { window_handle.get_handle() };
    let thread_pool = AsyncComputeTaskPool::get();
    let future = AsyncFileDialog::new()
        .add_filter("Navmesh", &["nav"])
        .add_filter("All files", &["*"])
        .set_title("Load Navmesh")
        .set_file_name("navmesh.nav")
        .set_parent(&window_handle)
        .set_can_create_directories(false)
        .pick_file();
    let task = thread_pool.spawn(future);
    commands.insert_resource(LoadTask(task));
}

fn menu_button(button: impl Bundle) -> impl Bundle {
    (
        Node {
            width: Val::Px(130.0),
            ..default()
        },
        children![(button, ThemedText)],
    )
}

fn hspace(h: Val) -> impl Bundle {
    Node {
        width: h,
        ..default()
    }
}

fn vspace(v: Val) -> impl Bundle {
    Node {
        height: v,
        ..default()
    }
}

#[derive(Component)]
struct LoadSceneButton;

#[derive(Component)]
struct BuildNavmeshButton;

#[derive(Component)]
struct SaveNavmeshButton;

#[derive(Component)]
struct LoadNavmeshButton;

#[derive(Component)]
struct StatusText;

fn update_primary_buttons_when_obstacle_added(
    _obstacle_added: On<Add, ObstacleGizmo>,
    load_button: Single<Entity, With<LoadSceneButton>>,
    build_button: Single<Entity, With<BuildNavmeshButton>>,
    save_button: Single<Entity, With<SaveNavmeshButton>>,
    load_navmesh_button: Single<Entity, With<LoadNavmeshButton>>,
    mut commands: Commands,
) {
    commands.entity(*load_button).insert(ButtonVariant::Normal);
    commands
        .entity(*build_button)
        .insert(ButtonVariant::Primary)
        .remove::<InteractionDisabled>();
    commands
        .entity(*save_button)
        .remove::<InteractionDisabled>();
    commands
        .entity(*load_navmesh_button)
        .remove::<InteractionDisabled>();
}

fn update_primary_buttons_when_obstacle_removed(
    _obstacle_removed: On<Remove, ObstacleGizmo>,
    load_button: Single<Entity, With<LoadSceneButton>>,
    build_button: Single<Entity, With<BuildNavmeshButton>>,
    save_button: Single<Entity, With<SaveNavmeshButton>>,
    load_navmesh_button: Single<Entity, With<LoadNavmeshButton>>,
    mut commands: Commands,
) {
    commands.entity(*load_button).insert(ButtonVariant::Primary);
    commands
        .entity(*build_button)
        .insert((ButtonVariant::Normal, InteractionDisabled));
    commands.entity(*save_button).insert(InteractionDisabled);
    commands
        .entity(*load_navmesh_button)
        .insert(InteractionDisabled);
}

fn editable_text_field(
    initial_text: impl Into<String>,
    tab_index: i32,
    font_size: f32,
    node: Node,
    marker: impl Bundle,
) -> impl Bundle {
    (
        node,
        EditableText {
            allow_newlines: false,
            ..EditableText::new(initial_text.into())
        },
        TextLayout::no_wrap(),
        TextFont {
            font_size: FontSize::Px(font_size),
            ..default()
        },
        TextCursorStyle {
            color: Color::WHITE,
            ..TextCursorStyle::default()
        },
        TabIndex(tab_index),
        marker,
    )
}

fn decimal_option_label(text: impl Into<String>) -> impl Bundle {
    (
        Node {
            justify_self: JustifySelf::End,
            ..default()
        },
        ThemedText,
        Text::new(text.into()),
    )
}

fn decimal_option_input(tab_index: i32, marker: impl Bundle, initial_value: f32) -> impl Bundle {
    (
        editable_text_field(
            initial_value.to_string(),
            tab_index,
            14.0,
            Node {
                width: Val::Px(50.),
                height: Val::Px(25.),
                ..default()
            },
            marker,
        ),
        ThemeBackgroundColor(tokens::SLIDER_BG),
        EditableTextFilter::new(|character| {
            character.is_ascii_digit() || character == '.' || character == '-'
        }),
    )
}

fn set_gizmo(gizmo: AvailableGizmos) -> impl ObserverSystem<ValueChange<bool>, ()> {
    IntoObserverSystem::into_system(
        move |val: On<ValueChange<bool>>,
              mut gizmos: ResMut<GizmosToDraw>,
              mut commands: Commands| {
            if val.value {
                commands.entity(val.source).insert(Checked);
            } else {
                commands.entity(val.source).remove::<Checked>();
            }
            gizmos.set(gizmo, val.value);
        },
    )
}

fn set_ui_size(add: On<Add, InheritableFont>, mut font: Query<&mut InheritableFont>) {
    font.get_mut(add.entity).unwrap().font_size = FontSize::Px(FONT_SIZE);
}
fn set_font_size(add: On<Add, TextFont>, mut font: Query<&mut TextFont>) {
    font.get_mut(add.entity).unwrap().font_size = FontSize::Px(FONT_SIZE);
}

const FONT_SIZE: f32 = 18.0;

fn label(text: impl Into<String>) -> impl Bundle {
    (
        Node::default(),
        InheritableFont {
            font_size: FontSize::Px(FONT_SIZE),
            ..default()
        },
        children![(Text(text.into()), ThemedText)],
    )
}
#[derive(Component)]
pub(crate) struct ConnectionInput;
