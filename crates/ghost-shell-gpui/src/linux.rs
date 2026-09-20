mod dispatcher;
mod headless;
mod keyboard;
mod platform;
mod system_notifications;
mod text_system;
mod wayland;
mod xdg_desktop_portal;

use std::rc::Rc;

pub use dispatcher::*;
pub(crate) use headless::*;
pub(crate) use keyboard::*;
pub(crate) use platform::*;
pub(crate) use text_system::*;
pub(crate) use wayland::*;

/// Creates either a Wayland platform or an explicitly requested headless platform.
pub fn current_platform(headless: bool) -> Rc<dyn gpui::Platform> {
    if headless {
        Rc::new(LinuxPlatform {
            inner: HeadlessClient::new(),
        })
    } else {
        Rc::new(LinuxPlatform {
            inner: WaylandClient::new(),
        })
    }
}
