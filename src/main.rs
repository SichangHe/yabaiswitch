use std::{collections::VecDeque, env::args, process::Command};

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

fn main() -> Result<()> {
    let args: Vec<_> = args().collect();

    let raw_run = |command, args| -> Result<_> {
        let output = Command::new(command).args(args).output()?;
        Ok(String::from_utf8(output.stdout)?)
    };
    let run = |command: String| -> Result<_> {
        let result = raw_run("bash".to_owned(), vec!["-c".into(), command])?;
        Ok(result)
    };
    let display = |content, title, subtitle| {
        run(format!(
            r#"osascript -e 'display notification "{content}" with title "{title}" subtitle "{subtitle}"'"#
        ))
    };
    const YABAI: &str = "/usr/local/bin/yabai";
    const JQ: &str = "~/.nix-profile/bin/jq";
    let split_args = |string: &str| string.split_whitespace().map(|s| s.into()).collect();
    let cycle = || -> Result<_> {
        let raw_ids = raw_run(YABAI.into(), split_args("-m query --windows --space"))?;
        let mut id_list: VecDeque<WindowInfo> = serde_json::from_str(&raw_ids)?;
        id_list.make_contiguous().sort();

        let index = id_list
            .iter()
            .enumerate()
            .find_map(|(index, window)| window.has_focus.then_some(index))
            .unwrap_or(0);
        if args[1] == "next" {
            id_list.rotate_left(1);
        } else {
            id_list.rotate_right(1);
        }
        let target = id_list[index].id.to_string();
        let mut args = split_args("-m window --focus");
        args.push(target);
        raw_run(YABAI.into(), args)
    };
    let info = || -> Result<_> {
        let apps = run(format!(
            "{YABAI} -m query --windows --space | {JQ} '.[].app'"
        ))?;
        let app_str: String = apps
            .lines()
            .map(|l| l.split('\"').nth(1).unwrap())
            .collect::<Vec<_>>()
            .join(";");
        let space_id = run(format!("{YABAI} -m query --spaces --space | {JQ} '.index'"))?
            .trim()
            .to_owned();
        let window_num = run(format!(
            "{YABAI} -m query --windows --space | {JQ} '.[].title' | wc -l"
        ))?
        .trim()
        .to_owned();
        display(
            app_str,
            format!("Space {space_id}"),
            format!("{window_num} windows"),
        )
    };

    let result = match args[1].as_str() {
        "next" | "last" => cycle(),
        "info" => info(),
        wrong => {
            bail!("Unkown argument {wrong}. Correct arguments are `next`, `last`, and `info`.")
        }
    }?;
    if !result.is_empty() {
        display(result, "yabai".into(), "scirpt failed".into())?;
    }

    Ok(())
}

#[derive(Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
struct WindowInfo {
    id: usize,
    has_focus: bool,
}
