use clap::{crate_version, App, Arg, ArgMatches};

use crate::resim::*;

const ARG_EPOCH: &str = "EPOCH";

/// Constructs a `set-current-epoch` subcommand.
pub fn make_set_current_epoch<'a>() -> App<'a> {
    App::new(CMD_SET_CURRENT_EPOCH)
        .about("Sets the current epoch")
        .version(crate_version!())
        .arg(
            Arg::new(ARG_EPOCH)
                .help("Specify the current epoch.")
                .required(true),
        )
}

/// Handles a `set-current-epoch` request.
pub fn handle_set_current_epoch(matches: &ArgMatches) -> Result<(), Error> {
    let epoch = match_u64(matches, ARG_EPOCH)?;

    let mut configs = get_configs()?;
    configs.current_epoch = epoch;
    set_configs(configs)?;

    println!("Current epoch set!");
    Ok(())
}
