use colored::*;
use std::io::{self, IsTerminal, Write};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

const FRAMES: [char; 10] = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
const TICK: Duration = Duration::from_millis(80);

/// Runs `f`, animating a spinner next to `label` while it's in flight.
///
/// Falls back to just calling `f()` directly — no ANSI at all — when
/// stdout isn't a terminal (piped output, `--json`, a pre-commit hook's
/// captured output, CI logs), so scripted/non-interactive runs are
/// completely unaffected.
pub fn run<T, F>(label: &str, f: F) -> T
where
    T: Send,
    F: FnOnce() -> T + Send,
{
    if !io::stdout().is_terminal() {
        return f();
    }

    let (tx, rx) = mpsc::channel();
    let start = Instant::now();

    let result = thread::scope(|scope| {
        scope.spawn(|| {
            let _ = tx.send(f());
        });

        let mut frame = 0usize;
        loop {
            match rx.recv_timeout(TICK) {
                Ok(value) => break value,
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    render(label, FRAMES[frame % FRAMES.len()], start.elapsed());
                    frame += 1;
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    // Only reachable if the check itself panicked
                    // before sending a result — a bug in the check,
                    // not in the spinner.
                    unreachable!("spinner worker disconnected without sending a result")
                }
            }
        }
    });

    clear_line();
    result
}

fn render(label: &str, frame: char, elapsed: Duration) {
    clear_line();
    print!(
        "  {} {}  {}",
        frame.to_string().cyan(),
        label.dimmed(),
        format!("{:.1}s", elapsed.as_secs_f32()).dimmed()
    );
    let _ = io::stdout().flush();
}

fn clear_line() {
    print!("\r\x1b[2K");
    let _ = io::stdout().flush();
}
