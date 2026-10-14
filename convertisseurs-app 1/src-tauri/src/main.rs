// Cache la fenêtre de console noire sous Windows en version finale.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::Serialize;
use std::path::PathBuf;
use std::process::{Command, Stdio};

#[derive(Serialize)]
struct AppInfo {
    version: String,
    os: &'static str,
    arch: &'static str,
}

/// Version installée, système et architecture (pour choisir le bon fichier à télécharger).
#[tauri::command]
fn app_info(app: tauri::AppHandle) -> AppInfo {
    AppInfo {
        version: app.package_info().version.to_string(),
        os: std::env::consts::OS,
        arch: std::env::consts::ARCH,
    }
}

/// N'accepte que les fichiers publiés sur GitHub Releases, avec des caractères sûrs.
fn valid_url(url: &str, ext: &str) -> bool {
    url.starts_with("https://github.com/")
        && url.contains("/releases/download/")
        && url.ends_with(ext)
        && url.len() < 300
        && url
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "-_./:".contains(c))
}

fn update_dir() -> Result<PathBuf, String> {
    let dir = std::env::temp_dir().join("convertisseurs-update");
    std::fs::create_dir_all(&dir).map_err(|e| format!("dossier temporaire : {e}"))?;
    Ok(dir)
}

/// Télécharge le fichier avec curl (présent sur Windows 10/11 et sur macOS).
fn download(url: &str, dest: &PathBuf) -> Result<(), String> {
    let _ = std::fs::remove_file(dest);
    let mut cmd = Command::new(if cfg!(windows) { "curl.exe" } else { "curl" });
    cmd.args([
        "-L",
        "--fail",
        "--silent",
        "--show-error",
        "--max-time",
        "900",
        "-o",
    ])
    .arg(dest)
    .arg(url)
    .stdin(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    let out = cmd
        .output()
        .map_err(|e| format!("téléchargement impossible : {e}"))?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
        return Err(format!("téléchargement échoué : {err}"));
    }
    let size = std::fs::metadata(dest).map(|m| m.len()).unwrap_or(0);
    if size < 100_000 {
        return Err("le fichier téléchargé est incomplet".into());
    }
    Ok(())
}

#[cfg(windows)]
fn launch_installer(file: &PathBuf) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    // On attend un instant que l'application se ferme, on installe, puis on la rouvre.
    let line = format!(
        "ping -n 3 127.0.0.1 >nul & msiexec /i \"{}\" /passive /norestart & start \"\" \"{}\"",
        file.display(),
        exe.display()
    );
    Command::new("cmd.exe")
        .arg("/C")
        .raw_arg(line)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .creation_flags(0x0800_0000 | 0x0000_0200) // CREATE_NO_WINDOW | CREATE_NEW_PROCESS_GROUP
        .spawn()
        .map_err(|e| format!("installation impossible : {e}"))?;
    Ok(())
}

#[cfg(target_os = "macos")]
fn launch_installer(file: &PathBuf) -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    // /Applications/Convertisseurs.app/Contents/MacOS/convertisseurs -> /Applications/Convertisseurs.app
    let app_path = exe
        .ancestors()
        .find(|p| p.extension().map(|e| e == "app").unwrap_or(false))
        .map(|p| p.display().to_string())
        .unwrap_or_default();
    // Copie la nouvelle version à la place de l'ancienne ; si ça échoue, ouvre simplement le .dmg.
    let script = r#"
sleep 2
MNT=$(mktemp -d /tmp/cv-mnt.XXXXXX) || { open "$CV_DMG"; exit 0; }
if hdiutil attach -nobrowse -readonly -quiet -mountpoint "$MNT" "$CV_DMG"; then
  SRC=$(ls -d "$MNT"/*.app 2>/dev/null | head -n 1)
  if [ -n "$SRC" ] && [ -n "$CV_APP" ] \
     && rm -rf "$CV_APP.new" && cp -R "$SRC" "$CV_APP.new" \
     && mv "$CV_APP" "$CV_APP.old" && mv "$CV_APP.new" "$CV_APP"; then
    rm -rf "$CV_APP.old"
    xattr -cr "$CV_APP" 2>/dev/null
    hdiutil detach -quiet "$MNT" 2>/dev/null
    open "$CV_APP"
    exit 0
  fi
  rm -rf "$CV_APP.new"
  hdiutil detach -quiet "$MNT" 2>/dev/null
fi
open "$CV_DMG"
"#;
    Command::new("sh")
        .arg("-c")
        .arg(script)
        .env("CV_DMG", file)
        .env("CV_APP", app_path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("installation impossible : {e}"))?;
    Ok(())
}

#[cfg(not(any(windows, target_os = "macos")))]
fn launch_installer(_file: &PathBuf) -> Result<(), String> {
    Err("mise à jour automatique non prise en charge sur ce système".into())
}

/// Télécharge la nouvelle version depuis GitHub, la lance, puis ferme l'application.
#[tauri::command]
async fn install_update(app: tauri::AppHandle, url: String) -> Result<(), String> {
    let ext = if cfg!(windows) { ".msi" } else { ".dmg" };
    if !valid_url(&url, ext) {
        return Err("adresse de mise à jour refusée".into());
    }
    let dest = update_dir()?.join(format!("Convertisseurs-update{ext}"));
    let (u, d) = (url.clone(), dest.clone());
    tauri::async_runtime::spawn_blocking(move || download(&u, &d))
        .await
        .map_err(|e| e.to_string())??;
    launch_installer(&dest)?;
    app.exit(0);
    Ok(())
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![app_info, install_update])
        .run(tauri::generate_context!())
        .expect("erreur au lancement de l'application");
}
