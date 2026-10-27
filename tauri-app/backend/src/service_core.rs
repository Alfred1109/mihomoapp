//! Read-only validation before migrating configuration for an installed service.
use anyhow::{Context, Result};
use std::path::Path;

pub fn verify_installed_service_core(core: &Path, config: &Path) -> Result<()> {
    verify_platform_service(core, config)
        .context("已安装服务的核心或配置路径无法核实，未迁移配置；请重新安装本项目服务后重试")
}

#[cfg(target_os = "linux")]
fn verify_platform_service(core: &Path, config: &Path) -> Result<()> {
    // ExecStart is a structured D-Bus property, not localized systemctl prose.
    let output = std::process::Command::new("busctl")
        .args([
            "--system",
            "--timeout=3",
            "--auto-start=no",
            "--json=short",
            "get-property",
            "org.freedesktop.systemd1",
            "/org/freedesktop/systemd1/unit/mihomo_2eservice",
            "org.freedesktop.systemd1.Service",
            "ExecStart",
        ])
        .output()
        .context("无法只读查询 systemd ExecStart")?;
    if !output.status.success() {
        return Err(anyhow::anyhow!("systemd ExecStart 查询失败"));
    }
    validate_systemd_execstart(&output.stdout, core, config)
}

#[cfg(target_os = "windows")]
fn verify_platform_service(core: &Path, config: &Path) -> Result<()> {
    let executable = std::env::current_exe()?;
    let directory = executable.parent().context("无法获取应用目录")?;
    let wrapper = directory.join("MihomoService.exe");
    // Verify the registered wrapper too; an XML next to this app is not evidence
    // that an unrelated service wrapper actually uses that XML.
    let output = std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", "$ErrorActionPreference='Stop'; [Console]::OutputEncoding=New-Object System.Text.UTF8Encoding($false); $s=Get-CimInstance Win32_Service -OperationTimeoutSec 3 -Filter \"Name='MihomoService'\"; if ($null -eq $s) { exit 1 }; $s.PathName | ConvertTo-Json -Compress"])
        .output().context("无法只读查询 Windows 服务路径")?;
    if !output.status.success() {
        return Err(anyhow::anyhow!("Windows 服务路径查询失败"));
    }
    let registered: String =
        serde_json::from_slice(&output.stdout).context("Windows 服务路径返回格式无效")?;
    let expected = wrapper.to_str().context("Windows 服务路径不是有效 UTF-8")?;
    if registered != expected && registered != format!("\"{}\"", expected) {
        return Err(anyhow::anyhow!(
            "Windows 服务使用了不同的 WinSW 程序或附加参数"
        ));
    }
    let xml = std::fs::read_to_string(directory.join("MihomoService.xml"))
        .context("无法读取已有 WinSW 配置")?;
    validate_winsw_xml(&xml, core, config)
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
fn verify_platform_service(_core: &Path, _config: &Path) -> Result<()> {
    Err(anyhow::anyhow!("当前平台尚无可核实的系统服务适配器"))
}

fn expected_arguments(core: &Path, config: &Path) -> Result<Vec<Vec<String>>> {
    let core = core.to_str().context("核心路径不是有效 UTF-8")?;
    let file = config.to_str().context("配置路径不是有效 UTF-8")?;
    let directory = config
        .parent()
        .and_then(Path::to_str)
        .context("配置目录不是有效 UTF-8")?;
    Ok(vec![
        vec![core.into(), "-d".into(), directory.into()],
        vec![core.into(), "-f".into(), file.into()],
    ])
}

#[cfg(any(target_os = "linux", test))]
fn validate_systemd_execstart(data: &[u8], core: &Path, config: &Path) -> Result<()> {
    let value: serde_json::Value =
        serde_json::from_slice(data).context("无法解析结构化 ExecStart")?;
    if value["type"].as_str() != Some("a(sasbttttuii)") {
        return Err(anyhow::anyhow!("未知的 systemd ExecStart 属性类型"));
    }
    let entries = value["data"].as_array().context("ExecStart 不是数组")?;
    if entries.len() != 1 {
        return Err(anyhow::anyhow!("ExecStart 必须且只能有一个启动命令"));
    }
    let entry = entries[0].as_array().context("ExecStart 项格式错误")?;
    if entry.len() != 10 || entry[0].as_str() != core.to_str() || entry[2].as_bool() != Some(false)
    {
        return Err(anyhow::anyhow!(
            "ExecStart 核心路径或执行语义与本项目不一致"
        ));
    }
    let arguments: Vec<String> = entry[1]
        .as_array()
        .context("ExecStart argv 格式错误")?
        .iter()
        .map(|argument| {
            argument
                .as_str()
                .map(str::to_owned)
                .context("ExecStart argv 必须为字符串")
        })
        .collect::<Result<_>>()?;
    if !expected_arguments(core, config)?.contains(&arguments) {
        return Err(anyhow::anyhow!("ExecStart 参数未严格指向本项目配置"));
    }
    Ok(())
}

#[cfg(any(target_os = "windows", test))]
fn validate_winsw_xml(xml: &str, core: &Path, config: &Path) -> Result<()> {
    use quick_xml::{events::Event, Reader};
    let mut reader = Reader::from_str(xml);
    let mut stack: Vec<String> = Vec::new();
    let mut fields = std::collections::HashMap::<String, String>::new();
    let mut roots = 0;
    loop {
        match reader.read_event().context("WinSW XML 格式无效")? {
            Event::Start(event) => {
                let name = std::str::from_utf8(event.name().as_ref())?.to_owned();
                if stack.is_empty() {
                    roots += 1;
                    if name != "service" || roots != 1 || event.attributes().next().is_some() {
                        return Err(anyhow::anyhow!("WinSW XML 必须有唯一 service 根节点"));
                    }
                }
                if stack.len() == 1 && ["id", "executable", "arguments"].contains(&name.as_str()) {
                    if event.attributes().next().is_some()
                        || fields.insert(name.clone(), String::new()).is_some()
                    {
                        return Err(anyhow::anyhow!("WinSW 关键字段重复或包含属性"));
                    }
                } else if stack.len() > 1
                    && ["id", "executable", "arguments"].contains(&stack[1].as_str())
                {
                    return Err(anyhow::anyhow!("WinSW 关键字段必须为纯文本"));
                }
                stack.push(name);
            }
            Event::Empty(event) => {
                let name = std::str::from_utf8(event.name().as_ref())?.to_owned();
                if stack.is_empty() || ["id", "executable", "arguments"].contains(&name.as_str()) {
                    return Err(anyhow::anyhow!("WinSW 关键字段不能是空元素"));
                }
            }
            Event::GeneralRef(reference) => {
                if stack.len() == 2 {
                    if let Some(field) = fields.get_mut(&stack[1]) {
                        field.push_str(&quick_xml::escape::unescape(&format!(
                            "&{};",
                            reference.decode()?
                        ))?);
                    }
                }
            }
            Event::Text(text) if stack.is_empty() => {
                if !text.xml_content()?.trim().is_empty() {
                    return Err(anyhow::anyhow!("WinSW XML 根节点外存在内容"));
                }
            }
            Event::Text(text) if stack.len() == 2 => {
                if let Some(field) = fields.get_mut(&stack[1]) {
                    field.push_str(&quick_xml::escape::unescape(&text.xml_content()?)?);
                }
            }
            Event::CData(_) | Event::DocType(_) | Event::PI(_) => {
                return Err(anyhow::anyhow!("WinSW XML 含有不支持的声明或内容"))
            }
            Event::End(_) => {
                stack.pop().context("WinSW XML 节点不匹配")?;
            }
            Event::Eof => break,
            _ => {}
        }
    }
    if roots != 1 || !stack.is_empty() {
        return Err(anyhow::anyhow!("WinSW XML 未正确闭合"));
    }
    if fields.get("id").map(|value| value.trim()) != Some("MihomoService")
        || fields.get("executable").map(|value| value.trim()) != core.to_str()
    {
        return Err(anyhow::anyhow!("WinSW 服务标识或核心路径与本项目不一致"));
    }
    let arguments = fields
        .get("arguments")
        .context("WinSW 缺少 arguments")?
        .trim();
    let expected = expected_arguments(core, config)?;
    if !expected
        .iter()
        .any(|entry| arguments == format!("{} \"{}\"", entry[1], entry[2]))
    {
        return Err(anyhow::anyhow!("WinSW 参数未严格指向本项目配置"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn systemd_identity_and_arguments_must_match_exactly() {
        let core = Path::new("/opt/app/resources/mihomo");
        let config = Path::new("/home/user/mihomo/config.yaml");
        let mut value = serde_json::json!({"type":"a(sasbttttuii)","data":[[core,[core,"-d","/home/user/mihomo"],false,0,0,0,0,0,0,0]]});
        assert!(
            validate_systemd_execstart(&serde_json::to_vec(&value).unwrap(), core, config).is_ok()
        );
        value["data"][0][0] = serde_json::json!("/usr/bin/mihomo");
        assert!(
            validate_systemd_execstart(&serde_json::to_vec(&value).unwrap(), core, config).is_err()
        );
        value["data"][0][0] = serde_json::json!(core);
        value["data"][0][1][2] = serde_json::json!("/other/config");
        assert!(
            validate_systemd_execstart(&serde_json::to_vec(&value).unwrap(), core, config).is_err()
        );
    }
    #[test]
    fn winsw_identity_rejects_duplicate_fields_and_wrong_paths() {
        let core = Path::new("C:\\Program Files\\App\\resources\\mihomo.exe");
        // Use forward slashes in this cross-platform parser test for parent resolution.
        let config = Path::new("C:/AppData/mihomo/config.yaml");
        let xml = format!("<service><id>MihomoService</id><executable>{}</executable><arguments>-d &quot;C:/AppData/mihomo&quot;</arguments></service>",core.display());
        let result = validate_winsw_xml(&xml, core, config);
        assert!(result.is_ok(), "{:?}", result);
        assert!(validate_winsw_xml(
            &xml.replace("</service>", "<executable>other</executable></service>"),
            core,
            config
        )
        .is_err());
        assert!(
            validate_winsw_xml(&xml.replace("C:/AppData/mihomo", "C:/Other"), core, config)
                .is_err()
        );
        assert!(validate_winsw_xml(&format!("<!DOCTYPE service>{}", xml), core, config).is_err());
    }
}
