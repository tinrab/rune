//! The pointer lock (`El::pointer_lock`): a drawn element that asks for it captures the pointer
//! when pressed, as a game's view or a 3D viewport does, and keeps it until Escape, the window
//! losing focus, or the element going away. While locked, the host hides the cursor, keeps it
//! in the window, and sends its raw motion here ([`Runtime::pointer_motion`]), which goes to the
//! element instead of moving a cursor. The pointer stays where the capturing press was, so
//! presses and wheel turns while locked land on the element too, wherever the hidden cursor is.
//!
//! One machine, [`step`]: the host carries out its [`LockEffect`] (`take_pointer_lock_effect`)
//! when it syncs a frame with the window.

use crate::machine::Effects;

use super::*;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum LockState {
    /// Nothing drawn asks for the lock.
    #[default]
    Off,
    /// An element asks for it; a press on it captures the pointer.
    Ready,
    /// The pointer is captured, and its motion goes to the element.
    Locked,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LockEvent {
    /// Whether a drawn element asks for the lock, as of the last frame.
    Asked(bool),
    /// A press landed on the asking element.
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

/// The runtime's share of the lock: the machine, and the change the host hasn't carried out.
#[derive(Default)]
pub(crate) struct PointerLock {
    state: LockState,
    /// Grabs and ungrabs alternate, so a pair the host never saw cancels out and one change
    /// at most is owed, with or without a host draining it.
    owed: Option<LockEffect>,
}

impl PointerLock {
    fn owe(&mut self, effect: LockEffect) {
        self.owed = match self.owed {
            Some(owed) if owed != effect => None,
            _ => Some(effect),
        };
    }
}

impl<S: 'static> Runtime<S> {
    /// Whether the pointer is captured for an element's [`El::pointer_lock`](crate::El::pointer_lock).
    pub fn pointer_locked(&self) -> bool {
        self.pointer_lock.state == LockState::Locked
    }

    /// Raw pointer motion, as a captured pointer reports it: it goes to the element holding
    /// the lock, and nowhere while the pointer isn't captured. Hosts call it, about a point
    /// per unit (see [`El::pointer_lock`](crate::El::pointer_lock)); so can tests, and scripts
    /// with `{"type": "pointer-motion"}`.
    pub fn pointer_motion(&mut self, dx: f32, dy: f32) {
        if !self.pointer_locked() || (dx == 0.0 && dy == 0.0) { return; }
        let Some(i) = self.pointer_lock_target() else { return };
        // A fast mouse reports far more often than frames draw: motion still waiting for the
        // element adds up into one call.
        if let Some(Action::PointerLockMotion(path, [x, y])) = self.queue.last_mut()
            && *path == self.nodes[i].path
        {
            *x += dx;
            *y += dy;
            return;
        }
        self.queue.push(Action::PointerLockMotion(self.nodes[i].path.clone(), [dx, dy]));
    }

    /// Where a press at `x`, `y` lands: while the pointer is captured, where the capturing
    /// press was (`pointer_move` keeps it there), so every button reaches the element.
    pub(super) fn locked_at(&self, x: f32, y: f32) -> (f32, f32) {
        if self.pointer_locked() { self.pointer.unwrap_or((x, y)) } else { (x, y) }
    }

    /// The grab change the window owes the lock since it last asked.
    pub(crate) fn take_pointer_lock_effect(&mut self) -> Option<LockEffect> {
        self.sync_pointer_lock();
        self.pointer_lock.owed.take()
    }

    /// The topmost drawn element asking for the lock that a modal doesn't cover. Only what
    /// takes hits can ask, so it looks through those, not every node.
    fn pointer_lock_target(&self) -> Option<usize> {
        self.hits.iter().rev().find_map(|&(id, _)| {
            let i = *self.node_index.get(&id)?;
            let asks = self.nodes[i].handlers.pointer_lock.is_some();
            (asks && self.allowed(i) && !self.subtree_exiting(i)).then_some(i)
        })
    }

    fn step_pointer_lock(&mut self, event: LockEvent) {
        let was = self.pointer_locked();
        for effect in step(&mut self.pointer_lock.state, event) { self.pointer_lock.owe(effect); }
        // The view reads it (`Cx::pointer_locked`).
        self.dirty |= was != self.pointer_locked();
    }

    /// After a frame: the lock follows whether a drawn element still asks for it.
    pub(super) fn sync_pointer_lock(&mut self) {
        let asked = self.pointer_lock_target().is_some();
        self.step_pointer_lock(LockEvent::Asked(asked));
    }

    /// A press at `x`, `y` captures the pointer when it lands on the asking element: not on
    /// something drawn over it, nor on a control inside it that takes the press itself.
    pub(super) fn pointer_lock_press(&mut self, x: f32, y: f32) {
        self.sync_pointer_lock();
        let Some(i) = self.pointer_lock_target() else { return };
        if self.primary_target(x, y) == Some(i) { self.step_pointer_lock(LockEvent::Pressed); }
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
