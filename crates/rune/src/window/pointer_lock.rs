//! The window's half of `El::pointer_lock`: grabbing and releasing the pointer as the runtime's
//! lock machine asks (`Runtime::take_pointer_lock_effect`), and the device's raw motion back
//! to the runtime while it holds the pointer.

use winit::window::CursorGrabMode;

use super::*;
use crate::runtime::LockEffect;

impl Win {
    /// Carries out the lock change the runtime owes the window.
    pub(super) fn apply_pointer_lock<S: 'static>(&self, rt: &mut Runtime<S>) {
        match rt.take_pointer_lock_effect() {
            Some(LockEffect::Grab) => grab(&self.window),
            // Failing to let go means nothing was grabbed.
            Some(LockEffect::Ungrab) => self.window.set_cursor_grab(CursorGrabMode::None).ok().unwrap_or_default(),
            None => {}
        }
    }
}

/// Locked keeps the pointer where it is (macOS, Wayland); where that's missing, Confined keeps
/// it in the window (X11, Windows). The motion comes from the device either way, so the hidden
/// cursor reaching an edge doesn't stop it, and the runtime keeps its pointer where the
/// capturing press was, so where the hidden cursor wanders doesn't matter.
fn grab(window: &Window) {
    let grabbed = window.set_cursor_grab(CursorGrabMode::Locked).or_else(|_| window.set_cursor_grab(CursorGrabMode::Confined));
    // The motion still arrives (it's the device's), but the hidden cursor can leave the window.
    if let Err(error) = grabbed { eprintln!("rune: could not capture the pointer: {error}"); }
}

/// Whether the platform reports a device's motion in points already: macOS gives the event's
/// delta in points, and Wayland's relative pointer in surface coordinates. X11 and Windows give
/// the device's own counts, which move the cursor about a pixel each.
fn motion_in_points(event_loop: &ActiveEventLoop) -> bool {
    cfg!(target_os = "macos") || is_wayland(event_loop)
}

impl<S: 'static> Host<S> {
    /// The device moved by `delta`, unaccelerated where the platform says so: while this
    /// window holds the pointer and has focus, the motion goes to the element that captured
    /// it, in about a point per unit.
    pub(super) fn pointer_motion_in_turn(&mut self, event_loop: &ActiveEventLoop, (dx, dy): (f64, f64)) {
        let Some(l) = &self.win else { return };
        if !self.rt.pointer_locked() || !l.window.has_focus() { return; }
        let scale = if motion_in_points(event_loop) { 1.0 } else { l.window.scale_factor() };
        self.rt.pointer_motion((dx / scale) as f32, (dy / scale) as f32);
        l.request_redraw();
    }
}
