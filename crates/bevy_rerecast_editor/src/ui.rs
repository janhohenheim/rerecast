use bevy::{
    ecs::{
        prelude::*,
        system::{IntoObserverSystem, ObserverSystem},
    },
    feathers::{
        self,
        constants::fonts,
        controls::{ButtonProps, ButtonVariant},
        font_styles::InheritableFont,
        handle_or_path::HandleOrPath,
        theme::{ThemeBackgroundColor, ThemedText},
        tokens,
    },
    input_focus::InputFocus,
    prelude::*,
    tasks::prelude::*,
    ui::{Checked, InteractionDisabled, Val::*},
    ui_widgets::{Activate, ValueChange, observe},
    window::{PrimaryWindow, RawHandleWrapper},
};
use bevy_rerecast::prelude::*;
use bevy_ui_text_input::{
    TextInputContents, TextInputFilter, TextInputMode, TextInputNode, TextInputQueue,
    actions::{TextInputAction, TextInputEdit},
};

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
    app.add_observer(clear_focus);
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
                    (
                        Node {
                            width: Val::Px(250.),
                            height: percent(100),
                            top: px(2),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        TextInputNode {
                            mode: TextInputMode::SingleLine,
                            clear_on_submit: false,
                            ..Default::default()
                        },
                        TextFont {
                            font_size: 16.0,
                            ..default()
                        },
                        text_input_queue("http://127.0.0.1:15702"),
                        TextInputContents::default(),
                        ConnectionInput,
                    ),
                    menu_button((
                        feathers::controls::button(
                            ButtonProps::default(),
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
                        feathers::controls::button(
                            ButtonProps::default(),
                            InteractionDisabled,
                            Spawn((Text::new("Build"), ThemedText))
                        ),
                        observe(|_: On<Activate>, mut commands: Commands| {
                            commands.trigger(BuildNavmesh);
                        }),
                        BuildNavmeshButton
                    )),
                    menu_button((
                        feathers::controls::button(
                            ButtonProps::default(),
                            InteractionDisabled,
                            Spawn((Text::new("Save"), ThemedText))
                        ),
                        observe(save_navmesh),
                        SaveNavmeshButton
                    )),
                    menu_button((
                        feathers::controls::button(
                            ButtonProps::default(),
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
                            font: HandleOrPath::Path(fonts::REGULAR.to_owned()),
                            ..default()
                        },
                        children![
                            decimal_option_label("Cell Size Fraction"),
                            decimal_option_input(
                                CellSizeInput,
                                GlobalNavmeshSettings::default().cell_size_fraction
                            ),
                            decimal_option_label("Cell Height Fraction"),
                            decimal_option_input(
                                CellHeightInput,
                                GlobalNavmeshSettings::default().cell_height_fraction
                            ),
                            decimal_option_label("Agent Radius"),
                            decimal_option_input(
                                AgentRadiusInput,
                                GlobalNavmeshSettings::default().agent_radius
                            ),
                            decimal_option_label("Agent Height"),
                            decimal_option_input(
                                AgentHeightInput,
                                GlobalNavmeshSettings::default().agent_height
                            ),
                            decimal_option_label("Agent Walkable Climb"),
                            decimal_option_input(
                                WalkableClimbInput,
                                GlobalNavmeshSettings::default().walkable_climb
                            ),
                            decimal_option_label("Max Slope (degrees)"),
                            decimal_option_input(
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
                                feathers::controls::checkbox(
                                    Checked,
                                    Spawn((Text::new("Show Visual"), ThemedText))
                                ),
                                observe(set_gizmo(AvailableGizmos::Visual))
                            ),
                            (
                                feathers::controls::checkbox(
                                    (),
                                    Spawn((Text::new("Show Obstacles"), ThemedText))
                                ),
                                observe(set_gizmo(AvailableGizmos::Obstacles))
                            ),
                            (
                                feathers::controls::checkbox(
                                    Checked,
                                    Spawn((Text::new("Show Detail Mesh"), ThemedText))
                                ),
                                observe(set_gizmo(AvailableGizmos::DetailMesh))
                            ),
                            (
                                feathers::controls::checkbox(
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
                children![(StatusText, label("")), label("Rerecast Editor v0.4.0")],
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
    cell_size: Single<&TextInputContents, With<CellSizeInput>>,
    cell_height: Single<&TextInputContents, With<CellHeightInput>>,
    agent_height: Single<&TextInputContents, With<AgentHeightInput>>,
    agent_radius: Single<&TextInputContents, With<AgentRadiusInput>>,
    walkable_climb: Single<&TextInputContents, With<WalkableClimbInput>>,
    max_slope: Single<&TextInputContents, With<MaxSlopeInput>>,
) {
    let d = NavmeshSettings::default();
    settings.0 = NavmeshSettings {
        cell_size_fraction: cell_size.get().parse().unwrap_or(d.cell_size_fraction),
        cell_height_fraction: cell_height.get().parse().unwrap_or(d.cell_height_fraction),
        walkable_slope_angle: max_slope
            .get()
            .parse()
            .unwrap_or(d.walkable_slope_angle.to_degrees())
            .to_radians(),
        agent_height: agent_height.get().parse().unwrap_or(d.agent_height),
        walkable_climb: walkable_climb.get().parse().unwrap_or(d.walkable_climb),
        agent_radius: agent_radius.get().parse().unwrap_or(d.agent_radius),
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
            width: Val::Px(120.0),
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

fn text_input_queue(initial_text: impl Into<String>) -> TextInputQueue {
    let mut queue = TextInputQueue::default();
    let overwrite_mode = false;
    for char in initial_text.into().chars() {
        queue.add(TextInputAction::Edit(TextInputEdit::Insert(
            char,
            overwrite_mode,
        )));
    }
    queue
}

#[derive(Component)]
pub(crate) struct ConnectionInput;

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

fn clear_focus(press: On<Pointer<Press>>, mut focus: ResMut<InputFocus>) {
    if Some(press.original_event_target()) != focus.0 {
        focus.0 = None;
    }
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

fn decimal_option_input(marker: impl Bundle, initial_value: f32) -> impl Bundle {
    (
        Node {
            width: Val::Px(50.),
            height: Val::Px(25.),
            ..default()
        },
        TextInputNode {
            mode: TextInputMode::SingleLine,
            clear_on_submit: false,
            ..Default::default()
        },
        TextInputFilter::Decimal,
        TextFont {
            font_size: 14.0,
            ..default()
        },
        text_input_queue(initial_value.to_string()),
        TextInputContents::default(),
        ThemeBackgroundColor(tokens::SLIDER_BG),
        marker,
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
    font.get_mut(add.entity).unwrap().font_size = FONT_SIZE;
}
fn set_font_size(add: On<Add, TextFont>, mut font: Query<&mut TextFont>) {
    font.get_mut(add.entity).unwrap().font_size = FONT_SIZE;
}

const FONT_SIZE: f32 = 18.0;

fn label(text: impl Into<String>) -> impl Bundle {
    (
        Node::default(),
        InheritableFont {
            font: HandleOrPath::Path(fonts::REGULAR.to_owned()),
            ..default()
        },
        children![(Text(text.into()), ThemedText)],
    )
}
