//! One selection policy for process launch, service installation and diagnostics.
use anyhow::{Context, Result};
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

static RESOURCE_DIRECTORY: OnceLock<PathBuf> = OnceLock::new();

/// Called from Tauri setup so packaged resources use the platform's actual layout.
pub fn initialize(resource_directory: Option<PathBuf>) {
    if let Some(directory) = resource_directory {
        let _ = RESOURCE_DIRECTORY.set(directory);
    }
}

fn resource_candidates(name: &str) -> Result<Vec<PathBuf>> {
    let mut candidates = Vec::new();
    if let Some(directory) = RESOURCE_DIRECTORY.get() {
        candidates.push(directory.join(name));
    }
    #[cfg(debug_assertions)]
    candidates.push(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("resources")
            .join(name),
    );
    let executable = std::env::current_exe().context("获取应用路径失败")?;
    let directory = executable.parent().context("获取应用目录失败")?;
    candidates.push(directory.join("resources").join(name));
    Ok(candidates)
}

fn first_present(candidates: impl IntoIterator<Item = PathBuf>) -> Result<Option<PathBuf>> {
    for path in candidates {
        match std::fs::metadata(&path) {
            Ok(metadata) => {
                if !metadata.is_file() {
                    return Err(anyhow::anyhow!("核心资源不是普通文件: {}", path.display()));
                }
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    if metadata.permissions().mode() & 0o111 == 0 {
                        return Err(anyhow::anyhow!("核心资源不可执行: {}", path.display()));
                    }
                }
                return Ok(Some(path));
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(error).with_context(|| format!("读取核心资源失败: {}", path.display()))
            }
        }
    }
    Ok(None)
}

#[cfg(target_os = "windows")]
pub fn resolve_bundled_resource(name: &str) -> Result<PathBuf> {
    let mut candidates = resource_candidates(name)?;
    if let Some(directory) = std::env::current_exe()?.parent() {
        candidates.push(directory.join(name));
    }
    first_present(candidates)?
        .ok_or_else(|| anyhow::anyhow!("未找到应用资源 {}，请重新准备并构建资源", name))
}

pub fn resolve_mihomo() -> Result<PathBuf> {
    #[cfg(target_os = "windows")]
    let name = "mihomo.exe";
    #[cfg(not(target_os = "windows"))]
    let name = "mihomo";
    if let Some(path) = first_present(resource_candidates(name)?)? {
        tracing::info!("Using application Mihomo resource: {}", path.display());
        return Ok(path);
    }

    // Compatibility for installations that deliberately provide a separate core.
    // A present but unusable application resource fails above instead of silently
    // running an unrelated older system version.
    let mut system_candidates = Vec::new();
    if let Some(directory) = std::env::current_exe()?.parent() {
        system_candidates.push(directory.join(name));
    }
    #[cfg(not(target_os = "windows"))]
    system_candidates.extend([
        PathBuf::from("/usr/local/bin/mihomo"),
        PathBuf::from("/usr/bin/mihomo"),
        PathBuf::from("/opt/mihomo/mihomo"),
    ]);
    if let Some(paths) = std::env::var_os("PATH") {
        system_candidates.extend(
            std::env::split_paths(&paths)
                .filter(|directory| directory.is_absolute())
                .map(|directory| directory.join(name)),
        );
    }
    if let Some(path) = first_present(system_candidates)? {
        verify_system_core(&path)
            .with_context(|| format!("系统兼容核心不满足项目要求: {}", path.display()))?;
        tracing::warn!(
            "Application Mihomo resource absent; using system compatibility core: {}",
            path.display()
        );
        return Ok(path);
    }
    Err(anyhow::anyhow!(
        "未找到 Mihomo 应用资源或系统兼容核心，请运行资源准备脚本并重新构建"
    ))
}

fn stable_version(version: &str) -> Result<(u32, u32, u32)> {
    let parts: Vec<_> = version
        .strip_prefix('v')
        .context("核心版本必须为 vX.Y.Z 正式版本")?
        .split('.')
        .collect();
    if parts.len() != 3
        || parts
            .iter()
            .any(|part| part.is_empty() || !part.bytes().all(|byte| byte.is_ascii_digit()))
    {
        return Err(anyhow::anyhow!("无法识别核心正式版本"));
    }
    Ok((parts[0].parse()?, parts[1].parse()?, parts[2].parse()?))
}

fn validate_system_version(reported: &str, minimum: &str) -> Result<()> {
    let mut words = reported.split_whitespace();
    if words.next() != Some("Mihomo") || words.next() != Some("Meta") {
        return Err(anyhow::anyhow!(
            "系统核心未报告可识别的 Mihomo Meta 正式版本"
        ));
    }
    let actual = words.next().context("系统核心未报告版本")?;
    if stable_version(actual).context("系统核心版本无法识别或不是正式版本")?
        < stable_version(minimum).context("项目资源版本基线无效")?
    {
        return Err(anyhow::anyhow!(
            "系统 Mihomo {} 低于项目最低要求 {}，请准备项目资源或升级系统兼容核心",
            actual,
            minimum
        ));
    }
    Ok(())
}

fn verify_system_core(path: &std::path::Path) -> Result<()> {
    let manifest: serde_json::Value =
        serde_json::from_str(include_str!("../../resource-versions.json"))?;
    let minimum = manifest["mihomo"]
        .as_str()
        .context("项目资源版本基线缺失")?;
    // A temporary file avoids pipe-buffer deadlocks, and -v never invokes a shell.
    // Bound the child lifetime so diagnostics cannot indefinitely block startup.
    let output_path =
        std::env::temp_dir().join(format!("mihomo-version-{}.txt", uuid::Uuid::new_v4()));
    struct VersionOutput(std::path::PathBuf);
    impl Drop for VersionOutput {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }
    let output_guard = VersionOutput(output_path);
    let output = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&output_guard.0)?;
    let mut command = std::process::Command::new(path);
    command
        .arg("-v")
        .stdin(std::process::Stdio::null())
        .stdout(output)
        .stderr(std::process::Stdio::null());
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let mut child = command.spawn().context("系统核心 -v 执行失败")?;
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        let status = match child.try_wait() {
            Ok(status) => status,
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(error).context("无法检查系统核心版本进程");
            }
        };
        match status {
            Some(status) if status.success() => break,
            Some(_) => return Err(anyhow::anyhow!("系统核心 -v 返回失败")),
            None if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(anyhow::anyhow!("系统核心版本检查超时"));
            }
            None => std::thread::sleep(Duration::from_millis(20)),
        }
    }
    use std::io::Read;
    let mut reported = String::new();
    std::fs::File::open(&output_guard.0)?
        .take(8192)
        .read_to_string(&mut reported)?;
    validate_system_version(&reported, minimum)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_core_must_meet_the_manifest_baseline_as_a_stable_release() {
        assert!(validate_system_version("Mihomo Meta v1.19.32 linux amd64", "v1.19.33").is_err());
        assert!(validate_system_version("Mihomo Meta v1.19.33 linux amd64", "v1.19.33").is_ok());
        assert!(validate_system_version("Mihomo Meta v1.20.0 linux amd64", "v1.19.33").is_ok());
        assert!(validate_system_version("Mihomo Meta v2.0.0 linux amd64", "v1.19.33").is_ok());
        for output in [
            "Mihomo Meta alpha linux amd64",
            "Mihomo Meta v1.19.33-alpha linux amd64",
            "clash v1.99.0",
            "",
            "Mihomo Meta v1.19",
        ] {
            assert!(validate_system_version(output, "v1.19.33").is_err());
        }
    }

    #[test]
    fn packaged_candidate_wins_over_legacy_or_system_candidate() {
        let directory =
            std::env::temp_dir().join(format!("mihomo-selection-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&directory).unwrap();
        let paths = [
            directory.join("packaged"),
            directory.join("legacy"),
            directory.join("system"),
        ];
        for path in &paths {
            std::fs::write(path, b"fixture").unwrap();
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
            }
        }
        assert_eq!(
            first_present(paths.clone()).unwrap(),
            Some(paths[0].clone())
        );
        std::fs::remove_file(&paths[0]).unwrap();
        assert_eq!(
            first_present(paths.clone()).unwrap(),
            Some(paths[1].clone())
        );
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn invalid_packaged_candidate_never_silently_falls_back() {
        let directory =
            std::env::temp_dir().join(format!("mihomo-selection-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&directory).unwrap();
        let fallback = directory.join("system");
        std::fs::write(&fallback, b"fixture").unwrap();
        assert!(first_present([directory.clone(), fallback]).is_err());
        std::fs::remove_dir_all(directory).unwrap();
    }
}
