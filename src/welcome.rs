use crate::util::home_dir;
use colored::*;
use std::fs;
use std::io::{self, IsTerminal};

const MARKER_FILENAME: &str = ".ship_welcome_shown";

/// Shows a one-time welcome banner on the very first `ship` invocation
/// on this machine, then never again.
///
/// Silently does nothing when stdout isn't a terminal (so a CI job or
/// script that happens to run `ship` for the first time on a fresh
/// container doesn't get an ANSI banner in its logs) or when the home
/// directory can't be resolved — and in both of those cases the marker
/// is intentionally *not* written, so a human still gets the welcome
/// the first time they actually run it interactively.
pub fn maybe_show() {
    if !io::stdout().is_terminal() {
        return;
    }

    let Some(marker) = home_dir().map(|h| h.join(MARKER_FILENAME)) else {
        return;
    };

    if marker.exists() {
        return;
    }

    print_banner();
    let _ = fs::write(&marker, b"");
}

const WIDTH: usize = 64;

fn print_banner() {
    let top = format!("╭{}╮", "─".repeat(WIDTH - 2));
    let bottom = format!("╰{}╯", "─".repeat(WIDTH - 2));

    println!();
    println!("{}", top.cyan());
    blank();
    text("  ship — one command pre-deploy checklist");
    blank();
    text("  ship               run the fast checklist");
    text("  ship security      deeper scan: secrets, deps, auth");
    text("  ship init          install as a git pre-commit hook");
    text("  ship update        self-update to the latest release");
    blank();
    text("  Run `ship --help` any time for the full list.");
    blank();
    println!("{}", bottom.cyan());
    println!();
}

fn text(content: &str) {
    let inner = WIDTH - 2;
    let pad = inner.saturating_sub(content.chars().count());
    println!("{}{}{}{}", "│".cyan(), content, " ".repeat(pad), "│".cyan());
}

fn blank() {
    text("");
}
