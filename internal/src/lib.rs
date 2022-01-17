mod application;
mod element;
mod error;
mod result;
mod sandbox;

pub mod clipboard;
pub mod executor;
pub mod keyboard;
pub mod mouse;
pub mod settings;
pub mod widget;
pub mod window;

#[cfg(all(
    any(feature = "tokio", feature = "async-std", feature = "smol"),
    not(target_arch = "wasm32")
))]
#[cfg_attr(
    docsrs,
    doc(cfg(any(
        feature = "tokio",
        feature = "async-std"
        feature = "smol"
    )))
)]
pub mod time;

#[cfg(all(
    not(target_arch = "wasm32"),
    not(feature = "glow"),
    feature = "wgpu"
))]
use iced_winit as runtime;

#[cfg(all(not(target_arch = "wasm32"), feature = "glow"))]
use iced_glutin as runtime;

#[cfg(all(
    not(target_arch = "wasm32"),
    not(feature = "glow"),
    feature = "wgpu"
))]
use iced_wgpu as renderer;

#[cfg(all(not(target_arch = "wasm32"), feature = "glow"))]
use iced_glow as renderer;

#[cfg(target_arch = "wasm32")]
use iced_web as runtime;

#[doc(no_inline)]
pub use widget::*;

pub use application::Application;
pub use element::Element;
pub use error::Error;
pub use executor::Executor;
pub use result::Result;
pub use sandbox::Sandbox;
pub use settings::Settings;

pub use runtime::alignment;
pub use runtime::futures;
pub use runtime::{
    Alignment, Background, Color, Command, Font, Length, Point, Rectangle,
    Size, Subscription, Vector,
};