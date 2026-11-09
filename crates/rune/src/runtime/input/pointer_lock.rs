//! The pointer lock (`El::pointer_lock`): a drawn element that asks for it captures the pointer
//! when pressed, as a game's view or a 3D viewport does, and keeps it until Escape, the window
//! losing focus, or the element going away. While locked, the host hides the cursor, keeps it
//! in the window, and sends its raw motion here ([`Runtime::pointer_motion`]), which goes to the
//! element instead of moving a cursor.
//!
//! One machine, [`step`]: the host carries out its [`LockEffect`]s (`take_pointer_lock_effects`)
//! when it syncs a frame with the window.

use crate::machine::Effects;

use super::*;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum LockState {
    /// Nothing drawn asks for the lock.
    #[default]
    Off,
    /// An element asks for it; a press inside it captures the pointer.
    Ready,
    /// The pointer is captured, and its motion goes to the element.
    Locked,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LockEvent {
    /// Whether a drawn element asks for the lock, as of the last frame.
    Asked(bool),
    /// A press landed inside the asking element.
    Pressed,
    /// Escape, or the window losing focus: the pointer goes back to the user.
    Released,
}

/// What the window does for a lock change.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LockEffect {
    /// Capture the pointer: hide it, keep it in the window, report its raw motion.
    Grab,
    /// Give it back.
    Ungrab,
}

pub(crate) fn step(state: &mut LockState, event: LockEvent) -> Effects<LockEffect> {
    match (*state, event) {
        (LockState::Off, LockEvent::Asked(true)) => {
            *state = LockState::Ready;
            Effects::none()
        }
        (LockState::Ready, LockEvent::Asked(false)) => {
            *state = LockState::Off;
            Effects::none()
        }
        (LockState::Ready, LockEvent::Pressed) => {
            *state = LockState::Locked;
            Effects::one(LockEffect::Grab)
        }
        (LockState::Locked, LockEvent::Asked(false)) => {
            *state = LockState::Off;
            Effects::one(LockEffect::Ungrab)
        }
        (LockState::Locked, LockEvent::Released) => {
            *state = LockState::Ready;
            Effects::one(LockEffect::Ungrab)
        }
        _ => Effects::none(),
    }
}

/// The runtime's share of the lock: the machine, and the effects the host hasn't carried out.
#[derive(Default)]
pub(crate) struct PointerLock {
    state: LockState,
    effects: Vec<LockEffect>,
}

impl<S: 'static> Runtime<S> {
    /// Whether the pointer is captured for an element's [`El::pointer_lock`](crate::El::pointer_lock).
    pub fn pointer_locked(&self) -> bool {
        self.pointer_lock.state == LockState::Locked
    }

    /// Raw pointer motion in points, as a captured pointer reports it: it goes to the element
    /// holding the lock, and nowhere while the pointer isn't captured. Hosts call it; so can
    /// tests and scripts.
    pub fn pointer_motion(&mut self, dx: f32, dy: f32) {
        if !self.pointer_locked() || (dx == 0.0 && dy == 0.0) { return; }
        let Some(i) = self.pointer_lock_target() else { return };
        self.queue.push(Action::PointerLockMotion(self.nodes[i].path.clone(), [dx, dy]));
    }

    /// The grab changes the window owes the lock since it last asked, in order.
    pub(crate) fn take_pointer_lock_effects(&mut self) -> Vec<LockEffect> {
        self.sync_pointer_lock();
        std::mem::take(&mut self.pointer_lock.effects)
    }

    /// The topmost drawn element asking for the lock that a modal doesn't cover.
    fn pointer_lock_target(&self) -> Option<usize> {
        (0..self.nodes.len().min(self.seen.len())).rev().find(|&i| {
            self.seen[i]
                && self.nodes[i].handlers.pointer_lock.is_some()
                && self.allowed(i)
                && !self.subtree_exiting(i)
        })
    }

    fn step_pointer_lock(&mut self, event: LockEvent) {
        let was = self.pointer_locked();
        let effects = step(&mut self.pointer_lock.state, event);
        self.pointer_lock.effects.extend(effects);
        // The view reads it (`Cx::pointer_locked`).
        self.dirty |= was != self.pointer_locked();
    }

    /// After a frame: the lock follows whether a drawn element still asks for it.
    pub(super) fn sync_pointer_lock(&mut self) {
        let asked = self.pointer_lock_target().is_some();
        self.step_pointer_lock(LockEvent::Asked(asked));
    }

    /// A press at `x`, `y` captures the pointer when it lands in the asking element.
    pub(super) fn pointer_lock_press(&mut self, x: f32, y: f32) {
        self.sync_pointer_lock();
        let Some(i) = self.pointer_lock_target() else { return };
        let [left, top, width, height] = self.drawn[i];
        if x >= left && x < left + width && y >= top && y < top + height {
            self.step_pointer_lock(LockEvent::Pressed);
        }
    }

    /// Escape or a lost focus gives the pointer back; true when it was captured.
    pub(super) fn release_pointer_lock(&mut self) -> bool {
        let locked = self.pointer_locked();
        self.step_pointer_lock(LockEvent::Released);
        locked
    }
}

#[cfg(test)]
#[path = "pointer_lock_tests.rs"]
mod tests;
