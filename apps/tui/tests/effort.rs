//! Reading pace, worn as reasoning effort: the `^r` ladder, `/effort`, `/rate`
//! retuning a level, and `/speed` moving the base underneath them.

mod common;

use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use readio::app::App;
use readio::book::Book;
use readio::effort::Effort;
use readio::library::Library;
use readio::store::Store;

/// These tests share one `config.yaml`, so they take turns.
fn exclusive() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn fixture() -> (App, Terminal<TestBackend>) {
    common::isolated_home();
    let mut config = readio::config::Config::load().0;
    config.reading.mode = readio::mode::Mode::Manual;
    config.voice.output.allow.clear();
    config.reading.speed = 46.0;
    config.effort = readio::config::EffortConfig::default();
    let _ = config.save();
    let book = Book::load(None).expect("sample book");
    let app = App::new(Library::ephemeral(), Store::ephemeral(), Some(book), None);
    let terminal = Terminal::new(TestBackend::new(96, 30)).expect("terminal");
    (app, terminal)
}

fn screen(terminal: &Terminal<TestBackend>) -> String {
    let buffer = terminal.backend().buffer();
    (0..buffer.area.height)
        .map(|y| {
            let mut row = String::new();
            let mut x = 0u16;
            while x < buffer.area.width {
                let symbol = buffer[(x, y)].symbol();
                row.push_str(symbol);
                x += unicode_width::UnicodeWidthStr::width(symbol).max(1) as u16;
            }
            row.trim_end().to_string()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn draw(app: &mut App, terminal: &mut Terminal<TestBackend>, frames: usize) -> String {
    for _ in 0..frames {
        app.on_tick();
        terminal.draw(|frame| app.draw(frame)).expect("draw");
        std::thread::sleep(std::time::Duration::from_millis(4));
    }
    screen(terminal)
}

fn settle(app: &mut App, terminal: &mut Terminal<TestBackend>) {
    // A deadline rather than a frame count: turn timing is wall-clock, so a
    // frame budget gives a busy CI machine less time, not more.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
    while app.turn.busy() && std::time::Instant::now() < deadline {
        draw(app, terminal, 1);
    }
}

fn type_line(app: &mut App, line: &str) {
    for c in line.chars() {
        app.on_key(KeyEvent::from(KeyCode::Char(c)));
    }
    app.on_key(KeyEvent::from(KeyCode::Enter));
}

fn ctrl(app: &mut App, c: char) {
    app.on_key(KeyEvent {
        code: KeyCode::Char(c),
        modifiers: KeyModifiers::CONTROL,
        kind: KeyEventKind::Press,
        state: ratatui::crossterm::event::KeyEventState::NONE,
    });
}

fn saved() -> readio::config::Config {
    readio::config::Config::load().0
}

fn saved_level() -> Effort {
    saved().effort.level
}

fn saved_multiplier(level: Effort) -> f32 {
    saved().effort.multipliers.get(level)
}

/// `^r` belongs to whichever mode is running: with no voice involved it is the
/// reveal speed, and the voice ladder is `tts::next_speed`, tested there.
#[test]
fn ctrl_r_walks_the_effort_ladder_and_the_pace_follows() {
    let _guard = exclusive();
    let (mut app, mut terminal) = fixture();
    settle(&mut app, &mut terminal);
    assert_eq!(saved_level(), Effort::High, "high is the pace readio ships");
    let normal = app.turn.cps();

    ctrl(&mut app, 'r');
    let view = draw(&mut app, &mut terminal, 3);

    assert_eq!(saved_level(), Effort::Xhigh, "^r moves one level up");
    assert!(
        app.turn.cps() < normal,
        "more effort should read more slowly: {} vs {normal}",
        app.turn.cps()
    );
    assert!(view.contains("Xhigh"), "the level should be named:\n{view}");
    assert!(view.contains("0.85×"), "and what it is worth:\n{view}");

    // Six presses from anywhere land back where they started: one taken already,
    // five to go.
    for _ in 0..5 {
        ctrl(&mut app, 'r');
        draw(&mut app, &mut terminal, 1);
    }
    assert_eq!(saved_level(), Effort::High, "the ladder wraps");
    assert!(
        (app.turn.cps() - normal).abs() < 1.0,
        "and the pace comes back with it: {} vs {normal}",
        app.turn.cps()
    );
}

#[test]
fn effort_can_be_named_and_says_what_it_costs() {
    let _guard = exclusive();
    let (mut app, mut terminal) = fixture();
    settle(&mut app, &mut terminal);

    type_line(&mut app, "/effort minimal");
    let view = draw(&mut app, &mut terminal, 3);

    assert_eq!(saved_level(), Effort::Minimal);
    assert!((app.multiplier() - 2.5).abs() < 0.001);
    assert!(app.turn.cps() > 100.0, "skimming should be fast");
    assert!(view.contains("2.5×"), "{view}");
}

#[test]
fn a_level_nobody_can_spell_is_explained_not_ignored() {
    let _guard = exclusive();
    let (mut app, mut terminal) = fixture();
    settle(&mut app, &mut terminal);

    type_line(&mut app, "/effort sideways");
    let view = draw(&mut app, &mut terminal, 3);

    assert_eq!(saved_level(), Effort::High, "nothing should have changed");
    assert!(view.contains("minimal"), "expected the usage line:\n{view}");
}

#[test]
fn bare_effort_lists_every_level_with_its_multiplier() {
    let _guard = exclusive();
    let (mut app, mut terminal) = fixture();
    settle(&mut app, &mut terminal);

    type_line(&mut app, "/effort");
    let view = draw(&mut app, &mut terminal, 3);

    for level in ["Minimal", "Low", "Medium", "High", "Xhigh", "Max"] {
        assert!(view.contains(level), "{level} missing:\n{view}");
    }
    assert!(view.contains("2.5×") && view.contains("0.7×"), "{view}");
    assert!(
        view.contains("config.yaml"),
        "the reader owns the multipliers, so say where they live:\n{view}"
    );
}

#[test]
fn rate_retunes_the_level_in_force_and_writes_it_back() {
    let _guard = exclusive();
    let (mut app, mut terminal) = fixture();
    settle(&mut app, &mut terminal);

    type_line(&mut app, "/effort max");
    draw(&mut app, &mut terminal, 2);
    type_line(&mut app, "/rate 1.2");
    let view = draw(&mut app, &mut terminal, 3);

    assert_eq!(saved_level(), Effort::Max);
    assert!(
        (saved_multiplier(Effort::Max) - 1.2).abs() < 0.001,
        "the level should be worth 1.2× now, not {}",
        saved_multiplier(Effort::Max)
    );
    assert!(
        (saved_multiplier(Effort::High) - 1.0).abs() < 0.001,
        "and no other level should move"
    );
    assert!(view.contains("1.2×"), "{view}");
    assert!((app.multiplier() - 1.2).abs() < 0.001, "applied at once");
}

#[test]
fn an_impossible_multiplier_is_refused_with_the_usage_line() {
    let _guard = exclusive();
    let (mut app, mut terminal) = fixture();
    settle(&mut app, &mut terminal);

    type_line(&mut app, "/rate 9");
    let view = draw(&mut app, &mut terminal, 3);

    assert!((saved_multiplier(Effort::High) - 1.0).abs() < 0.001);
    assert!(view.contains("/rate"), "{view}");
}

#[test]
fn speed_moves_the_base_the_multiplier_scales() {
    let _guard = exclusive();
    let (mut app, mut terminal) = fixture();
    settle(&mut app, &mut terminal);

    type_line(&mut app, "/effort low");
    draw(&mut app, &mut terminal, 2);
    type_line(&mut app, "/speed 50");
    let view = draw(&mut app, &mut terminal, 3);

    assert_eq!(saved().reading.speed, 50.0);
    // low is worth 2×, so the pacer runs at a hundred characters a second.
    assert!(
        (app.turn.cps() - 100.0).abs() < 1.0,
        "expected 50 × 2, got {}",
        app.turn.cps()
    );
    assert!(
        view.contains("Low"),
        "the level in force should be named:\n{view}"
    );
}

/// `/effort 1.25`: a number where a level goes, for the reader who finds `high`
/// (1×) too slow and `medium` (1.5×) too brisk. It is not a rung, so it gets a
/// level of its own and `(custom)` beside the model name — but the pace is real,
/// and the pacer runs at it straight away.
#[test]
fn a_typed_multiplier_is_a_pace_between_the_rungs() {
    let _guard = exclusive();
    let (mut app, mut terminal) = fixture();
    settle(&mut app, &mut terminal);

    type_line(&mut app, "/effort 1.25");
    let view = draw(&mut app, &mut terminal, 3);

    assert_eq!(
        saved_level(),
        Effort::Custom,
        "a number is a level of its own"
    );
    assert!(
        (saved_multiplier(Effort::Custom) - 1.25).abs() < 0.001,
        "the number should be written down: {}",
        saved_multiplier(Effort::Custom)
    );
    assert!((app.multiplier() - 1.25).abs() < 0.001, "applied at once");
    // 46 base × 1.25 = 57.5: faster than high's 46, slower than medium's 69.
    assert!(
        (app.turn.cps() - 57.5).abs() < 1.0,
        "expected 46 × 1.25, got {}",
        app.turn.cps()
    );
    assert!(view.contains("1.25×"), "{view}");
    assert!(
        view.contains("(custom)"),
        "the header should say the pace is the reader's own:\n{view}"
    );
}

#[test]
fn a_custom_pace_survives_a_named_level_and_can_be_returned_to() {
    let _guard = exclusive();
    let (mut app, mut terminal) = fixture();
    settle(&mut app, &mut terminal);

    type_line(&mut app, "/effort 1.25");
    draw(&mut app, &mut terminal, 2);
    type_line(&mut app, "/effort high");
    let view = draw(&mut app, &mut terminal, 2);

    assert_eq!(saved_level(), Effort::High, "a named rung is a rung again");
    assert!(
        !view.contains("(custom)"),
        "the header should name a rung again:\n{view}"
    );
    assert!(
        (saved_multiplier(Effort::Custom) - 1.25).abs() < 0.001,
        "choosing a rung must not eat the reader's own number: {}",
        saved_multiplier(Effort::Custom)
    );

    // And it is still there: `/effort custom` is the way back to it.
    type_line(&mut app, "/effort custom");
    let view = draw(&mut app, &mut terminal, 3);
    assert_eq!(saved_level(), Effort::Custom);
    assert!((app.multiplier() - 1.25).abs() < 0.001);
    assert!(view.contains("(custom)"), "{view}");
}

/// `^r` means one step slower from anywhere, including from a number: it lands
/// on the named rung at or below the pace, so the reader is never stranded on a
/// level the menu cannot mark.
#[test]
fn ctrl_r_from_a_custom_pace_lands_on_the_named_rung_below() {
    let _guard = exclusive();
    let (mut app, mut terminal) = fixture();
    settle(&mut app, &mut terminal);

    type_line(&mut app, "/effort 1.25");
    draw(&mut app, &mut terminal, 2);
    let custom = app.turn.cps();

    ctrl(&mut app, 'r');
    let view = draw(&mut app, &mut terminal, 3);

    assert_eq!(saved_level(), Effort::High, "1.0 is the rung below 1.25");
    assert!(
        app.turn.cps() < custom,
        "more effort reads more slowly: {} vs {custom}",
        app.turn.cps()
    );
    assert!(view.contains("(high)"), "{view}");
}

#[test]
fn a_number_the_player_cannot_read_at_is_refused_not_clamped() {
    let _guard = exclusive();
    let (mut app, mut terminal) = fixture();
    settle(&mut app, &mut terminal);

    type_line(&mut app, "/effort 9");
    let view = draw(&mut app, &mut terminal, 3);
    assert_eq!(saved_level(), Effort::High, "nothing should have changed");
    assert!(
        (saved_multiplier(Effort::Custom) - 1.25).abs() < 0.001,
        "and 9 is not 3"
    );
    assert!(view.contains("/effort"), "expected the usage line:\n{view}");

    type_line(&mut app, "/effort 0.25");
    draw(&mut app, &mut terminal, 2);
    assert_eq!(saved_level(), Effort::High);
}
