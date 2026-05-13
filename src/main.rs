use std::{collections::VecDeque, env, env::args, process::Command};

use anyhow::{bail, Result};
use serde::Deserialize;

const YABAI: &str = "/usr/local/bin/yabai";

fn main() {
    if let Err(e) = run() {
        notify(&e.to_string(), "yabaiswitch", "error");
    }
}

fn notify(content: &str, title: &str, subtitle: &str) {
    let script = if subtitle.is_empty() {
        format!(r#"display notification "{content}" with title "{title}""#)
    } else {
        format!(r#"display notification "{content}" with title "{title}" subtitle "{subtitle}""#)
    };
    let _ = Command::new("osascript").args(["-e", &script]).output();
}

fn yabai_run(yabai_args: &[&str]) -> Result<String> {
    let out = Command::new(YABAI).args(yabai_args).output()?;
    if !out.status.success() {
        bail!("{}", String::from_utf8_lossy(&out.stderr).trim());
    }
    Ok(String::from_utf8(out.stdout)?)
}

fn run() -> Result<()> {
    let args: Vec<_> = args().collect();
    let exclude: Vec<String> = env::var("EXCLUDE_APPS")
        .unwrap_or_default()
        .split(',')
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .collect();
    let debug = env::var("YABAISWITCH_DEBUG").is_ok();
    if debug {
        notify(&format!("{exclude:?}").replace('"', ""), "exclude", "");
    }
    match args.get(1).map(String::as_str) {
        Some(dir @ ("next" | "last")) => cycle(dir, &exclude, debug),
        Some("space") => {
            let sel = match args.get(2).map(String::as_str) {
                Some(s) if !s.starts_with('-') => s,
                _ => bail!("space requires a selector: prev, next, first, last, recent, <index>"),
            };
            space_focus(sel, &exclude, debug)
        }
        Some("info") => info(),
        Some(cmd) => bail!("Unknown command `{cmd}`. Valid: next, last, space, info."),
        None => bail!("No command. Valid: next, last, space, info."),
    }
}

fn cycle(dir: &str, exclude: &[String], debug: bool) -> Result<()> {
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
    let target = &windows[idx];
    if debug {
        notify(
            &format!("id={} app={}", target.id, target.app),
            "cycle target",
            "",
        );
    }
    yabai_run(&["-m", "window", "--focus", &target.id.to_string()])?;
    Ok(())
}

/// Focus a space and then refocus the best non-excluded, non-minimized, non-hidden window.
/// Pre-queries target space to select preferred window before switching, preventing both
/// Zoom decorative windows grabbing focus and the macOS bounce on stale/missing last-focused window.
fn space_focus(sel: &str, exclude: &[String], debug: bool) -> Result<()> {
    let raw = yabai_run(&["-m", "query", "--windows", "--space", sel])?;
    let windows: Vec<WindowInfo> = serde_json::from_str(&raw)?;
    if debug {
        notify(&format!("{windows:?}").replace('"', ""), "windows", "");
    }
    let candidates: Vec<_> = windows
        .iter()
        .filter(|w| !exclude.contains(&w.app) && !w.is_minimized && !w.is_hidden)
        .collect();
    if debug {
        notify(
            &format!("{candidates:?}").replace('"', ""),
            "candidates",
            "",
        );
    }
    let preferred = candidates
        .iter()
        .find(|w| w.has_focus)
        .or_else(|| candidates.first())
        .map(|w| w.id.to_string());
    if debug {
        notify(&format!("{preferred:?}").replace('"', ""), "preferred", "");
    }
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
    notify(
        &apps,
        &format!("Space {}", space.index),
        &format!("{} windows", wins.len()),
    );
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
