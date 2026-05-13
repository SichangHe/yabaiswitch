use std::{collections::VecDeque, env::args, process::Command};

use anyhow::{bail, Result};
use serde::Deserialize;

const YABAI: &str = "/usr/local/bin/yabai";

fn main() {
    if let Err(e) = run() {
        let _ = Command::new("osascript")
            .args([
                "-e",
                &format!(r#"display notification "{e}" with title "yabaiswitch" subtitle "error""#),
            ])
            .output();
    }
}

fn yabai_run(yabai_args: &[&str]) -> Result<String> {
    let out = Command::new(YABAI).args(yabai_args).output()?;
    if !out.status.success() {
        bail!("{}", String::from_utf8_lossy(&out.stderr).trim());
    }
    Ok(String::from_utf8(out.stdout)?)
}

fn parse_exclude_apps(args: &[String]) -> Vec<String> {
    args.windows(2)
        .find(|w| w[0] == "--exclude-apps")
        .map(|w| w[1].split(',').map(str::to_owned).collect())
        .unwrap_or_default()
}

fn run() -> Result<()> {
    let args: Vec<_> = args().collect();
    let exclude = parse_exclude_apps(&args);
    match args.get(1).map(String::as_str) {
        Some(dir @ ("next" | "last")) => cycle(dir, &exclude),
        Some("space") => {
            let sel = match args.get(2).map(String::as_str) {
                Some(s) if !s.starts_with('-') => s,
                _ => bail!("space requires a selector: prev, next, first, last, recent, <index>"),
            };
            space_focus(sel, &exclude)
        }
        Some("info") => info(),
        Some(cmd) => bail!("Unknown command `{cmd}`. Valid: next, last, space, info."),
        None => bail!("No command. Valid: next, last, space, info."),
    }
}

fn cycle(dir: &str, exclude: &[String]) -> Result<()> {
    let raw = yabai_run(&["-m", "query", "--windows", "--space"])?;
    let mut windows: VecDeque<WindowInfo> = serde_json::from_str(&raw)?;
    windows.make_contiguous().sort();
    windows.retain(|w| !exclude.contains(&w.app));
    if windows.is_empty() {
        return Ok(());
    }
    let idx = windows
        .iter()
        .enumerate()
        .find_map(|(i, w)| w.has_focus.then_some(i))
        .unwrap_or(0);
    if dir == "next" {
        windows.rotate_left(1);
    } else {
        windows.rotate_right(1);
    }
    yabai_run(&["-m", "window", "--focus", &windows[idx].id.to_string()])?;
    Ok(())
}

/// Focus a space and then refocus the best non-excluded, non-minimized, non-hidden window.
/// Pre-queries target space to select preferred window before switching, preventing both
/// Zoom decorative windows grabbing focus and the macOS bounce on stale/missing last-focused window.
fn space_focus(sel: &str, exclude: &[String]) -> Result<()> {
    let raw = yabai_run(&["-m", "query", "--windows", "--space", sel])?;
    let windows: Vec<WindowInfo> = serde_json::from_str(&raw)?;
    let candidates: Vec<_> = windows
        .iter()
        .filter(|w| !exclude.contains(&w.app) && !w.is_minimized && !w.is_hidden)
        .collect();
    let preferred = candidates
        .iter()
        .find(|w| w.has_focus)
        .or_else(|| candidates.first())
        .map(|w| w.id.to_string());
    yabai_run(&["-m", "space", "--focus", sel])?;
    if let Some(id) = preferred {
        yabai_run(&["-m", "window", "--focus", &id])?;
    }
    Ok(())
}

fn info() -> Result<()> {
    let raw_wins = yabai_run(&["-m", "query", "--windows", "--space"])?;
    let raw_space = yabai_run(&["-m", "query", "--spaces", "--space"])?;
    let wins: Vec<WindowApp> = serde_json::from_str(&raw_wins)?;
    let space: SpaceIndex = serde_json::from_str(&raw_space)?;
    let apps = wins
        .iter()
        .map(|w| w.app.as_str())
        .collect::<Vec<_>>()
        .join(";");
    Command::new("osascript")
        .args([
            "-e",
            &format!(
                r#"display notification "{apps}" with title "Space {}" subtitle "{} windows""#,
                space.index,
                wins.len()
            ),
        ])
        .output()?;
    Ok(())
}

#[derive(Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd)]
#[serde(rename_all = "kebab-case")]
struct WindowInfo {
    id: usize,
    app: String,
    has_focus: bool,
    is_minimized: bool,
    is_hidden: bool,
}

#[derive(Deserialize)]
struct WindowApp {
    app: String,
}

#[derive(Deserialize)]
struct SpaceIndex {
    index: usize,
}
