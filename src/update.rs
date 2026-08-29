use anyhow::{anyhow, bail, Context, Result};
use colored::*;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::util::command_exists;

const REPO: &str = "dominicOT/ship";
const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub struct UpdateOptions {
    /// Only check for an update, don't install it
    pub check: bool,
    /// Reinstall the latest release even if already up to date
    pub force: bool,
}

pub fn run(options: &UpdateOptions) -> Result<()> {
    println!("{}", "ship update".bold().cyan());
    println!();

    if !command_exists("curl") {
        bail!("curl is required to check for and download updates");
    }

    println!("  Current version: {}", CURRENT_VERSION.dimmed());

    let latest_tag = resolve_latest_tag()
        .context("Failed to resolve the latest release from GitHub")?;
    let latest_version = latest_tag.trim_start_matches('v');

    println!("  Latest version:  {}", latest_version.dimmed());
    println!();

    if latest_version == CURRENT_VERSION && !options.force {
        println!("{}", "✓ Already up to date".green().bold());
        return Ok(());
    }

    if options.check {
        println!(
            "{} update available: {} → {}",
            "⚠".yellow(),
            CURRENT_VERSION,
            latest_version
        );
        println!("  Run {} to install it", "ship update".bold());
        return Ok(());
    }

    let (os, arch) = target_os_arch()?;
    let (asset_name, is_zip) = asset_for(os, arch);

    let asset_url = format!(
        "https://github.com/{REPO}/releases/download/{latest_tag}/{asset_name}"
    );

    let tmp_dir = env::temp_dir().join(format!("ship-update-{}", std::process::id()));
    fs::create_dir_all(&tmp_dir)
        .with_context(|| format!("Failed to create temp dir: {}", tmp_dir.display()))?;

    struct CleanupGuard(PathBuf);
    impl Drop for CleanupGuard {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    let _cleanup = CleanupGuard(tmp_dir.clone());

    let asset_path = tmp_dir.join(&asset_name);
    println!("Downloading {}", asset_url.dimmed());
    download(&asset_url, &asset_path)?;

    let binary_name = if is_zip { "ship.exe" } else { "ship" };
    extract(&asset_path, &tmp_dir, is_zip)?;

    let extracted_binary = tmp_dir.join(binary_name);
    if !extracted_binary.is_file() {
        bail!(
            "downloaded archive did not contain {}",
            binary_name
        );
    }

    install_binary(&extracted_binary)?;

    println!();
    println!(
        "{} Updated ship {} → {}",
        "✓".green().bold(),
        CURRENT_VERSION,
        latest_version
    );

    Ok(())
}

/// Resolve the latest release tag (e.g. "v0.2.0") by following the
/// GitHub "latest release" redirect, mirroring scripts/install.sh's
/// approach of avoiding the rate-limited GitHub API.
fn resolve_latest_tag() -> Result<String> {
    let url = format!("https://github.com/{REPO}/releases/latest");

    let output = Command::new("curl")
        .args(["-fsSL", "-o", "/dev/null", "-w", "%{url_effective}", &url])
        .output()
        .context("Failed to run curl")?;

    if !output.status.success() {
        bail!(
            "curl exited with an error while resolving the latest release:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let final_url = String::from_utf8_lossy(&output.stdout).trim().to_string();
    final_url
        .rsplit('/')
        .next()
        .filter(|tag| !tag.is_empty() && tag.starts_with('v'))
        .map(|tag| tag.to_string())
        .ok_or_else(|| anyhow!("Could not determine latest release tag from {final_url}"))
}

fn target_os_arch() -> Result<(&'static str, &'static str)> {
    let os = match env::consts::OS {
        "linux" => "linux",
        "macos" => "macos",
        "windows" => "windows",
        other => bail!(
            "Unsupported OS: {other}. Install from source with: cargo install --path ."
        ),
    };

    let arch = match env::consts::ARCH {
        "x86_64" => "x86_64",
        "aarch64" => "aarch64",
        other => bail!(
            "Unsupported architecture: {other} on {os}. Install from source with: cargo install --path ."
        ),
    };

    Ok((os, arch))
}

fn asset_for(os: &str, arch: &str) -> (String, bool) {
    if os == "windows" {
        (format!("ship-{os}-{arch}.zip"), true)
    } else {
        (format!("ship-{os}-{arch}.tar.gz"), false)
    }
}

fn download(url: &str, dest: &Path) -> Result<()> {
    let status = Command::new("curl")
        .args(["-fsSL", url, "-o"])
        .arg(dest)
        .status()
        .context("Failed to run curl")?;

    if !status.success() {
        bail!("curl failed to download {url}");
    }

    Ok(())
}

fn extract(archive: &Path, dest_dir: &Path, is_zip: bool) -> Result<()> {
    if is_zip {
        if command_exists("unzip") {
            let status = Command::new("unzip")
                .args(["-q", "-o"])
                .arg(archive)
                .args(["-d"])
                .arg(dest_dir)
                .status()
                .context("Failed to run unzip")?;
            if !status.success() {
                bail!("unzip failed to extract {}", archive.display());
            }
        } else if command_exists("python3") {
            let status = Command::new("python3")
                .arg("-c")
                .arg("import zipfile, sys; zipfile.ZipFile(sys.argv[1]).extractall(sys.argv[2])")
                .arg(archive)
                .arg(dest_dir)
                .status()
                .context("Failed to run python3")?;
            if !status.success() {
                bail!("python3 failed to extract {}", archive.display());
            }
        } else {
            bail!("unzip or python3 is required to extract the downloaded archive");
        }
    } else {
        let status = Command::new("tar")
            .arg("-xzf")
            .arg(archive)
            .args(["-C"])
            .arg(dest_dir)
            .status()
            .context("Failed to run tar")?;
        if !status.success() {
            bail!("tar failed to extract {}", archive.display());
        }
    }

    Ok(())
}

#[cfg(unix)]
fn install_binary(new_binary: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let current_exe = env::current_exe().context("Failed to determine current executable path")?;
    let current_exe =
        fs::canonicalize(&current_exe).unwrap_or(current_exe);

    let mut perms = fs::metadata(new_binary)
        .context("Failed to read downloaded binary metadata")?
        .permissions();
    perms.set_mode(0o755);
    fs::set_permissions(new_binary, perms)
        .context("Failed to set downloaded binary permissions")?;

    // Stage the replacement next to the current binary so the final
    // rename is an atomic same-filesystem move. The running process
    // keeps its own file handle to the old inode, so this is safe
    // even while ship is executing.
    let staged = current_exe.with_extension("update");
    fs::copy(new_binary, &staged).with_context(|| {
        format!(
            "Failed to stage new binary at {}",
            staged.display()
        )
    })?;

    fs::rename(&staged, &current_exe).with_context(|| {
        format!(
            "Failed to replace {} — check write permissions (you may need sudo)",
            current_exe.display()
        )
    })?;

    Ok(())
}

#[cfg(windows)]
fn install_binary(new_binary: &Path) -> Result<()> {
    let current_exe = env::current_exe().context("Failed to determine current executable path")?;

    // Windows won't let us overwrite a running executable in place, so
    // move the running binary aside first, then move the new one in.
    let backup = current_exe.with_extension("exe.old");
    let _ = fs::remove_file(&backup);
    fs::rename(&current_exe, &backup).with_context(|| {
        format!(
            "Failed to move aside {} — check write permissions",
            current_exe.display()
        )
    })?;

    if let Err(e) = fs::copy(new_binary, &current_exe).with_context(|| {
        format!("Failed to install new binary to {}", current_exe.display())
    }) {
        // Best-effort rollback
        let _ = fs::rename(&backup, &current_exe);
        return Err(e);
    }

    println!(
        "  {} Old binary kept at {} (delete it manually once you've confirmed the update works)",
        "–".dimmed(),
        backup.display()
    );

    Ok(())
}
