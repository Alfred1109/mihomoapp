use std::fs;

fn main() {
    let config_path = r"D:\Users\lixing.dong\AppData\Roaming\mihomo\config.yaml";
    
    match fs::read_to_string(config_path) {
        Ok(content) => {
            println!("文件读取成功，长度: {}", content.len());
            
            // Handle multi-document YAML by only parsing the first document
            let content_to_parse = if let Some(pos) = content.find("\n---\n") {
                println!("检测到多文档 YAML，只解析第一个文档");
                &content[..pos]
            } else if let Some(pos) = content.find("\r\n---\r\n") {
                println!("检测到多文档 YAML (Windows 换行符)，只解析第一个文档");
                &content[..pos]
            } else {
                &content
            };
            
            match serde_yaml::from_str::<serde_yaml::Value>(content_to_parse) {
                Ok(yaml) => {
                    println!("YAML 解析成功!");
                    match serde_json::to_value(yaml) {
                        Ok(_json) => println!("转换为 JSON 成功!"),
                        Err(e) => println!("转换为 JSON 失败: {}", e),
                    }
                }
                Err(e) => {
                    println!("YAML 解析失败: {}", e);
                    if let Some(location) = e.location() {
                        println!("错误位置 - 行: {}, 列: {}", location.line(), location.column());
                    }
                }
            }
        }
        Err(e) => println!("文件读取失败: {}", e),
    }
}
