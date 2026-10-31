//! L'appli au premier plan, par `lsappinfo` (livré avec macOS, sans autorisation à demander) : `lsappinfo front` donne son identifiant,
//! `lsappinfo info` son nom et son identifiant de bundle. La lecture de la sortie est dans `crate::apple`.

use crate::apple::{parse_app_info, parse_front};
use crate::apps::{AppInfo, ForegroundApp};
use std::process::Command;

pub struct MacForeground;

fn run(args: &[&str]) -> Option<String> {
    let output = Command::new("lsappinfo").args(args).output().ok()?;
    output.status.success().then(|| String::from_utf8_lossy(&output.stdout).into_owned())
}

impl ForegroundApp for MacForeground {
    fn current(&self) -> AppInfo {
        let Some(front) = run(&["front"]).as_deref().and_then(parse_front) else { return AppInfo::unknown() };
        run(&["info", "-only", "name", "-only", "bundleid", &front]).map_or_else(AppInfo::unknown, |text| parse_app_info(&text))
    }
}
