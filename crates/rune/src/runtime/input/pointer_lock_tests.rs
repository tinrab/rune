use super::*;
use crate::el::{col, el};
use crate::{Key, KeyEvent, Mods};

#[derive(Default)]
struct Game {
    // The view asks for the pointer.
    playing: bool,
    turned: [f32; 2],
    presses: u32,
    keys: Vec<Key>,
}

fn view(s: &Game, _: &mut Cx<Game>) -> El {
    let mut viewport = el().key("viewport").size(300.0, 160.0).on_press(|s: &mut Game| s.presses += 1);
    if s.playing {
        viewport = viewport.pointer_lock(|s: &mut Game, [dx, dy], _| {
            s.turned[0] += dx;
            s.turned[1] += dy;
        });
    }
    col().size(300.0, 200.0).children([el().key("toolbar").size(300.0, 40.0), viewport])
}

fn game() -> Runtime<Game> {
    let mut rt = Runtime::new(Game { playing: true, ..Default::default() }, view, (300.0, 200.0), Env::new(true, 1.0));
    rt.on_key = Some(|s, ev, _| { s.keys.push(ev.key.clone()); true });
    rt.window_focus(true);
    rt.frame(0.0);
    rt
}

fn click(rt: &mut Runtime<Game>, x: f32, y: f32) {
    rt.pointer_down(x, y);
    rt.pointer_up(x, y);
    rt.frame(0.0);
}

#[test]
fn the_machine_grabs_on_a_press_and_lets_go_on_release_or_when_unasked() {
    let mut state = LockState::Off;
    assert_eq!(step(&mut state, LockEvent::Pressed), Effects::none(), "nothing asks yet");
    assert_eq!(step(&mut state, LockEvent::Asked(true)), Effects::none());
    assert_eq!(state, LockState::Ready);
    assert_eq!(step(&mut state, LockEvent::Pressed), Effects::one(LockEffect::Grab));
    assert_eq!(step(&mut state, LockEvent::Pressed), Effects::none(), "already held");
    assert_eq!(step(&mut state, LockEvent::Released), Effects::one(LockEffect::Ungrab));
    assert_eq!(state, LockState::Ready, "the next press captures it again");
    step(&mut state, LockEvent::Pressed);
    assert_eq!(step(&mut state, LockEvent::Asked(false)), Effects::one(LockEffect::Ungrab));
    assert_eq!(state, LockState::Off);
}

#[test]
fn a_press_on_the_element_captures_the_pointer_and_its_motion_goes_there() {
    let mut rt = game();
    rt.pointer_motion(5.0, 5.0);
    rt.frame(0.0);
    assert_eq!(rt.state.turned, [0.0, 0.0], "no motion before the pointer is captured");

    click(&mut rt, 150.0, 20.0);
    assert!(!rt.pointer_locked(), "a press elsewhere doesn't capture it");
    click(&mut rt, 150.0, 100.0);
    assert!(rt.pointer_locked());
    assert_eq!(rt.state.presses, 1, "the capturing press reaches the element as usual");
    assert_eq!(rt.take_pointer_lock_effects(), [LockEffect::Grab]);
    assert!(rt.take_pointer_lock_effects().is_empty(), "each change once");

    rt.pointer_motion(12.0, -3.0);
    rt.pointer_motion(0.5, 0.0);
    rt.frame(0.0);
    assert_eq!(rt.state.turned, [12.5, -3.0]);
}

#[test]
fn escape_gives_the_pointer_back_and_goes_no_further() {
    let mut rt = game();
    click(&mut rt, 150.0, 100.0);
    rt.key(KeyEvent { key: Key::Escape, mods: Mods::default() });
    rt.frame(0.0);
    assert!(!rt.pointer_locked());
    assert!(rt.state.keys.is_empty(), "the Escape that released it isn't the app's");
    assert_eq!(rt.take_pointer_lock_effects(), [LockEffect::Grab, LockEffect::Ungrab]);

    rt.key(KeyEvent { key: Key::Escape, mods: Mods::default() });
    rt.frame(0.0);
    assert_eq!(rt.state.keys, [Key::Escape], "with nothing captured, Escape is the app's");
    rt.pointer_motion(4.0, 4.0);
    rt.frame(0.0);
    assert_eq!(rt.state.turned, [0.0, 0.0]);
}

#[test]
fn losing_focus_or_the_element_releases_it() {
    let mut rt = game();
    click(&mut rt, 150.0, 100.0);
    rt.window_focus(false);
    rt.frame(0.0);
    assert!(!rt.pointer_locked(), "focus loss gives it back");

    rt.window_focus(true);
    rt.frame(0.0);
    assert!(!rt.pointer_locked(), "focus coming back doesn't take it again");
    click(&mut rt, 150.0, 100.0);
    assert!(rt.pointer_locked());

    rt.update(|s, _| s.playing = false);
    rt.frame(0.0);
    assert!(!rt.pointer_locked(), "the element stopped asking");
    assert_eq!(rt.take_pointer_lock_effects().last(), Some(&LockEffect::Ungrab));
    click(&mut rt, 150.0, 100.0);
    assert!(!rt.pointer_locked());
}
