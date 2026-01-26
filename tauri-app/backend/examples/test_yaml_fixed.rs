use std::fs;
use yaml_rust::YamlLoader;

fn main() {
    let config_path = r"D:\Users\lixing.dong\AppData\Roaming\mihomo\config.yaml";
    
    match fs::read_to_string(config_path) {
        Ok(content) => {
            println!("文件读取成功，长度: {}", content.len());
            
            match YamlLoader::load_from_str(&content) {
                Ok(yaml_docs) => {
                    println!("YAML 解析成功! 文档数量: {}", yaml_docs.len());
                    
                    if !yaml_docs.is_empty() {
                        println!("使用第一个文档进行转换...");
                        match mihomoapp::config::yaml_to_json(&yaml_docs[0]) {
                            Ok(_json) => println!("✓ 转换为 JSON 成功!"),
                            Err(e) => println!("✗ 转换为 JSON 失败: {}", e),
                        }
                    }
                }
                Err(e) => {
                    println!("✗ YAML 解析失败: {:?}", e);
                }
            }
        }
        Err(e) => println!("✗ 文件读取失败: {}", e),
    }
}
