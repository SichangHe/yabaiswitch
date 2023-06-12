use std::{collections::VecDeque, env::args, process::Command};

use anyhow::{bail, Result};

fn main() -> Result<()> {
    let args: Vec<_> = args().collect();

    let run = |command: String| -> Result<_> {
        let output = Command::new("bash").args(["-c", &command]).output()?;
        Ok(String::from_utf8(output.stdout)?)
    };
    let display = |content, title, subtitle| {
        run(format!(
            r#"osascript -e 'display notification "{content}" with title "{title}" subtitle "{subtitle}"'"#
        ))
    };
    const YABAI: &str = " /opt/homebrew/bin/yabai ";
    const JQ: &str = " ~/.nix-profile/bin/jq ";
    let cycle = || -> Result<_> {
        let ids = run(format!(
            "{YABAI} -m query --windows --space | {JQ} '.[].id'"
        ))?;
        let id: usize = run(format!("{YABAI} -m query --windows --window | {JQ} '.id'"))?
            .trim()
            .parse()?;

        let mut id_list = ids
            .split_whitespace()
            .map(|s| s.parse())
            .collect::<Result<VecDeque<_>, _>>()?;
        id_list.make_contiguous().sort();

        let index = id_list.binary_search(&id).unwrap();
        if args[1] == "next" {
            id_list.rotate_left(1);
        } else {
            id_list.rotate_right(1);
        }
        let target = id_list[index];
        run(format!("{YABAI} -m window --focus {target}"))
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
