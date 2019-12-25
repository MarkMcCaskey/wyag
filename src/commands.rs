use std::path::PathBuf;
use structopt::StructOpt;

use crate::repository;

#[derive(Debug, StructOpt)]
pub struct Init {
    /// Where to create the repository
    #[structopt(parse(from_os_str), default_value = ".")]
    path: PathBuf,
}

pub fn cmd_init(init: &Init) -> Result<(), String> {
    repository::repo_create(&init.path)?;

    Ok(())
}
