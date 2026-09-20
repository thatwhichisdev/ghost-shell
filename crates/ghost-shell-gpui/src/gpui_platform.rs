//! Ghost's Wayland-only GPUI entry point.

extern crate self as ghost_shell_gpui;

#[cfg(not(target_os = "linux"))]
compile_error!("ghost-shell-gpui supports Linux with Wayland only");

mod linux;

pub use gpui::*;
pub use linux::current_platform;

/// Creates an application using the Wayland backend.
pub fn application() -> Application {
    Application::with_platform(current_platform(false))
}

/// Creates an application without a display connection.
pub fn headless() -> Application {
    Application::with_platform(current_platform(true))
}

/// Creates a background executor without opening a display connection.
pub fn background_executor() -> BackgroundExecutor {
    current_platform(true).background_executor()
}

/// Headless tests use the simulated renderer; native offscreen GPU rendering
/// is not currently implemented by this backend.
#[cfg(any(feature = "bench-support", feature = "test-support"))]
pub fn current_headless_renderer() -> Option<Box<dyn PlatformHeadlessRenderer>> {
    None
}
