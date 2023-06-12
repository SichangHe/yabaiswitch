use std::process::Command;

use anyhow::Result;

fn main() -> Result<()> {
    let display = |content, title, subtitle| {
        let mut arg2 = format!(r#"display notification "{content}""#);
        if let Some(title) = title {
            arg2 += &format!(r#" with title "{title}" "#);
            if let Some(subtitle) = subtitle {
                arg2 += &format!(r#" subtitle "{subtitle}" "#);
            }
        }
        Command::new("osascript").args(["-e", &arg2]).output()
    };
    display("test", Some("title"), Some("subtitle"))?;

    Ok(())
}
