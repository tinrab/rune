//! Mouse look with `El::pointer_lock`: click the view to capture the pointer, move the mouse to
//! look around a ring of markers, and press Escape to get the pointer back. `cargo run -p
//! mouse-look`; `-- --snap look.png --scene turned` renders it headless, turned to the right.

use rune::tok::{self, type_};
use rune::{AppSpec, Cx, El, col, el, text};

// Degrees of turn per point of pointer motion.
const SENSITIVITY: f32 = 0.15;
// How many degrees the view shows across.
const FIELD: f32 = 90.0;
const MARKERS: [(&str, f32); 8] =
    [("N", 0.0), ("NE", 45.0), ("E", 90.0), ("SE", 135.0), ("S", 180.0), ("SW", 225.0), ("W", 270.0), ("NW", 315.0)];

#[derive(Default)]
struct State {
    yaw: f32,
    pitch: f32,
}

impl State {
    fn look(&mut self, [dx, dy]: [f32; 2]) {
        self.yaw = (self.yaw + dx * SENSITIVITY).rem_euclid(360.0);
        self.pitch = (self.pitch - dy * SENSITIVITY).clamp(-60.0, 60.0);
    }
}

fn view(s: &State, cx: &mut Cx<State>) -> El {
    let t = tok::theme(true);
    let (w, h) = (640.0, 360.0);
    let horizon = h / 2.0 + s.pitch / FIELD * w;
    let mut world = el().key("world").size(w, h).clip_rounded(12.0).fill(t.bg)
        .child(el().key("horizon").absolute(0.0, horizon).size(w, 1.0).fill(t.text.alpha(0.3)));
    for (name, at) in MARKERS {
        // Where the marker sits across the view, from -180 to 180 degrees off centre.
        let off = (at - s.yaw + 540.0).rem_euclid(360.0) - 180.0;
        if off.abs() < FIELD {
            let x = w / 2.0 + off / FIELD * w;
            world = world.child(text(name).key(format!("marker-{name}")).font(type_::TITLE).color(t.text).absolute(x - 12.0, horizon - 40.0));
        }
    }
    let hint = if cx.pointer_locked() { "Escape gives the pointer back" } else { "Click to look around" };
    let world = world
        .child(el().key("crosshair").absolute(w / 2.0 - 3.0, h / 2.0 - 3.0).size(6.0, 6.0).radius(3.0).fill(tok::accent(0, true)))
        .pointer_lock(|s: &mut State, delta, _| s.look(delta));
    col().w_full().h_full().center().gap(12.0).fill(t.bg)
        .child(world)
        .child(text(format!("{hint} · facing {:.0}°, pitch {:.0}°", s.yaw, s.pitch)).color(t.text))
}

fn main() {
    rune::run(AppSpec::<State> {
        scenes: &[("turned", |s| s.look([300.0, -40.0]))],
        ..AppSpec::new("Mouse Look", (720.0, 460.0), State::default, view)
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use rune::{Env, Runtime};

    #[test]
    fn a_click_captures_the_pointer_and_motion_turns_the_view() {
        let mut rt = Runtime::new(State::default(), view, (720.0, 460.0), Env::new(true, 1.0));
        rt.window_focus(true);
        rt.frame(0.0);
        rt.pointer_down(360.0, 230.0);
        rt.pointer_up(360.0, 230.0);
        rt.frame(0.0);
        assert!(rt.pointer_locked());
        rt.pointer_motion(100.0, 0.0);
        rt.frame(0.0);
        assert!((rt.state.yaw - 15.0).abs() < 1e-3, "{}", rt.state.yaw);
    }
}
