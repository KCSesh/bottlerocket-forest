use std::fs;

fn main() {
    let content = fs::read_to_string("/etc/token-printer.toml").expect("Failed to read config");
    let config: toml::Table = content.parse().expect("Failed to parse TOML");
    if let Some(token) = config.get("token").and_then(|v| v.as_str()) {
        println!("{}", token);
    }
}
