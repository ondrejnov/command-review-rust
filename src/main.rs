mod cli;
mod models;
mod openai;
mod review;
mod schemas;
mod tools;

use std::io::{self, Read};

use anyhow::Result;
use clap::Parser;

use crate::cli::Args;
use crate::review::{DECISION_REJECT, review_command};

fn main() -> Result<()> {
    let args = Args::parse();
    let command = if args.command.is_empty() {
        let mut input = String::new();
        io::stdin().read_to_string(&mut input)?;
        input.trim().to_string()
    } else {
        args.command.join(" ").trim().to_string()
    };

    let result = review_command(&command, &args)?;
    if args.pretty {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        println!("{}", serde_json::to_string(&result)?);
    }

    if args.fail_on_reject && result.decision == DECISION_REJECT {
        std::process::exit(2);
    }
    Ok(())
}
