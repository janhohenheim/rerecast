use bevy::{
    color::palettes::tailwind,
    ecs::{prelude::*, relationship::RelatedSpawner, spawn::SpawnWith, system::ObserverSystem},
    prelude::*,
    tasks::prelude::*,
    ui::Val::*,
    window::{PrimaryWindow, RawHandleWrapper},
};
use bevy_rerecast::prelude::*;
use bevy_ui_text_input::TextInputContents;

use rfd::AsyncFileDialog;

use crate::{
    backend::{BuildNavmesh, GlobalNavmeshSettings},
    get_navmesh_input::GetNavmeshInput,
    save::SaveTask,
    theme::{
        palette::BEVY_GRAY,
        widget::{button, checkbox, decimal_input},
    },
    visualization::{AvailableGizmos, GizmosToDraw},
};

pub(super) fn plugin(app: &mut App) {
    app.add_systems(Startup, spawn_ui);
    app.add_systems(Update, read_config_inputs);
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
                    button("Build Navmesh", build_navmesh),
                    button("Save", save_navmesh),
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
                Children::spawn(SpawnWith(|parent: &mut RelatedSpawner<ChildOf>| {
                    parent.spawn(checkbox(
                        "Show Visual",
                        toggle_gizmo(AvailableGizmos::Visual),
                    ));
                    parent.spawn(checkbox(
                        "Show Affector",
                        toggle_gizmo(AvailableGizmos::Affector),
                    ));
                    parent.spawn(checkbox(
                        "Show Polygon Mesh",
                        toggle_gizmo(AvailableGizmos::PolyMesh),
                    ));
                    parent.spawn(checkbox(
                        "Show Detail Mesh",
                        toggle_gizmo(AvailableGizmos::DetailMesh),
                    ));

                    parent.spawn(decimal_input(
                        "Cell Size Fraction",
                        GlobalNavmeshSettings::default().cell_size_fraction,
                        CellSizeInput,
                    ));

                    parent.spawn(decimal_input(
                        "Cell Height Fraction",
                        GlobalNavmeshSettings::default().cell_height_fraction,
                        CellHeightInput,
                    ));
                    parent.spawn(decimal_input(
                        "Agent Radius",
                        GlobalNavmeshSettings::default().agent_radius,
                        WalkableRadiusInput,
                    ));
                    parent.spawn(decimal_input(
                        "Agent Height",
                        GlobalNavmeshSettings::default().agent_height,
                        WalkableHeightInput,
                    ));
                })),
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
struct CellSizeInput;

#[derive(Component)]
struct CellHeightInput;

#[derive(Component)]
struct WalkableHeightInput;

#[derive(Component)]
struct WalkableRadiusInput;

fn read_config_inputs(
    mut settings: ResMut<GlobalNavmeshSettings>,
    cell_size: Single<&TextInputContents, With<CellSizeInput>>,
    cell_height: Single<&TextInputContents, With<CellHeightInput>>,
    walkable_height: Single<&TextInputContents, With<WalkableHeightInput>>,
    walkable_radius: Single<&TextInputContents, With<WalkableRadiusInput>>,
) {
    let d = NavmeshSettings::default();
    settings.0 = NavmeshSettings {
        cell_size_fraction: cell_size.get().parse().unwrap_or(d.cell_size_fraction),
        cell_height_fraction: cell_height.get().parse().unwrap_or(d.cell_height_fraction),
        agent_max_slope: d.agent_max_slope,
        agent_height: walkable_height.get().parse().unwrap_or(d.agent_height),
        agent_max_climb: d.agent_max_climb,
        agent_radius: walkable_radius.get().parse().unwrap_or(d.agent_radius),
        region_min_size: d.region_min_size,
        region_merge_size: d.region_merge_size,
        detail_sample_max_error: d.detail_sample_max_error,
        tile_size: d.tile_size,
        aabb: d.aabb,
        contour_flags: d.contour_flags,
        tiling: d.tiling,
        area_volumes: d.area_volumes.clone(),
        edge_max_len_factor: d.edge_max_len_factor,
        edge_max_error: d.edge_max_error,
        verts_per_poly: d.verts_per_poly,
        detail_sample_dist: d.detail_sample_dist,
        filter: None,
        up: d.up,
    };
}

#[derive(Component)]
struct LoadSceneModal;

fn build_navmesh(_: Trigger<Pointer<Click>>, mut commands: Commands) {
    commands.trigger(BuildNavmesh);
}

fn save_navmesh(
    _: Trigger<Pointer<Click>>,
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
