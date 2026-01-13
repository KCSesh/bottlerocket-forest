//! Forester CLI binary.

use forester::cli;

fn main() -> miette::Result<()> {
    tracing_subscriber::fmt::init();
    miette::set_panic_hook();
    cli::run()
}
