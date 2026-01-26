use std::fs;

fn main() {
    let config_path = r"D:\Users\lixing.dong\AppData\Roaming\mihomo\config.yaml";
    
    match fs::read_to_string(config_path) {
        Ok(content) => {
            println!("File read successfully, length: {}", content.len());
            
            match serde_yaml::from_str::<serde_yaml::Value>(&content) {
                Ok(yaml) => {
                    println!("YAML parsed successfully!");
                    match serde_json::to_value(yaml) {
                        Ok(json) => println!("Converted to JSON successfully!"),
                        Err(e) => println!("Failed to convert to JSON: {}", e),
                    }
                }
                Err(e) => {
                    println!("Failed to parse YAML: {}", e);
                    if let Some(location) = e.location() {
                        println!("Error at line: {}, column: {}", location.line(), location.column());
                    }
                }
            }
        }
        Err(e) => println!("Failed to read file: {}", e),
    }
}
