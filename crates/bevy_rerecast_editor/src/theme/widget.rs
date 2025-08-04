//! Helper functions for creating common widgets.

use std::borrow::Cow;

use bevy::{
    ecs::{spawn::SpawnWith, system::IntoObserverSystem},
    prelude::*,
    ui::Val::*,
};
use bevy_ui_text_input::{
    TextInputContents, TextInputFilter, TextInputMode, TextInputNode, TextInputPrompt,
};

use crate::theme::{interaction::InteractionPalette, palette::*};

/// A root UI node that fills the window and centers its content.
pub fn ui_root(name: impl Into<Cow<'static, str>>) -> impl Bundle {
    (
        Name::new(name),
        Node {
            position_type: PositionType::Absolute,
            width: Percent(100.0),
            height: Percent(100.0),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            flex_direction: FlexDirection::Column,
            row_gap: Px(20.0),
            ..default()
        },
        // Don't block picking events for other UI roots.
        Pickable::IGNORE,
    )
}

/// A simple header label. Bigger than [`label`].
pub fn header(text: impl Into<String>) -> impl Bundle {
    (
        Name::new("Header"),
        Text(text.into()),
        TextFont::from_font_size(40.0),
        TextColor(HEADER_TEXT),
    )
}

/// A simple text label.
pub fn label(text: impl Into<String>) -> impl Bundle {
    (
        Name::new("Label"),
        Text(text.into()),
        TextFont::from_font_size(18.0),
        TextColor(LABEL_TEXT),
    )
}

pub fn hspace(px: f32) -> impl Bundle {
    (
        Name::new("Space"),
        Node {
            width: Px(px),
            ..default()
        },
    )
}

/// A small square button with text and an action defined as an [`Observer`].
pub(crate) fn button_small<E, B, M, I>(text: impl Into<String>, action: I) -> impl Bundle
where
    E: Event,
    B: Bundle,
    I: IntoObserverSystem<E, B, M>,
{
    button_base(
        text,
        action,
        (
            Node {
                width: Px(25.0),
                height: Px(25.0),
                border: UiRect::all(Px(2.0)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BorderRadius::all(Px(5.0)),
        ),
        (TextFont::from_font_size(24.0),),
    )
}

/// A large rounded button with text and an action defined as an [`Observer`].
pub fn button<E, B, M, I>(text: impl Into<String>, action: I) -> impl Bundle
where
    E: Event,
    B: Bundle,
    I: IntoObserverSystem<E, B, M>,
{
    button_base(
        text,
        action,
        (
            Node {
                padding: UiRect::axes(Px(8.0), Px(2.0)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BorderRadius::all(Percent(20.0)),
        ),
        (TextFont::from_font_size(15.0),),
    )
}

/// A simple button with text and an action defined as an [`Observer`]. The button's layout is provided by `button_bundle`.
fn button_base<E, B, M, I>(
    text: impl Into<String>,
    action: I,
    button_bundle: impl Bundle,
    label_bundle: impl Bundle,
) -> impl Bundle
where
    E: Event,
    B: Bundle,
    I: IntoObserverSystem<E, B, M>,
{
    let text = text.into();
    let action = IntoObserverSystem::into_system(action);
    (
        Name::new("Button"),
        Node::default(),
        Children::spawn(SpawnWith(|parent: &mut ChildSpawner| {
            parent
                .spawn((
                    Name::new("Button Inner"),
                    Button,
                    BackgroundColor(BUTTON_BACKGROUND),
                    InteractionPalette {
                        none: BUTTON_BACKGROUND,
                        disabled: BUTTON_DISABLED_BACKGROUND,
                        hovered: BUTTON_HOVERED_BACKGROUND,
                        pressed: BUTTON_PRESSED_BACKGROUND,
                    },
                    Children::spawn(SpawnWith(|parent: &mut ChildSpawner| {
                        parent
                            .spawn((
                                Name::new("Button Text"),
                                Text(text),
                                TextFont::from_font_size(20.0),
                                TextColor(BUTTON_TEXT),
                                // Don't bubble picking events from the text up to the button.
                                Pickable::IGNORE,
                            ))
                            .insert(label_bundle);
                    })),
                ))
                .insert(button_bundle)
                .observe(action);
        })),
    )
}

pub fn checkbox<E, B, M, I>(text: impl Into<String>, action: I) -> impl Bundle
where
    E: Event,
    B: Bundle,
    I: IntoObserverSystem<E, B, M>,
{
    (
        Name::new("Checkbox"),
        Node {
            align_items: AlignItems::Center,
            ..default()
        },
        children![label(text), hspace(10.0), button_small("", action)],
    )
}

pub fn decimal_input<C: Component>(text: impl Into<String>, val: f32, marker: C) -> impl Bundle {
    (
        Node {
            align_items: AlignItems::Center,
            ..default()
        },
        children![
            label(text),
            hspace(10.0),
            (
                Name::new("Number Input"),
                TextInputNode {
                    mode: TextInputMode::SingleLine,
                    filter: Some(TextInputFilter::Decimal),
                    max_chars: Some(5),
                    clear_on_submit: false,
                    ..Default::default()
                },
                TextInputPrompt::new(val.to_string()),
                TextInputContents::default(),
                TextFont::from_font_size(18.0),
                marker,
                Node {
                    margin: UiRect::top(Val::Px(5.0)),
                    width: Val::Px(100.),
                    height: Val::Px(25.),
                    ..default()
                },
            )
        ],
    )
}
