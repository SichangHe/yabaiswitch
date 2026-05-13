use std::{collections::VecDeque, env, env::args, process::Command, thread::sleep, time::Duration};

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

/// Focus window by id, re-issuing every 10 ms if macOS steals focus, up to 10 retries.
/// Focusing an already-focused window is not an error.
fn focus_window(id: usize) -> Result<()> {
    let id_s = id.to_string();
    let _ = yabai_run(&["-m", "window", "--focus", &id_s]);
    for _ in 0..10 {
        sleep(Duration::from_millis(10));
        let focused = yabai_run(&["-m", "query", "--windows", "--window"])
            .ok()
            .and_then(|s| serde_json::from_str::<WindowId>(&s).ok())
            .map(|w| w.id == id)
            .unwrap_or(false);
        if focused {
            break;
        }
        let _ = yabai_run(&["-m", "window", "--focus", &id_s]);
    }
    Ok(())
}

/// Focus a space by selector, re-issuing every 10 ms if macOS overrides, up to 10 retries.
/// Resolves selector to absolute index first so relative selectors (prev/next) stay correct
/// after the space switch changes what "prev"/"next" means.
fn focus_space(sel: &str) -> Result<()> {
    let raw = yabai_run(&["-m", "query", "--spaces", "--space", sel])?;
    let idx = serde_json::from_str::<SpaceIndex>(&raw)?.index;
    let idx_s = idx.to_string();
    let _ = yabai_run(&["-m", "space", "--focus", &idx_s]);
    for _ in 0..10 {
        sleep(Duration::from_millis(10));
        let on_target = yabai_run(&["-m", "query", "--spaces", "--space"])
            .ok()
            .and_then(|s| serde_json::from_str::<SpaceIndex>(&s).ok())
            .map(|s| s.index == idx)
            .unwrap_or(false);
        if on_target {
            break;
        }
        let _ = yabai_run(&["-m", "space", "--focus", &idx_s]);
    }
    Ok(())
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
    #[cfg(debug_assertions)]
    notify(&format!("{exclude:?}").replace('"', ""), "exclude", "");
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
    let target = &windows[idx];
    #[cfg(debug_assertions)]
    notify(
        &format!("id={} app={}", target.id, target.app),
        "cycle target",
        "",
    );
    focus_window(target.id)?;
    Ok(())
}

/// Focus a space and then refocus the best non-excluded, non-minimized, non-hidden window.
/// Pre-queries target space to select preferred window before switching, preventing both
/// Zoom decorative windows grabbing focus and the macOS bounce on stale/missing last-focused window.
fn space_focus(sel: &str, exclude: &[String]) -> Result<()> {
    let raw = yabai_run(&["-m", "query", "--windows", "--space", sel])?;
    let windows: Vec<WindowInfo> = serde_json::from_str(&raw)?;
    #[cfg(debug_assertions)]
    notify(&format!("{windows:?}").replace('"', ""), "windows", "");
    let candidates: Vec<_> = windows
        .iter()
        .filter(|w| !exclude.contains(&w.app) && !w.is_minimized && !w.is_hidden)
        .collect();
    #[cfg(debug_assertions)]
    notify(
        &format!("{candidates:?}").replace('"', ""),
        "candidates",
        "",
    );
    let preferred = candidates
        .iter()
        .find(|w| w.has_focus)
        .or_else(|| candidates.first())
        .map(|w| w.id);
    #[cfg(debug_assertions)]
    notify(&format!("{preferred:?}").replace('"', ""), "preferred", "");
    focus_space(sel)?;
    if let Some(id) = preferred {
        focus_window(id)?;
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
struct WindowId {
    id: usize,
}

#[derive(Deserialize)]
struct WindowApp {
    app: String,
}

#[derive(Deserialize)]
struct SpaceIndex {
    index: usize,
}
