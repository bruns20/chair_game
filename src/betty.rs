//! Scripted test runs, so Betty (the coding assistant) can see the game: built only with
//! `--features betty`, and only active when a script is passed:
//!
//!     cargo run --features betty -- betty/walk.txt
//!
//! The game runs as usual, but time advances a fixed 1/60 s per frame (so a run looks the same
//! every time), input comes from the script, screenshots are saved to betty/shots/, and the game
//! exits by itself. Without the feature none of this is compiled in.
//!
//! It knows nothing about the game. Input goes in as the raw messages the window itself sends
//! (`KeyboardInput`, `MouseButtonInput`, `MouseMotion`), so whatever reads input sees it: Bevy's
//! `ButtonInput` and `AccumulatedMouseMotion`, and crates that read the messages directly, like
//! bevy_fps_controller. Swapping the controller, physics or input crate needs no change here.
//! The only coupling is the hook in main.rs: `app.add_plugins(betty::BettyPlugin)` under
//! `#[cfg(feature = "betty")]`. Without it a scripted run never prints `betty:` and never exits.
//!
//! A script is one command per line; `#` starts a comment. Frames count from 0.
//!
//!     size 1280 720           window size (before frame 0)
//!     at 0 press MouseLeft    hold a key or mouse button (KeyW, Space, ShiftLeft, MouseLeft, ...)
//!     at 60 release KeyW      let it go
//!     at 30 mouse 200 -50     move the mouse by (dx, dy) on that frame
//!     at 90 shot after_walk   save betty/shots/after_walk.png
//!     at 90 dump              print the camera's and every named entity's transform
//!     at 120 quit             exit once the screenshots are written
//!
//! The run quits by itself 600 frames after the last command if the script has no `quit`.

use std::{
    collections::HashSet,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use bevy::{
    input::{
        ButtonState,
        keyboard::{Key, KeyboardInput, NativeKey},
        mouse::{MouseButtonInput, MouseMotion},
    },
    ecs::message::Messages,
    prelude::*,
    render::view::screenshot::{Screenshot, ScreenshotCaptured, save_to_disk},
    time::TimeUpdateStrategy,
    window::PrimaryWindow,
};

pub const SHOTS_DIR: &str = "betty/shots";
const FRAME: Duration = Duration::from_nanos(1_000_000_000 / 60);
const IDLE_QUIT_FRAMES: u32 = 600;

#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    Press(Button),
    Release(Button),
    Mouse(Vec2),
    Shot(String),
    Dump,
    Quit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Button {
    Key(KeyCode),
    Mouse(MouseButton),
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Script {
    pub size: Option<(u32, u32)>,
    /// (frame, action), sorted by frame, in script order within a frame.
    pub steps: Vec<(u32, Action)>,
}

impl Script {
    pub fn parse(text: &str) -> Result<Script, String> {
        let mut script = Script::default();
        for (n, raw) in text.lines().enumerate() {
            let line = raw.split('#').next().unwrap_or("").trim();
            if line.is_empty() {
                continue;
            }
            let err = |msg: &str| format!("line {}: {} ({:?})", n + 1, msg, raw.trim());
            let words: Vec<&str> = line.split_whitespace().collect();
            match words.as_slice() {
                ["size", w, h] => {
                    let w = w.parse().map_err(|_| err("bad width"))?;
                    let h = h.parse().map_err(|_| err("bad height"))?;
                    script.size = Some((w, h));
                }
                ["at", frame, rest @ ..] => {
                    let frame: u32 = frame.parse().map_err(|_| err("bad frame number"))?;
                    let action = match rest {
                        ["press", b] => Action::Press(parse_button(b).ok_or_else(|| err("unknown key"))?),
                        ["release", b] => Action::Release(parse_button(b).ok_or_else(|| err("unknown key"))?),
                        ["mouse", dx, dy] => Action::Mouse(Vec2::new(
                            dx.parse().map_err(|_| err("bad dx"))?,
                            dy.parse().map_err(|_| err("bad dy"))?,
                        )),
                        ["shot", name] => {
                            if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') {
                                return Err(err("a shot name is letters, digits, _ and -"));
                            }
                            Action::Shot(name.to_string())
                        }
                        ["dump"] => Action::Dump,
                        ["quit"] => Action::Quit,
                        _ => return Err(err("unknown command")),
                    };
                    script.steps.push((frame, action));
                }
                _ => return Err(err("expected `size W H` or `at FRAME ...`")),
            }
        }
        script.steps.sort_by_key(|(f, _)| *f);
        Ok(script)
    }

    fn last_frame(&self) -> u32 {
        self.steps.last().map(|(f, _)| *f).unwrap_or(0)
    }
}

pub fn parse_button(name: &str) -> Option<Button> {
    let mouse = match name {
        "MouseLeft" => Some(MouseButton::Left),
        "MouseRight" => Some(MouseButton::Right),
        "MouseMiddle" => Some(MouseButton::Middle),
        _ => None,
    };
    if let Some(m) = mouse {
        return Some(Button::Mouse(m));
    }
    let key = match name {
        "Space" => KeyCode::Space,
        "Enter" => KeyCode::Enter,
        "Escape" => KeyCode::Escape,
        "Tab" => KeyCode::Tab,
        "Backspace" => KeyCode::Backspace,
        "ShiftLeft" => KeyCode::ShiftLeft,
        "ShiftRight" => KeyCode::ShiftRight,
        "ControlLeft" => KeyCode::ControlLeft,
        "ControlRight" => KeyCode::ControlRight,
        "AltLeft" => KeyCode::AltLeft,
        "ArrowUp" => KeyCode::ArrowUp,
        "ArrowDown" => KeyCode::ArrowDown,
        "ArrowLeft" => KeyCode::ArrowLeft,
        "ArrowRight" => KeyCode::ArrowRight,
        _ => {
            let letter = name.strip_prefix("Key").filter(|s| s.len() == 1);
            let digit = name.strip_prefix("Digit").filter(|s| s.len() == 1);
            return match (letter, digit) {
                (Some(l), _) => key_letter(l.chars().next()?),
                (_, Some(d)) => key_digit(d.chars().next()?),
                _ => None,
            }
            .map(Button::Key);
        }
    };
    Some(Button::Key(key))
}

fn key_letter(c: char) -> Option<KeyCode> {
    use KeyCode::*;
    const KEYS: [KeyCode; 26] = [
        KeyA, KeyB, KeyC, KeyD, KeyE, KeyF, KeyG, KeyH, KeyI, KeyJ, KeyK, KeyL, KeyM, KeyN, KeyO, KeyP,
        KeyQ, KeyR, KeyS, KeyT, KeyU, KeyV, KeyW, KeyX, KeyY, KeyZ,
    ];
    c.is_ascii_uppercase().then(|| KEYS[(c as u8 - b'A') as usize])
}

fn key_digit(c: char) -> Option<KeyCode> {
    use KeyCode::*;
    const KEYS: [KeyCode; 10] = [Digit0, Digit1, Digit2, Digit3, Digit4, Digit5, Digit6, Digit7, Digit8, Digit9];
    c.is_ascii_digit().then(|| KEYS[(c as u8 - b'0') as usize])
}

/// Adds the scripted run when the first command-line argument is a script file; otherwise nothing.
pub struct BettyPlugin;

#[derive(Resource)]
struct Run {
    script: Script,
    next: usize,
    frame: u32,
    held: HashSet<Button>,
    quitting: bool,
    /// Screenshots asked for and not yet written.
    pending: Arc<AtomicUsize>,
}

impl Plugin for BettyPlugin {
    fn build(&self, app: &mut App) {
        let Some(path) = std::env::args().nth(1) else {
            return;
        };
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("betty: cannot read the script {path}: {e}"));
        let script = Script::parse(&text).unwrap_or_else(|e| panic!("betty: {path}: {e}"));
        std::fs::create_dir_all(SHOTS_DIR).expect("betty: cannot create betty/shots");
        println!("betty: running {path} ({} steps)", script.steps.len());
        app.insert_resource(TimeUpdateStrategy::ManualDuration(FRAME))
            .insert_resource(Run {
                script,
                next: 0,
                frame: 0,
                held: HashSet::new(),
                quitting: false,
                pending: Arc::new(AtomicUsize::new(0)),
            })
            .add_systems(Startup, size_window)
            // First: the messages are written before anything in the frame reads input.
            .add_systems(First, play);
    }
}

/// The message a real key or button change would have produced.
fn send(
    b: Button,
    state: ButtonState,
    repeat: bool,
    window: Entity,
    keys: &mut Messages<KeyboardInput>,
    buttons: &mut Messages<MouseButtonInput>,
) {
    match b {
        Button::Key(key_code) => {
            keys.write(KeyboardInput {
                key_code,
                logical_key: Key::Unidentified(NativeKey::Unidentified),
                state,
                text: None,
                repeat,
                window,
            });
        }
        Button::Mouse(button) => {
            buttons.write(MouseButtonInput { button, state, window });
        }
    }
}

fn size_window(run: Res<Run>, mut window: Single<&mut Window>) {
    window.title = "chair_game (scripted run)".into();
    if let Some((w, h)) = run.script.size {
        window.resolution.set(w as f32, h as f32);
    }
}

#[allow(clippy::too_many_arguments)]
fn play(
    mut commands: Commands,
    mut run: ResMut<Run>,
    window: Single<Entity, With<PrimaryWindow>>,
    key_state: Res<ButtonInput<KeyCode>>,
    button_state: Res<ButtonInput<MouseButton>>,
    mut keys: ResMut<Messages<KeyboardInput>>,
    mut buttons: ResMut<Messages<MouseButtonInput>>,
    mut motion: ResMut<Messages<MouseMotion>>,
    cameras: Query<(Entity, &Transform), With<Camera>>,
    named: Query<(&Name, &Transform)>,
    mut exit: MessageWriter<AppExit>,
) {
    let frame = run.frame;
    let window = *window;
    // Only the script plays: the real keyboard and mouse are dropped, or a run would depend on
    // what the person at the PC is doing (the window grabs the cursor, and it takes the focus).
    keys.clear();
    buttons.clear();
    motion.clear();
    // Held input stays held: losing window focus releases every key, so press again what the
    // script still holds (as a key repeat, which is what the OS sends for a held key).
    for b in run.held.clone() {
        let down = match b {
            Button::Key(k) => key_state.pressed(k),
            Button::Mouse(m) => button_state.pressed(m),
        };
        if !down && frame > 0 {
            send(b, ButtonState::Pressed, true, window, &mut keys, &mut buttons);
        }
    }
    while run.next < run.script.steps.len() && run.script.steps[run.next].0 <= frame {
        let action = run.script.steps[run.next].1.clone();
        run.next += 1;
        match action {
            Action::Press(b) => {
                run.held.insert(b);
                send(b, ButtonState::Pressed, false, window, &mut keys, &mut buttons);
            }
            Action::Release(b) => {
                run.held.remove(&b);
                send(b, ButtonState::Released, false, window, &mut keys, &mut buttons);
            }
            Action::Mouse(delta) => {
                motion.write(MouseMotion { delta });
            }
            Action::Shot(name) => {
                let path = PathBuf::from(SHOTS_DIR).join(format!("{name}.png"));
                let shown = path.display().to_string().replace('\\', "/");
                let pending = run.pending.clone();
                pending.fetch_add(1, Ordering::SeqCst);
                commands
                    .spawn(Screenshot::primary_window())
                    .observe(save_to_disk(path))
                    .observe(move |_: On<ScreenshotCaptured>| {
                        println!("betty: saved {shown}");
                        pending.fetch_sub(1, Ordering::SeqCst);
                    });
                println!("betty: frame {frame}: shot {name}");
            }
            Action::Dump => {
                for (e, t) in &cameras {
                    let (yaw, pitch, _) = t.rotation.to_euler(EulerRot::YXZ);
                    println!(
                        "betty: frame {frame}: camera {e} at {:.3} yaw {:.3} pitch {:.3}",
                        t.translation, yaw, pitch
                    );
                }
                for (name, t) in &named {
                    println!("betty: frame {frame}: {name} at {:.3}", t.translation);
                }
            }
            Action::Quit => run.quitting = true,
        }
    }
    if frame > run.script.last_frame() + IDLE_QUIT_FRAMES {
        run.quitting = true;
    }
    if run.quitting && run.pending.load(Ordering::SeqCst) == 0 {
        println!("betty: done at frame {frame}");
        exit.write(AppExit::Success);
    }
    run.frame += 1;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_script() {
        let s = Script::parse(
            "size 640 360\n# walk\nat 10 release KeyW\nat 0 press KeyW  # hold\nat 5 mouse 3 -4\nat 10 shot a_1\nat 12 quit\n",
        )
        .unwrap();
        assert_eq!(s.size, Some((640, 360)));
        assert_eq!(s.steps[0], (0, Action::Press(Button::Key(KeyCode::KeyW))));
        assert_eq!(s.steps[1], (5, Action::Mouse(Vec2::new(3.0, -4.0))));
        assert_eq!(s.steps[2], (10, Action::Release(Button::Key(KeyCode::KeyW))));
        assert_eq!(s.steps[3], (10, Action::Shot("a_1".into())));
        assert_eq!(s.steps[4], (12, Action::Quit));
    }

    #[test]
    fn rejects_bad_lines() {
        assert!(Script::parse("at x quit").is_err());
        assert!(Script::parse("at 1 press KeyWW").is_err());
        assert!(Script::parse("at 1 shot ../escape").is_err());
        assert!(Script::parse("jump").is_err());
    }

    #[test]
    fn knows_keys() {
        assert_eq!(parse_button("KeyA"), Some(Button::Key(KeyCode::KeyA)));
        assert_eq!(parse_button("Digit7"), Some(Button::Key(KeyCode::Digit7)));
        assert_eq!(parse_button("MouseLeft"), Some(Button::Mouse(MouseButton::Left)));
        assert_eq!(parse_button("Keya"), None);
    }
}
