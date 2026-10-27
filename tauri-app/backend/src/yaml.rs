use anyhow::Result;
use yaml_rust::Yaml;

pub fn yaml_to_json(yaml: &Yaml) -> Result<serde_json::Value> {
    match yaml {
        Yaml::Real(f) => Ok(serde_json::json!(f.parse::<f64>().unwrap_or(0.0))),
        Yaml::Integer(i) => Ok(serde_json::json!(*i)),
        Yaml::String(s) => Ok(serde_json::json!(s)),
        Yaml::Boolean(b) => Ok(serde_json::json!(*b)),
        Yaml::Array(arr) => {
            let mut json_arr = Vec::new();
            for item in arr {
                json_arr.push(yaml_to_json(item)?);
            }
            Ok(serde_json::json!(json_arr))
        }
        Yaml::Hash(hash) => {
            let mut json_obj = serde_json::Map::new();
            for (key, value) in hash {
                if let Yaml::String(key_str) = key {
                    json_obj.insert(key_str.clone(), yaml_to_json(value)?);
                }
            }
            Ok(serde_json::Value::Object(json_obj))
        }
        Yaml::Alias(_) => Err(anyhow::anyhow!("YAML aliases not supported")),
        Yaml::Null => Ok(serde_json::Value::Null),
        Yaml::BadValue => Err(anyhow::anyhow!("Bad YAML value")),
    }
}
