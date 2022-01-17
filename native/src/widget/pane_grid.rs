//! Let your users split regions of your application and organize layout dynamically.
//!
//! [![Pane grid - Iced](https://thumbs.gfycat.com/MixedFlatJellyfish-small.gif)](https://gfycat.com/mixedflatjellyfish)
//!
//! # Example
//! The [`pane_grid` example] showcases how to use a [`PaneGrid`] with resizing,
//! drag and drop, and hotkey support.
//!
//! [`pane_grid` example]: https://github.com/iced-rs/iced/tree/0.3/examples/pane_grid
mod axis;
mod configuration;
mod content;
mod direction;
mod node;
mod pane;
mod split;
mod state;
mod title_bar;

pub use axis::Axis;
pub use configuration::Configuration;
pub use content::Content;
pub use direction::Direction;
pub use node::Node;
pub use pane::Pane;
pub use split::Split;
pub use state::State;
pub use title_bar::TitleBar;

use crate::event::{self, Event};
use crate::layout;
use crate::mouse;
use crate::overlay;
use crate::renderer;
use crate::touch;
use crate::{
    Clipboard, Color, Element, Hasher, Layout, Length, Point, Rectangle, Shell,
    Size, Vector, Widget,
};

pub use iced_style::pane_grid::{Line, StyleSheet};

/// A collection of panes distributed using either vertical or horizontal splits
/// to completely fill the space available.
///
/// [![Pane grid - Iced](https://thumbs.gfycat.com/FrailFreshAiredaleterrier-small.gif)](https://gfycat.com/frailfreshairedaleterrier)
///
/// This distribution of space is common in tiling window managers (like
/// [`awesome`](https://awesomewm.org/), [`i3`](https://i3wm.org/), or even
/// [`tmux`](https://github.com/tmux/tmux)).
///
/// A [`PaneGrid`] supports:
///
/// * Vertical and horizontal splits
/// * Tracking of the last active pane
/// * Mouse-based resizing
/// * Drag and drop to reorganize panes
/// * Hotkey support
/// * Configurable modifier keys
/// * [`State`] API to perform actions programmatically (`split`, `swap`, `resize`, etc.)
///
/// ## Example
///
/// ```
/// # use iced_native::widget::{pane_grid, Text};
/// #
/// # type PaneGrid<'a, Message> =
/// #     iced_native::widget::PaneGrid<'a, Message, iced_native::renderer::Null>;
/// #
/// enum PaneState {
///     SomePane,
///     AnotherKindOfPane,
/// }
///
/// enum Message {
///     PaneDragged(pane_grid::DragEvent),
///     PaneResized(pane_grid::ResizeEvent),
/// }
///
/// let (mut state, _) = pane_grid::State::new(PaneState::SomePane);
///
/// let pane_grid =
///     PaneGrid::new(&mut state, |pane, state| {
///         pane_grid::Content::new(match state {
///             PaneState::SomePane => Text::new("This is some pane"),
///             PaneState::AnotherKindOfPane => Text::new("This is another kind of pane"),
///         })
///     })
///     .on_drag(Message::PaneDragged)
///     .on_resize(10, Message::PaneResized);
/// ```
#[allow(missing_debug_implementations)]
pub struct PaneGrid<'a, Message, Renderer> {
    state: &'a mut state::Internal,
    elements: Vec<(Pane, Content<'a, Message, Renderer>)>,
    width: Length,
    height: Length,
    spacing: u16,
    on_click: Option<Box<dyn Fn(Pane) -> Message + 'a>>,
    on_drag: Option<Box<dyn Fn(DragEvent) -> Message + 'a>>,
    on_resize: Option<(u16, Box<dyn Fn(ResizeEvent) -> Message + 'a>)>,
    style_sheet: Box<dyn StyleSheet + 'a>,
}

impl<'a, Message, Renderer> PaneGrid<'a, Message, Renderer>
where
    Renderer: crate::Renderer,
{
    /// Creates a [`PaneGrid`] with the given [`State`] and view function.
    ///
    /// The view function will be called to display each [`Pane`] present in the
    /// [`State`].
    pub fn new<T>(
        state: &'a mut State<T>,
        view: impl Fn(Pane, &'a mut T) -> Content<'a, Message, Renderer>,
    ) -> Self {
        let elements = {
            state
                .panes
                .iter_mut()
                .map(|(pane, pane_state)| (*pane, view(*pane, pane_state)))
                .collect()
        };

        Self {
            state: &mut state.internal,
            elements,
            width: Length::Fill,
            height: Length::Fill,
            spacing: 0,
            on_click: None,
            on_drag: None,
            on_resize: None,
            style_sheet: Default::default(),
        }
    }

    /// Sets the width of the [`PaneGrid`].
    pub fn width(mut self, width: Length) -> Self {
        self.width = width;
        self
    }

    /// Sets the height of the [`PaneGrid`].
    pub fn height(mut self, height: Length) -> Self {
        self.height = height;
        self
    }

    /// Sets the spacing _between_ the panes of the [`PaneGrid`].
    pub fn spacing(mut self, units: u16) -> Self {
        self.spacing = units;
        self
    }

    /// Sets the message that will be produced when a [`Pane`] of the
    /// [`PaneGrid`] is clicked.
    pub fn on_click<F>(mut self, f: F) -> Self
    where
        F: 'a + Fn(Pane) -> Message,
    {
        self.on_click = Some(Box::new(f));
        self
    }

    /// Enables the drag and drop interactions of the [`PaneGrid`], which will
    /// use the provided function to produce messages.
    pub fn on_drag<F>(mut self, f: F) -> Self
    where
        F: 'a + Fn(DragEvent) -> Message,
    {
        self.on_drag = Some(Box::new(f));
        self
    }

    /// Enables the resize interactions of the [`PaneGrid`], which will
    /// use the provided function to produce messages.
    ///
    /// The `leeway` describes the amount of space around a split that can be
    /// used to grab it.
    ///
    /// The grabbable area of a split will have a length of `spacing + leeway`,
    /// properly centered. In other words, a length of
    /// `(spacing + leeway) / 2.0` on either side of the split line.
    pub fn on_resize<F>(mut self, leeway: u16, f: F) -> Self
    where
        F: 'a + Fn(ResizeEvent) -> Message,
    {
        self.on_resize = Some((leeway, Box::new(f)));
        self
    }

    /// Sets the style of the [`PaneGrid`].
    pub fn style(mut self, style: impl Into<Box<dyn StyleSheet + 'a>>) -> Self {
        self.style_sheet = style.into();
        self
    }
}

impl<'a, Message, Renderer> PaneGrid<'a, Message, Renderer>
where
    Renderer: crate::Renderer,
{
    fn click_pane(
        &mut self,
        layout: Layout<'_>,
        cursor_position: Point,
        shell: &mut Shell<'_, Message>,
    ) {
        let mut clicked_region =
            self.elements.iter().zip(layout.children()).filter(
                |(_, layout)| layout.bounds().contains(cursor_position),
            );

        if let Some(((pane, content), layout)) = clicked_region.next() {
            if let Some(on_click) = &self.on_click {
                shell.publish(on_click(*pane));
            }

            if let Some(on_drag) = &self.on_drag {
                if content.can_be_picked_at(layout, cursor_position) {
                    let pane_position = layout.position();

                    let origin = cursor_position
                        - Vector::new(pane_position.x, pane_position.y);

                    self.state.pick_pane(pane, origin);

                    shell.publish(on_drag(DragEvent::Picked { pane: *pane }));
                }
            }
        }
    }

    fn trigger_resize(
        &mut self,
        layout: Layout<'_>,
        cursor_position: Point,
        shell: &mut Shell<'_, Message>,
    ) -> event::Status {
        if let Some((_, on_resize)) = &self.on_resize {
            if let Some((split, _)) = self.state.picked_split() {
                let bounds = layout.bounds();

                let splits = self.state.split_regions(
                    f32::from(self.spacing),
                    Size::new(bounds.width, bounds.height),
                );

                if let Some((axis, rectangle, _)) = splits.get(&split) {
                    let ratio = match axis {
                        Axis::Horizontal => {
                            let position =
                                cursor_position.y - bounds.y - rectangle.y;

                            (position / rectangle.height).max(0.1).min(0.9)
                        }
                        Axis::Vertical => {
                            let position =
                                cursor_position.x - bounds.x - rectangle.x;

                            (position / rectangle.width).max(0.1).min(0.9)
                        }
                    };

                    shell.publish(on_resize(ResizeEvent { split, ratio }));

                    return event::Status::Captured;
                }
            }
        }

        event::Status::Ignored
    }
}

/// An event produced during a drag and drop interaction of a [`PaneGrid`].
#[derive(Debug, Clone, Copy)]
pub enum DragEvent {
    /// A [`Pane`] was picked for dragging.
    Picked {
        /// The picked [`Pane`].
        pane: Pane,
    },

    /// A [`Pane`] was dropped on top of another [`Pane`].
    Dropped {
        /// The picked [`Pane`].
        pane: Pane,

        /// The [`Pane`] where the picked one was dropped on.
        target: Pane,
    },

    /// A [`Pane`] was picked and then dropped outside of other [`Pane`]
    /// boundaries.
    Canceled {
        /// The picked [`Pane`].
        pane: Pane,
    },
}

/// An event produced during a resize interaction of a [`PaneGrid`].
#[derive(Debug, Clone, Copy)]
pub struct ResizeEvent {
    /// The [`Split`] that is being dragged for resizing.
    pub split: Split,

    /// The new ratio of the [`Split`].
    ///
    /// The ratio is a value in [0, 1], representing the exact position of a
    /// [`Split`] between two panes.
    pub ratio: f32,
}

impl<'a, Message, Renderer> Widget<Message, Renderer>
    for PaneGrid<'a, Message, Renderer>
where
    Renderer: crate::Renderer,
{
    fn width(&self) -> Length {
        self.width
    }

    fn height(&self) -> Length {
        self.height
    }

    fn layout(
        &self,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let limits = limits.width(self.width).height(self.height);
        let size = limits.resolve(Size::ZERO);

        let regions = self.state.pane_regions(f32::from(self.spacing), size);

        let children = self
            .elements
            .iter()
            .filter_map(|(pane, element)| {
                let region = regions.get(pane)?;
                let size = Size::new(region.width, region.height);

                let mut node =
                    element.layout(renderer, &layout::Limits::new(size, size));

                node.move_to(Point::new(region.x, region.y));

                Some(node)
            })
            .collect();

        layout::Node::with_children(size, children)
    }

    fn on_event(
        &mut self,
        event: Event,
        layout: Layout<'_>,
        cursor_position: Point,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
    ) -> event::Status {
        let mut event_status = event::Status::Ignored;

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
            | Event::Touch(touch::Event::FingerPressed { .. }) => {
                let bounds = layout.bounds();

                if bounds.contains(cursor_position) {
                    event_status = event::Status::Captured;

                    match self.on_resize {
                        Some((leeway, _)) => {
                            let relative_cursor = Point::new(
                                cursor_position.x - bounds.x,
                                cursor_position.y - bounds.y,
                            );

                            let splits = self.state.split_regions(
                                f32::from(self.spacing),
                                Size::new(bounds.width, bounds.height),
                            );

                            let clicked_split = hovered_split(
                                splits.iter(),
                                f32::from(self.spacing + leeway),
                                relative_cursor,
                            );

                            if let Some((split, axis, _)) = clicked_split {
                                self.state.pick_split(&split, axis);
                            } else {
                                self.click_pane(layout, cursor_position, shell);
                            }
                        }
                        None => {
                            self.click_pane(layout, cursor_position, shell);
                        }
                    }
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
            | Event::Touch(touch::Event::FingerLifted { .. })
            | Event::Touch(touch::Event::FingerLost { .. }) => {
                if let Some((pane, _)) = self.state.picked_pane() {
                    if let Some(on_drag) = &self.on_drag {
                        let mut dropped_region =
                            self.elements.iter().zip(layout.children()).filter(
                                |(_, layout)| {
                                    layout.bounds().contains(cursor_position)
                                },
                            );

                        let event = match dropped_region.next() {
                            Some(((target, _), _)) if pane != *target => {
                                DragEvent::Dropped {
                                    pane,
                                    target: *target,
                                }
                            }
                            _ => DragEvent::Canceled { pane },
                        };

                        shell.publish(on_drag(event));
                    }

                    self.state.idle();

                    event_status = event::Status::Captured;
                } else if self.state.picked_split().is_some() {
                    self.state.idle();

                    event_status = event::Status::Captured;
                }
            }
            Event::Mouse(mouse::Event::CursorMoved { .. })
            | Event::Touch(touch::Event::FingerMoved { .. }) => {
                event_status =
                    self.trigger_resize(layout, cursor_position, shell);
            }
            _ => {}
        }

        let picked_pane = self.state.picked_pane().map(|(pane, _)| pane);

        self.elements
            .iter_mut()
            .zip(layout.children())
            .map(|((pane, content), layout)| {
                let is_picked = picked_pane == Some(*pane);

                content.on_event(
                    event.clone(),
                    layout,
                    cursor_position,
                    renderer,
                    clipboard,
                    shell,
                    is_picked,
                )
            })
            .fold(event_status, event::Status::merge)
    }

    fn mouse_interaction(
        &self,
        layout: Layout<'_>,
        cursor_position: Point,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        if self.state.picked_pane().is_some() {
            return mouse::Interaction::Grab;
        }

        let resize_axis =
            self.state.picked_split().map(|(_, axis)| axis).or_else(|| {
                self.on_resize.as_ref().and_then(|(leeway, _)| {
                    let bounds = layout.bounds();

                    let splits = self
                        .state
                        .split_regions(f32::from(self.spacing), bounds.size());

                    let relative_cursor = Point::new(
                        cursor_position.x - bounds.x,
                        cursor_position.y - bounds.y,
                    );

                    hovered_split(
                        splits.iter(),
                        f32::from(self.spacing + leeway),
                        relative_cursor,
                    )
                    .map(|(_, axis, _)| axis)
                })
            });

        if let Some(resize_axis) = resize_axis {
            return match resize_axis {
                Axis::Horizontal => mouse::Interaction::ResizingVertically,
                Axis::Vertical => mouse::Interaction::ResizingHorizontally,
            };
        }

        self.elements
            .iter()
            .zip(layout.children())
            .map(|((_pane, content), layout)| {
                content.mouse_interaction(
                    layout,
                    cursor_position,
                    viewport,
                    renderer,
                )
            })
            .max()
            .unwrap_or_default()
    }

    fn draw(
        &self,
        renderer: &mut Renderer,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor_position: Point,
        viewport: &Rectangle,
    ) {
        let picked_pane = self.state.picked_pane();

        let picked_split = self
            .state
            .picked_split()
            .and_then(|(split, axis)| {
                let bounds = layout.bounds();

                let splits = self
                    .state
                    .split_regions(f32::from(self.spacing), bounds.size());

                let (_axis, region, ratio) = splits.get(&split)?;

                let region = axis.split_line_bounds(
                    *region,
                    *ratio,
                    f32::from(self.spacing),
                );

                Some((axis, region + Vector::new(bounds.x, bounds.y), true))
            })
            .or_else(|| match self.on_resize {
                Some((leeway, _)) => {
                    let bounds = layout.bounds();

                    let relative_cursor = Point::new(
                        cursor_position.x - bounds.x,
                        cursor_position.y - bounds.y,
                    );

                    let splits = self
                        .state
                        .split_regions(f32::from(self.spacing), bounds.size());

                    let (_split, axis, region) = hovered_split(
                        splits.iter(),
                        f32::from(self.spacing + leeway),
                        relative_cursor,
                    )?;

                    Some((
                        axis,
                        region + Vector::new(bounds.x, bounds.y),
                        false,
                    ))
                }
                None => None,
            });

        let pane_cursor_position = if picked_pane.is_some() {
            // TODO: Remove once cursor availability is encoded in the type
            // system
            Point::new(-1.0, -1.0)
        } else {
            cursor_position
        };

        for ((id, pane), layout) in self.elements.iter().zip(layout.children())
        {
            match picked_pane {
                Some((dragging, origin)) if *id == dragging => {
                    let bounds = layout.bounds();

                    renderer.with_translation(
                        cursor_position
                            - Point::new(
                                bounds.x + origin.x,
                                bounds.y + origin.y,
                            ),
                        |renderer| {
                            renderer.with_layer(bounds, |renderer| {
                                pane.draw(
                                    renderer,
                                    style,
                                    layout,
                                    pane_cursor_position,
                                    viewport,
                                );
                            });
                        },
                    );
                }
                _ => {
                    pane.draw(
                        renderer,
                        style,
                        layout,
                        pane_cursor_position,
                        viewport,
                    );
                }
            }
        }

        if let Some((axis, split_region, is_picked)) = picked_split {
            let highlight = if is_picked {
                self.style_sheet.picked_split()
            } else {
                self.style_sheet.hovered_split()
            };

            if let Some(highlight) = highlight {
                renderer.fill_quad(
                    renderer::Quad {
                        bounds: match axis {
                            Axis::Horizontal => Rectangle {
                                x: split_region.x,
                                y: (split_region.y
                                    + (split_region.height - highlight.width)
                                        / 2.0)
                                    .round(),
                                width: split_region.width,
                                height: highlight.width,
                            },
                            Axis::Vertical => Rectangle {
                                x: (split_region.x
                                    + (split_region.width - highlight.width)
                                        / 2.0)
                                    .round(),
                                y: split_region.y,
                                width: highlight.width,
                                height: split_region.height,
                            },
                        },
                        border_radius: 0.0,
                        border_width: 0.0,
                        border_color: Color::TRANSPARENT,
                    },
                    highlight.color,
                );
            }
        }
    }

    fn hash_layout(&self, state: &mut Hasher) {
        use std::hash::Hash;

        struct Marker;
        std::any::TypeId::of::<Marker>().hash(state);

        self.width.hash(state);
        self.height.hash(state);
        self.state.hash_layout(state);

        for (_, element) in &self.elements {
            element.hash_layout(state);
        }
    }

    fn overlay(
        &mut self,
        layout: Layout<'_>,
        renderer: &Renderer,
    ) -> Option<overlay::Element<'_, Message, Renderer>> {
        self.elements
            .iter_mut()
            .zip(layout.children())
            .filter_map(|((_, pane), layout)| pane.overlay(layout, renderer))
            .next()
    }
}

impl<'a, Message, Renderer> From<PaneGrid<'a, Message, Renderer>>
    for Element<'a, Message, Renderer>
where
    Renderer: 'a + crate::Renderer,
    Message: 'a,
{
    fn from(
        pane_grid: PaneGrid<'a, Message, Renderer>,
    ) -> Element<'a, Message, Renderer> {
        Element::new(pane_grid)
    }
}

/*
 * Helpers
 */
fn hovered_split<'a>(
    splits: impl Iterator<Item = (&'a Split, &'a (Axis, Rectangle, f32))>,
    spacing: f32,
    cursor_position: Point,
) -> Option<(Split, Axis, Rectangle)> {
    splits
        .filter_map(|(split, (axis, region, ratio))| {
            let bounds =
                axis.split_line_bounds(*region, *ratio, f32::from(spacing));

            if bounds.contains(cursor_position) {
                Some((*split, *axis, bounds))
            } else {
                None
            }
        })
        .next()
}
