//! brdev CLI binary.

use brdev::cli;

fn main() -> miette::Result<()> {
    miette::set_panic_hook();
    Ok(cli::run()?)
}
