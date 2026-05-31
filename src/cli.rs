use std::path::PathBuf;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(
    name = "command-review",
    about = "Review a shell command and return a JSON risk decision for AI agents."
)]
pub(crate) struct Args {
    #[arg(long)]
    pub(crate) pretty: bool,

    #[arg(long)]
    pub(crate) fail_on_reject: bool,

    #[arg(long)]
    pub(crate) stream_tools: bool,

    #[arg(long)]
    pub(crate) fast: bool,

    #[arg(long)]
    pub(crate) model: Option<String>,

    #[arg(long)]
    pub(crate) base_url: Option<String>,

    #[arg(long)]
    pub(crate) prompt_file: Option<PathBuf>,

    #[arg(long)]
    pub(crate) cwd: Option<PathBuf>,

    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub(crate) command: Vec<String>,
}
