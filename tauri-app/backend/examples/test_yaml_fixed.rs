use anyhow::{Context, Result};
use std::fs;
use yaml_rust::YamlLoader;

fn main() -> Result<()> {
    let path = std::env::args_os()
        .nth(1)
        .context("Usage: cargo run --example test_yaml_fixed -- <config.yaml>")?;
    let content = fs::read_to_string(&path).context("Failed to read YAML file")?;
    let documents = YamlLoader::load_from_str(&content).context("Failed to parse YAML")?;
    let document = documents.first().context("YAML file is empty")?;
    let json = mihomoapp::yaml::yaml_to_json(document)?;
    println!("{}", serde_json::to_string_pretty(&json)?);
    Ok(())
}
