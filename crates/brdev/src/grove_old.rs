use snafu::Snafu;
use std::path::PathBuf;

#[derive(Debug, Snafu)]
pub enum GroveError {
    #[snafu(display("Not in a grove (no .grove/ directory found)"))]
    NotInGrove,
}

pub fn find_grove_root() -> Result<(PathBuf, String), GroveError> {
    let mut current = std::env::current_dir().map_err(|_| GroveError::NotInGrove)?;
    loop {
        if current.join(".grove").is_dir() {
            let name = current
                .file_name()
                .and_then(|n| n.to_str())
                .map(String::from)
                .ok_or(GroveError::NotInGrove)?;
            return Ok((current, name));
        }
        if !current.pop() {
            return Err(GroveError::NotInGrove);
        }
    }
}
