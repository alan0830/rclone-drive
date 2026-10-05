use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::thread;
use std::time::Duration;
use serde::{Deserialize, Serialize};
use tauri::{
    menu::{MenuBuilder, MenuItemBuilder},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager, State,
};

const CREATE_NO_WINDOW: u32 = 0x08000000;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentStatus {
    pub rclone_found: bool,
    pub rclone_path: String,
    pub rclone_version: String,
    pub winfsp_found: bool,
    pub webgui_running: bool,
    pub webgui_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteInfo {
    pub name: String,
    pub remote_type: String,
    pub is_mounted: bool,
    pub mounted_drive: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MountInstance {
    pub remote: String,
    pub drive_letter: String,
    pub pid: u32,
    pub remote_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub start_minimized_to_tray: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            start_minimized_to_tray: true,
        }
    }
}

// Embedded Cloud Drive Icons
const ICON_GDRIVE: &[u8] = include_bytes!("../icons/drive_icons/gdrive.ico");
const ICON_DROPBOX: &[u8] = include_bytes!("../icons/drive_icons/dropbox.ico");
const ICON_ONEDRIVE: &[u8] = include_bytes!("../icons/drive_icons/onedrive.ico");
const ICON_WEBDAV: &[u8] = include_bytes!("../icons/drive_icons/webdav.ico");
const ICON_MEGA: &[u8] = include_bytes!("../icons/drive_icons/mega.ico");
const ICON_BOX: &[u8] = include_bytes!("../icons/drive_icons/box.ico");
const ICON_PCLOUD: &[u8] = include_bytes!("../icons/drive_icons/pcloud.ico");
const ICON_S3: &[u8] = include_bytes!("../icons/drive_icons/s3.ico");
const ICON_FTP: &[u8] = include_bytes!("../icons/drive_icons/ftp.ico");
const ICON_DEFAULT: &[u8] = include_bytes!("../icons/drive_icons/default_cloud.ico");

#[link(name = "shell32")]
extern "system" {
    fn SHChangeNotify(
        w_event_id: i32,
        u_flags: u32,
        dw_item_1: *const std::ffi::c_void,
        dw_item_2: *const std::ffi::c_void,
    );
}

pub fn refresh_shell_icons() {
    unsafe {
        // SHCNE_ASSOCCHANGED = 0x08000000, SHCNF_IDLIST = 0
        SHChangeNotify(0x08000000, 0, std::ptr::null(), std::ptr::null());
    }
}

fn dirs_fallback_local_appdata() -> std::path::PathBuf {
    if let Ok(val) = std::env::var("LOCALAPPDATA") {
        std::path::PathBuf::from(val)
    } else if let Ok(val) = std::env::var("USERPROFILE") {
        std::path::PathBuf::from(val).join("AppData").join("Local")
    } else {
        std::path::PathBuf::from(r"C:\ProgramData")
    }
}

fn dirs_fallback_appdata() -> std::path::PathBuf {
    if let Ok(val) = std::env::var("APPDATA") {
        std::path::PathBuf::from(val)
    } else if let Ok(val) = std::env::var("USERPROFILE") {
        std::path::PathBuf::from(val).join("AppData").join("Roaming")
    } else {
        std::path::PathBuf::from(r"C:\ProgramData")
    }
}

fn ensure_drive_icons() -> std::path::PathBuf {
    let base_dir = dirs_fallback_local_appdata().join("RcloneDrive").join("icons");
    let _ = std::fs::create_dir_all(&base_dir);

    let icons: [(&str, &[u8]); 10] = [
        ("gdrive.ico", ICON_GDRIVE),
        ("dropbox.ico", ICON_DROPBOX),
        ("onedrive.ico", ICON_ONEDRIVE),
        ("webdav.ico", ICON_WEBDAV),
        ("mega.ico", ICON_MEGA),
        ("box.ico", ICON_BOX),
        ("pcloud.ico", ICON_PCLOUD),
        ("s3.ico", ICON_S3),
        ("ftp.ico", ICON_FTP),
        ("default_cloud.ico", ICON_DEFAULT),
    ];
    for (filename, data) in icons {
        let p = base_dir.join(filename);
        if !p.exists() || std::fs::metadata(&p).map(|m| m.len()).unwrap_or(0) != data.len() as u64 {
            let _ = std::fs::write(&p, data);
        }
    }
    base_dir
}

fn get_icon_filename_for_type(remote_type: &str) -> &'static str {
    let t = remote_type.to_lowercase();
    if t.contains("onedrive") {
        "onedrive.ico"
    } else if t.contains("drive") || t == "gdrive" {
        "gdrive.ico"
    } else if t.contains("dropbox") {
        "dropbox.ico"
    } else if t.contains("webdav") || t.contains("nextcloud") || t.contains("owncloud") {
        "webdav.ico"
    } else if t.contains("mega") {
        "mega.ico"
    } else if t.contains("box") {
        "box.ico"
    } else if t.contains("pcloud") {
        "pcloud.ico"
    } else if t.contains("s3") {
        "s3.ico"
    } else if t.contains("ftp") || t.contains("sftp") {
        "ftp.ico"
    } else {
        "default_cloud.ico"
    }
}

pub fn set_windows_drive_icon(drive_letter: &str, remote_type: &str, volname: Option<&str>) {
    let clean_letter = drive_letter.trim().trim_end_matches(':').trim_end_matches('\\').to_uppercase();
    if clean_letter.is_empty() {
        return;
    }
    let icons_dir = ensure_drive_icons();
    let icon_file = get_icon_filename_for_type(remote_type);
    let ico_path = icons_dir.join(icon_file).to_string_lossy().to_string();

    // 1. HKCU\Software\Classes\Applications\Explorer.exe\Drives\<Letter>\DefaultIcon
    let key1 = format!(r"HKCU\Software\Classes\Applications\Explorer.exe\Drives\{}\DefaultIcon", clean_letter);
    let mut cmd1 = Command::new("reg");
    cmd1.args(["add", &key1, "/ve", "/d", &ico_path, "/f"]);
    cmd1.creation_flags(CREATE_NO_WINDOW);
    let _ = cmd1.output();

    // 2. HKCU\Software\Microsoft\Windows\CurrentVersion\Explorer\DriveIcons\<Letter>\DefaultIcon
    let key2 = format!(r"HKCU\Software\Microsoft\Windows\CurrentVersion\Explorer\DriveIcons\{}\DefaultIcon", clean_letter);
    let mut cmd2 = Command::new("reg");
    cmd2.args(["add", &key2, "/ve", "/d", &ico_path, "/f"]);
    cmd2.creation_flags(CREATE_NO_WINDOW);
    let _ = cmd2.output();

    // 3. HKCU\Software\Microsoft\Windows\CurrentVersion\Explorer\MountPoints2\##server#<volname>\_Autorun\DefaultIcon
    if let Some(vn) = volname {
        let key3 = format!(r"HKCU\Software\Microsoft\Windows\CurrentVersion\Explorer\MountPoints2\##server#{}\_Autorun\DefaultIcon", vn);
        let mut cmd3 = Command::new("reg");
        cmd3.args(["add", &key3, "/ve", "/d", &ico_path, "/f"]);
        cmd3.creation_flags(CREATE_NO_WINDOW);
        let _ = cmd3.output();
    }

    refresh_shell_icons();
}

pub fn remove_windows_drive_icon(drive_letter: &str, volname: Option<&str>) {
    let clean_letter = drive_letter.trim().trim_end_matches(':').trim_end_matches('\\').to_uppercase();
    if clean_letter.is_empty() {
        return;
    }

    let key1 = format!(r"HKCU\Software\Classes\Applications\Explorer.exe\Drives\{}", clean_letter);
    let mut cmd1 = Command::new("reg");
    cmd1.args(["delete", &key1, "/f"]);
    cmd1.creation_flags(CREATE_NO_WINDOW);
    let _ = cmd1.output();

    let key2 = format!(r"HKCU\Software\Microsoft\Windows\CurrentVersion\Explorer\DriveIcons\{}", clean_letter);
    let mut cmd2 = Command::new("reg");
    cmd2.args(["delete", &key2, "/f"]);
    cmd2.creation_flags(CREATE_NO_WINDOW);
    let _ = cmd2.output();

    if let Some(vn) = volname {
        let key3 = format!(r"HKCU\Software\Microsoft\Windows\CurrentVersion\Explorer\MountPoints2\##server#{}\_Autorun", vn);
        let mut cmd3 = Command::new("reg");
        cmd3.args(["delete", &key3, "/f"]);
        cmd3.creation_flags(CREATE_NO_WINDOW);
        let _ = cmd3.output();
    }

    refresh_shell_icons();
}

fn get_settings_file_path() -> std::path::PathBuf {
    let dir = dirs_fallback_appdata().join("com.rclonedrive.desktop");
    let _ = std::fs::create_dir_all(&dir);
    dir.join("settings.json")
}

fn load_app_settings() -> AppSettings {
    let p = get_settings_file_path();
    if let Ok(data) = std::fs::read_to_string(&p) {
        if let Ok(settings) = serde_json::from_str::<AppSettings>(&data) {
            return settings;
        }
    }
    AppSettings::default()
}

fn save_app_settings_to_file(settings: &AppSettings) -> Result<(), String> {
    let p = get_settings_file_path();
    let json = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
    std::fs::write(&p, json).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn get_app_settings() -> Result<AppSettings, String> {
    Ok(load_app_settings())
}

#[tauri::command]
fn save_app_settings(settings: AppSettings) -> Result<(), String> {
    save_app_settings_to_file(&settings)
}

#[tauri::command]
fn apply_mounted_drive_icons(remotes: Vec<RemoteInfo>) -> Result<(), String> {
    for r in remotes {
        if r.is_mounted {
            if let Some(drive) = r.mounted_drive {
                set_windows_drive_icon(&drive, &r.remote_type, Some(&r.name));
            }
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffItem {
    pub path: String,
    pub status: String, // "source_only" (+), "dest_only" (-), "different" (*), "equal" (=), "error" (!)
    pub symbol: String,
    pub size_src: Option<String>,
    pub mtime_src: Option<String>,
    pub size_dest: Option<String>,
    pub mtime_dest: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffSummary {
    pub total_source_files: usize,
    pub total_dest_files: usize,
    pub total_different: usize,
    pub total_equal: usize,
    pub total_error: usize,
    pub items: Vec<DiffItem>,
    pub differences: Vec<String>,
    pub message: String,
}

#[derive(Default)]
pub struct AppState {
    pub mounts: Mutex<HashMap<String, MountInstance>>,
    pub webgui_child: Mutex<Option<u32>>,
}

fn resolve_rclone_path(custom_path: Option<String>) -> String {
    if let Some(path) = custom_path {
        if !path.trim().is_empty() && Path::new(&path).exists() {
            return path;
        }
    }
    let default_path = "C:\\rclone\\rclone.exe";
    if Path::new(default_path).exists() {
        return default_path.to_string();
    }
    if let Ok(local_appdata) = std::env::var("LOCALAPPDATA") {
        let appdata_path = Path::new(&local_appdata).join("RcloneDrive").join("bin").join("rclone.exe");
        if appdata_path.exists() {
            return appdata_path.to_string_lossy().to_string();
        }
    }
    "rclone".to_string()
}

#[tauri::command]
async fn auto_install_rclone() -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let script = r#"
            $ErrorActionPreference = 'Stop'
            [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
            $zipUrl = "https://downloads.rclone.org/rclone-current-windows-amd64.zip"
            $tempZip = Join-Path $env:TEMP "rclone-setup.zip"
            $tempExtract = Join-Path $env:TEMP "rclone-setup-extracted"

            Invoke-WebRequest -Uri $zipUrl -OutFile $tempZip -UseBasicParsing

            if (Test-Path $tempExtract) { Remove-Item -Recurse -Force $tempExtract }
            Expand-Archive -Path $tempZip -DestinationPath $tempExtract -Force

            $foundExe = Get-ChildItem -Path $tempExtract -Recurse -Filter "rclone.exe" | Select-Object -First 1
            if (-not $foundExe) {
                throw "無法在下載的壓縮包中找到 rclone.exe"
            }

            $targetDir = "C:\rclone"
            $canWriteC = $true
            try {
                if (-not (Test-Path $targetDir)) {
                    New-Item -ItemType Directory -Force -Path $targetDir -ErrorAction Stop | Out-Null
                }
                Copy-Item -Path $foundExe.FullName -Destination (Join-Path $targetDir "rclone.exe") -Force -ErrorAction Stop
            } catch {
                $canWriteC = $false
            }

            if (-not $canWriteC) {
                $targetDir = Join-Path $env:LOCALAPPDATA "RcloneDrive\bin"
                if (-not (Test-Path $targetDir)) {
                    New-Item -ItemType Directory -Force -Path $targetDir | Out-Null
                }
                Copy-Item -Path $foundExe.FullName -Destination (Join-Path $targetDir "rclone.exe") -Force
            }

            Remove-Item -Force $tempZip -ErrorAction SilentlyContinue
            Remove-Item -Recurse -Force $tempExtract -ErrorAction SilentlyContinue

            $finalExe = Join-Path $targetDir "rclone.exe"
            Write-Output $finalExe
        "#;

        let mut cmd = Command::new("powershell.exe");
        cmd.args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-Command", script]);
        cmd.creation_flags(CREATE_NO_WINDOW);
        let output = cmd.output().map_err(|e| format!("執行下載安裝失敗: {}", e))?;
        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            let out = String::from_utf8_lossy(&output.stdout);
            return Err(format!("安裝 Rclone 失敗: {} {}", err.trim(), out.trim()));
        }
        let installed_path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if installed_path.is_empty() {
            return Err("下載完成但未取得安裝路徑".to_string());
        }
        Ok(installed_path)
    })
    .await
    .map_err(|e| format!("執行緒錯誤: {}", e))?
}

#[tauri::command]
async fn auto_install_winfsp() -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let script = r#"
            $ErrorActionPreference = 'Stop'
            [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
            $msiUrl = "https://github.com/winfsp/winfsp/releases/download/v2.0/winfsp-2.0.23075.msi"
            $tempMsi = Join-Path $env:TEMP "winfsp-installer.msi"

            Invoke-WebRequest -Uri $msiUrl -OutFile $tempMsi -UseBasicParsing

            Start-Process msiexec.exe -ArgumentList "/i `"$tempMsi`"" -Wait

            Remove-Item -Force $tempMsi -ErrorAction SilentlyContinue
            Write-Output "OK"
        "#;

        let mut cmd = Command::new("powershell.exe");
        cmd.args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-Command", script]);
        cmd.creation_flags(CREATE_NO_WINDOW);
        let output = cmd.output().map_err(|e| format!("啟動 WinFsp 安裝程式失敗: {}", e))?;
        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            return Err(format!("下載或安裝 WinFsp 失敗: {}", err.trim()));
        }
        Ok("WinFsp 安裝程式已執行完畢".to_string())
    })
    .await
    .map_err(|e| format!("執行緒錯誤: {}", e))?
}

#[tauri::command]
fn check_environment(state: State<'_, AppState>, rclone_path: Option<String>) -> EnvironmentStatus {
    let exe = resolve_rclone_path(rclone_path);
    let mut status = EnvironmentStatus {
        rclone_found: false,
        rclone_path: exe.clone(),
        rclone_version: String::new(),
        winfsp_found: false,
        webgui_running: false,
        webgui_url: "http://127.0.0.1:5572/".to_string(),
    };

    let mut winfsp_paths = vec![
        "C:\\Program Files (x86)\\WinFsp".to_string(),
        "C:\\Program Files\\WinFsp".to_string(),
    ];
    if let Ok(pf) = std::env::var("ProgramFiles") {
        winfsp_paths.push(format!("{}\\WinFsp", pf));
    }
    if let Ok(pfx86) = std::env::var("ProgramFiles(x86)") {
        winfsp_paths.push(format!("{}\\WinFsp", pfx86));
    }
    for p in winfsp_paths {
        if Path::new(&p).exists() {
            status.winfsp_found = true;
            break;
        }
    }

    let mut cmd = Command::new(&exe);
    cmd.arg("version");
    cmd.creation_flags(CREATE_NO_WINDOW);
    if let Ok(output) = cmd.output() {
        if output.status.success() {
            status.rclone_found = true;
            let out_str = String::from_utf8_lossy(&output.stdout);
            if let Some(first_line) = out_str.lines().next() {
                status.rclone_version = first_line.trim().to_string();
            }
        }
    }

    let gui_guard = state.webgui_child.lock().unwrap();
    status.webgui_running = gui_guard.is_some();

    status
}

#[tauri::command]
fn get_available_drives(state: State<'_, AppState>) -> Vec<String> {
    let mounts = state.mounts.lock().unwrap();
    let occupied_by_us: Vec<String> = mounts.values().map(|m| m.drive_letter.clone()).collect();

    let mut available = Vec::new();
    for c in ('D'..='Z').rev() {
        let drive_root = format!("{}:\\", c);
        let drive_tag = format!("{}:", c);

        if !occupied_by_us.contains(&drive_tag) && !Path::new(&drive_root).exists() {
            available.push(drive_tag);
        }
    }
    available
}

#[tauri::command]
fn get_remotes(
    state: State<'_, AppState>,
    rclone_path: Option<String>,
) -> Result<Vec<RemoteInfo>, String> {
    let exe = resolve_rclone_path(rclone_path);
    let mut cmd = Command::new(&exe);
    cmd.args(["config", "dump"]);
    cmd.creation_flags(CREATE_NO_WINDOW);

    let output = cmd.output().map_err(|e| format!("無法執行 rclone config dump: {}", e))?;
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(format!("讀取 rclone 設定失敗: {}", err));
    }

    let json_val: serde_json::Value = serde_json::from_slice(&output.stdout)
        .map_err(|e| format!("解析 rclone 設定 JSON 失敗: {}", e))?;

    let mounts = state.mounts.lock().unwrap();
    let mut remotes = Vec::new();

    if let serde_json::Value::Object(map) = json_val {
        for (name, val) in map {
            let r_type = val
                .get("type")
                .and_then(|t| t.as_str())
                .unwrap_or("unknown")
                .to_string();

            let is_mounted = mounts.contains_key(&name);
            let mounted_drive = mounts.get(&name).map(|m| m.drive_letter.clone());

            remotes.push(RemoteInfo {
                name,
                remote_type: r_type,
                is_mounted,
                mounted_drive,
            });
        }
    }

    remotes.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(remotes)
}

fn get_rclone_conf_path(exe: &str) -> Option<PathBuf> {
    let mut cmd = Command::new(exe);
    cmd.args(["config", "file"]);
    cmd.creation_flags(CREATE_NO_WINDOW);
    if let Ok(output) = cmd.output() {
        if output.status.success() {
            let out_str = String::from_utf8_lossy(&output.stdout);
            for line in out_str.lines() {
                let trimmed = line.trim();
                if trimmed.ends_with("rclone.conf") && Path::new(trimmed).exists() {
                    return Some(PathBuf::from(trimmed));
                }
            }
        }
    }
    if let Ok(appdata) = std::env::var("APPDATA") {
        let p = PathBuf::from(appdata).join("rclone").join("rclone.conf");
        if p.exists() {
            return Some(p);
        }
    }
    if let Ok(userprofile) = std::env::var("USERPROFILE") {
        let p = PathBuf::from(userprofile).join(".config").join("rclone").join("rclone.conf");
        if p.exists() {
            return Some(p);
        }
    }
    None
}

fn auto_heal_onedrive_config(exe: &str, remote_name: &str) -> Result<(), String> {
    let conf_path = match get_rclone_conf_path(exe) {
        Some(p) => p,
        None => return Ok(()),
    };
    if !conf_path.exists() {
        return Ok(());
    }
    let content = fs::read_to_string(&conf_path).map_err(|e| e.to_string())?;

    let section_header = format!("[{}]", remote_name);
    let sec_start = match content.find(&section_header) {
        Some(idx) => idx,
        None => return Ok(()),
    };

    let rest = &content[sec_start..];
    let next_sec = rest[1..].find('[').map(|idx| idx + 1).unwrap_or(rest.len());
    let sec_body = &rest[..next_sec];

    let is_onedrive = sec_body.lines().any(|l| {
        let t = l.trim().to_lowercase();
        t == "type = onedrive" || t == "type=onedrive"
    });
    if !is_onedrive {
        return Ok(());
    }

    let has_drive_id = sec_body.lines().any(|l| l.trim().starts_with("drive_id"));
    let has_drive_type = sec_body.lines().any(|l| l.trim().starts_with("drive_type"));
    if has_drive_id && has_drive_type {
        return Ok(());
    }

    let token_line = sec_body.lines().find(|l| l.trim().starts_with("token"));
    let json_part = match token_line.and_then(|tl| tl.split_once('=')) {
        Some((_, val)) => val.trim(),
        None => return Ok(()),
    };

    let parsed: serde_json::Value = match serde_json::from_str(json_part) {
        Ok(v) => v,
        Err(_) => return Ok(()),
    };

    let access_token = match parsed.get("access_token").and_then(|a| a.as_str()) {
        Some(t) => t,
        None => return Ok(()),
    };

    let ps_script = format!(
        r#"$token = '{}'; $h = @{{ Authorization = "Bearer $token" }}; $r = Invoke-RestMethod -Uri "https://graph.microsoft.com/v1.0/me/drive" -Headers $h -TimeoutSec 10; "$($r.id)|$($r.driveType)""#,
        access_token.replace('\'', "''")
    );

    let mut cmd = Command::new("powershell");
    cmd.args(["-NoProfile", "-NonInteractive", "-Command", &ps_script]);
    cmd.creation_flags(CREATE_NO_WINDOW);

    if let Ok(output) = cmd.output() {
        if output.status.success() {
            let res = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if let Some((d_id, d_type)) = res.split_once('|') {
                let d_id = d_id.trim();
                let d_type = d_type.trim();
                if !d_id.is_empty() {
                    let mut new_lines = Vec::new();
                    let mut inserted = false;
                    for line in content.lines() {
                        new_lines.push(line.to_string());
                        if line.trim() == section_header && !inserted {
                            if !has_drive_id {
                                new_lines.push(format!("drive_id = {}", d_id));
                            }
                            if !has_drive_type {
                                new_lines.push(format!("drive_type = {}", if d_type.is_empty() { "personal" } else { d_type }));
                            }
                            inserted = true;
                        }
                    }
                    let _ = fs::write(&conf_path, new_lines.join("\r\n"));
                }
            }
        }
    }

    Ok(())
}

#[tauri::command]
fn mount_remote(
    state: State<'_, AppState>,
    rclone_path: Option<String>,
    remote: String,
    drive_letter: String,
    volname: Option<String>,
    cache_mode: Option<String>,
    read_only: Option<bool>,
    remote_type: Option<String>,
) -> Result<MountInstance, String> {
    let exe = resolve_rclone_path(rclone_path);

    {
        let mounts = state.mounts.lock().unwrap();
        if mounts.contains_key(&remote) {
            return Err(format!("遠端 '{}' 已經掛載中！", remote));
        }
    }

    let clean_letter = drive_letter.trim().trim_end_matches(':').trim_end_matches('\\').to_uppercase();
    let root_path = format!("{clean_letter}:\\");
    if Path::new(&root_path).exists() {
        return Err(format!("磁碟機代號 {clean_letter}: 已被本機其他裝置佔用！"));
    }

    let v_name = volname.unwrap_or_else(|| remote.clone());
    let c_mode = cache_mode.unwrap_or_else(|| "full".to_string());

    // Auto-heal onedrive drive_id if missing before mounting
    let _ = auto_heal_onedrive_config(&exe, &remote);

    let mut cmd = Command::new(&exe);
    cmd.arg("mount");
    cmd.arg(format!("{}:", remote));
    cmd.arg(format!("{}:", clean_letter));
    cmd.arg("--vfs-cache-mode");
    cmd.arg(&c_mode);
    cmd.arg("--volname");
    cmd.arg(&v_name);
    cmd.arg("--network-mode"); // Always mount as Windows network drive for 100% stability without Admin UAC!

    if read_only.unwrap_or(false) {
        cmd.arg("--read-only");
    }

    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd.stderr(Stdio::piped());

    let mut child = cmd.spawn().map_err(|e| format!("啟動 rclone 掛載行程失敗: {}", e))?;
    let pid = child.id();

    let mut mounted = false;
    for _ in 0..30 {
        thread::sleep(Duration::from_millis(300));
        if let Ok(Some(status)) = child.try_wait() {
            let mut err_msg = String::new();
            if let Some(mut stderr) = child.stderr.take() {
                let _ = stderr.read_to_string(&mut err_msg);
            }
            let detail = if err_msg.trim().is_empty() {
                format!("狀態碼: {:?}", status.code())
            } else {
                err_msg.trim().to_string()
            };
            return Err(format!("掛載失敗: rclone 行程已退出 ({detail})。請確認代號是否衝突或設定是否有效。"));
        }
        if Path::new(&root_path).exists() {
            mounted = true;
            break;
        }
    }

    // If drive root exists OR child process is still actively running after 9 seconds, accept as mounted
    if !mounted && child.try_wait().ok().flatten().is_none() {
        mounted = true;
    }

    if !mounted {
        let mut kill_cmd = Command::new("taskkill");
        kill_cmd.args(["/F", "/PID", &pid.to_string(), "/T"]);
        kill_cmd.creation_flags(CREATE_NO_WINDOW);
        let _ = kill_cmd.output();
        return Err(format!("掛載逾時，磁碟機 {clean_letter}: 未能及時就緒。請確認 WinFsp 正常運行。"));
    }

    // Set Windows drive icon in Explorer
    let r_type = remote_type.clone().unwrap_or_else(|| "default".to_string());
    set_windows_drive_icon(&clean_letter, &r_type, Some(&v_name));

    let instance = MountInstance {
        remote: remote.clone(),
        drive_letter: clean_letter,
        pid,
        remote_type,
    };

    let mut mounts = state.mounts.lock().unwrap();
    mounts.insert(remote, instance.clone());

    Ok(instance)
}

#[tauri::command]
fn unmount_remote(state: State<'_, AppState>, remote: String) -> Result<bool, String> {
    let mut mounts = state.mounts.lock().unwrap();
    if let Some(instance) = mounts.remove(&remote) {
        remove_windows_drive_icon(&instance.drive_letter, Some(&remote));

        let mut kill_cmd = Command::new("taskkill");
        kill_cmd.args(["/F", "/PID", &instance.pid.to_string(), "/T"]);
        kill_cmd.creation_flags(CREATE_NO_WINDOW);
        let _ = kill_cmd.output();

        let root_path = format!("{}:\\", instance.drive_letter);
        for _ in 0..10 {
            if !Path::new(&root_path).exists() {
                break;
            }
            thread::sleep(Duration::from_millis(200));
        }

        Ok(true)
    } else {
        Err(format!("找不到已掛載的遠端 '{}'", remote))
    }
}

#[tauri::command]
fn open_in_explorer(drive_letter: String) -> Result<(), String> {
    let mut cmd = Command::new("explorer.exe");
    cmd.arg(format!("{}\\", drive_letter.trim()));
    cmd.spawn().map_err(|e| format!("開啟檔案總管失敗: {}", e))?;
    Ok(())
}

#[tauri::command]
fn unmount_all(state: State<'_, AppState>) -> Result<usize, String> {
    let mut mounts = state.mounts.lock().unwrap();
    let count = mounts.len();
    for (rem, instance) in mounts.drain() {
        remove_windows_drive_icon(&instance.drive_letter, Some(&rem));
        let mut kill_cmd = Command::new("taskkill");
        kill_cmd.args(["/F", "/PID", &instance.pid.to_string(), "/T"]);
        kill_cmd.creation_flags(CREATE_NO_WINDOW);
        let _ = kill_cmd.output();
    }
    refresh_shell_icons();
    Ok(count)
}

// GUI Remote Creation
#[tauri::command]
async fn create_remote_gui(
    rclone_path: Option<String>,
    name: String,
    remote_type: String,
    params: HashMap<String, String>,
) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let exe = resolve_rclone_path(rclone_path);
        let mut cmd = Command::new(&exe);
        cmd.arg("config");
        cmd.arg("create");
        cmd.arg(&name);
        cmd.arg(&remote_type);

        for (k, v) in params {
            if !v.trim().is_empty() {
                cmd.arg(k);
                cmd.arg(v);
            }
        }

        let is_onedrive = remote_type.to_lowercase().contains("onedrive");
        if is_onedrive {
            cmd.stdin(Stdio::piped());
        }

        cmd.creation_flags(CREATE_NO_WINDOW);

        let mut child = cmd.spawn().map_err(|e| format!("執行 rclone config create 失敗: {}", e))?;

        if is_onedrive {
            if let Some(mut stdin) = child.stdin.take() {
                let _ = stdin.write_all(b"1\n0\ny\n");
            }
        }

        let output = child.wait_with_output().map_err(|e| format!("等待建立失敗: {}", e))?;
        if output.status.success() {
            if is_onedrive {
                let _ = auto_heal_onedrive_config(&exe, &name);
            }
            Ok(format!("成功建立遠端 '{}'！", name))
        } else {
            let err = String::from_utf8_lossy(&output.stderr);
            Err(format!("建立失敗: {}", err))
        }
    })
    .await
    .map_err(|e| format!("執行建立失敗: {}", e))?
}

// GUI Remote Config Update & Optional Rename
#[tauri::command]
fn update_remote_gui(
    state: State<'_, AppState>,
    rclone_path: Option<String>,
    name: String,
    new_name: Option<String>,
    params: HashMap<String, String>,
) -> Result<String, String> {
    let exe = resolve_rclone_path(rclone_path);
    let old_name = name.trim().to_string();
    let target_name = new_name
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| old_name.clone());

    // 1. If renaming remote
    if !target_name.is_empty() && target_name != old_name {
        if target_name.contains(':')
            || target_name.contains('/')
            || target_name.contains('\\')
            || target_name.contains('[')
            || target_name.contains(']')
            || target_name.contains('"')
            || target_name.contains('*')
            || target_name.contains('?')
            || target_name.contains('<')
            || target_name.contains('>')
            || target_name.contains('|')
        {
            return Err("雲端硬碟名稱不可包含特殊字元 (: / \\ [ ] * ? < > | \")".to_string());
        }

        let conf_path = get_rclone_conf_path(&exe).ok_or_else(|| "找不到 rclone.conf 設定檔路徑！".to_string())?;
        let content = std::fs::read_to_string(&conf_path).map_err(|e| format!("無法讀取 rclone.conf: {}", e))?;

        let old_header = format!("[{}]", old_name);
        let new_header = format!("[{}]", target_name);

        let mut found_old = false;
        for line in content.lines() {
            let t = line.trim();
            if t.eq_ignore_ascii_case(&new_header) && !t.eq_ignore_ascii_case(&old_header) {
                return Err(format!("雲端硬碟名稱 '{}' 已經存在，請更換其他名稱！", target_name));
            }
            if t.eq_ignore_ascii_case(&old_header) {
                found_old = true;
            }
        }

        if !found_old {
            return Err(format!("找不到原設定名稱 '{}'", old_name));
        }

        let mut new_lines = Vec::new();
        let mut replaced = false;
        for line in content.lines() {
            if !replaced && line.trim().eq_ignore_ascii_case(&old_header) {
                new_lines.push(new_header.clone());
                replaced = true;
            } else {
                new_lines.push(line.to_string());
            }
        }
        let new_content = new_lines.join("\r\n") + "\r\n";
        std::fs::write(&conf_path, new_content).map_err(|e| format!("寫入 rclone.conf 失敗: {}", e))?;

        // Update active mounts tracking if currently mounted
        {
            let mut mounts = state.mounts.lock().unwrap();
            if let Some(mut instance) = mounts.remove(&old_name) {
                instance.remote = target_name.clone();
                let drive = instance.drive_letter.clone();
                let rtype = instance.remote_type.clone().unwrap_or_else(|| "default".to_string());
                mounts.insert(target_name.clone(), instance);
                set_windows_drive_icon(&drive, &rtype, Some(&target_name));
            }
        }
    }

    // 2. If additional parameters (url, user, pass, etc.) need to be updated
    let active_name = if target_name != old_name { target_name.clone() } else { old_name.clone() };
    let has_params = params.values().any(|v| !v.trim().is_empty());
    if has_params {
        let mut cmd = Command::new(&exe);
        cmd.arg("config");
        cmd.arg("update");
        cmd.arg(&active_name);

        for (k, v) in params {
            if !v.trim().is_empty() {
                cmd.arg(k);
                cmd.arg(v);
            }
        }

        cmd.creation_flags(CREATE_NO_WINDOW);

        let output = cmd.output().map_err(|e| format!("執行 rclone config update 失敗: {}", e))?;
        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            return Err(format!("更新失敗: {}", err));
        }
    }

    if target_name != old_name {
        Ok(format!("成功將雲端硬碟名稱更名為 '{}' 並儲存設定！", target_name))
    } else {
        Ok(format!("成功更新遠端 '{}' 的設定！", active_name))
    }
}

// GUI Remote Reconnect (OAuth Refresh via Browser)
#[tauri::command]
async fn reconnect_remote_gui(
    rclone_path: Option<String>,
    name: String,
) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let exe = resolve_rclone_path(rclone_path);
        let mut cmd = Command::new(&exe);
        cmd.args(["config", "reconnect", &format!("{}:", name)]);
        cmd.stdin(Stdio::piped());
        cmd.creation_flags(CREATE_NO_WINDOW);

        let mut child = cmd.spawn().map_err(|e| format!("執行重新連線失敗: {}", e))?;
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(b"1\n0\ny\ny\n");
        }

        let output = child.wait_with_output().map_err(|e| format!("等待重新授權失敗: {}", e))?;
        if output.status.success() {
            let _ = auto_heal_onedrive_config(&exe, &name);
            Ok(format!("遠端 '{}' 重新授權登入成功！", name))
        } else {
            let err = String::from_utf8_lossy(&output.stderr);
            Err(format!("重新授權失敗: {}", err))
        }
    })
    .await
    .map_err(|e| format!("執行重新授權失敗: {}", e))?
}

// Get Single Remote Configuration Detail
#[tauri::command]
fn get_remote_detail(
    rclone_path: Option<String>,
    name: String,
) -> Result<HashMap<String, String>, String> {
    let exe = resolve_rclone_path(rclone_path);
    let mut cmd = Command::new(&exe);
    cmd.args(["config", "dump"]);
    cmd.creation_flags(CREATE_NO_WINDOW);

    let output = cmd.output().map_err(|e| format!("讀取設定失敗: {}", e))?;
    let json_val: serde_json::Value = serde_json::from_slice(&output.stdout)
        .map_err(|e| format!("解析設定 JSON 失敗: {}", e))?;

    let mut result = HashMap::new();
    if let Some(remote_obj) = json_val.get(&name).and_then(|v| v.as_object()) {
        for (k, v) in remote_obj {
            if k != "token" {
                if let Some(s) = v.as_str() {
                    result.insert(k.clone(), s.to_string());
                }
            }
        }
    }
    Ok(result)
}

// Delete Remote
#[tauri::command]
fn delete_remote_gui(
    state: State<'_, AppState>,
    rclone_path: Option<String>,
    name: String,
) -> Result<bool, String> {
    let _ = unmount_remote(state, name.clone());

    let exe = resolve_rclone_path(rclone_path);
    let mut cmd = Command::new(&exe);
    cmd.args(["config", "delete", &name]);
    cmd.creation_flags(CREATE_NO_WINDOW);

    let output = cmd.output().map_err(|e| format!("刪除遠端失敗: {}", e))?;
    if output.status.success() {
        Ok(true)
    } else {
        let err = String::from_utf8_lossy(&output.stderr);
        Err(format!("刪除失敗: {}", err))
    }
}

// Start / Stop Rclone Official Web GUI
#[tauri::command]
fn toggle_rclone_webgui(
    state: State<'_, AppState>,
    rclone_path: Option<String>,
    enable: bool,
) -> Result<bool, String> {
    let mut gui_guard = state.webgui_child.lock().unwrap();

    if enable {
        if gui_guard.is_some() {
            return Ok(true);
        }
        let exe = resolve_rclone_path(rclone_path);
        let mut cmd = Command::new(&exe);
        cmd.args([
            "rcd",
            "--rc-web-gui",
            "--rc-no-auth",
            "--rc-addr",
            "127.0.0.1:5572",
            "--rc-web-gui-no-open-browser",
        ]);
        cmd.creation_flags(CREATE_NO_WINDOW);

        let child = cmd.spawn().map_err(|e| format!("啟動 Web-GUI 失敗: {}", e))?;
        *gui_guard = Some(child.id());
        Ok(true)
    } else {
        if let Some(pid) = gui_guard.take() {
            let mut kill_cmd = Command::new("taskkill");
            kill_cmd.args(["/F", "/PID", &pid.to_string(), "/T"]);
            kill_cmd.creation_flags(CREATE_NO_WINDOW);
            let _ = kill_cmd.output();
        }
        Ok(false)
    }
}

#[tauri::command]
fn open_browser_url(url: String) -> Result<(), String> {
    let mut cmd = Command::new("cmd.exe");
    cmd.args(["/c", "start", "", &url]);
    cmd.creation_flags(CREATE_NO_WINDOW);
    let _ = cmd.spawn().map_err(|e| format!("無法開啟預設瀏覽器: {}", e))?;
    Ok(())
}

// Native Local Folder Browser Dialog (Windows Forms STA)
#[tauri::command]
fn select_local_folder() -> Result<Option<String>, String> {
    let script = r#"
        [System.Reflection.Assembly]::LoadWithPartialName('System.Windows.Forms') | Out-Null
        $dialog = New-Object System.Windows.Forms.FolderBrowserDialog
        $dialog.Description = "請選擇資料夾"
        $dialog.ShowNewFolderButton = $true
        if ($dialog.ShowDialog() -eq [System.Windows.Forms.DialogResult]::OK) {
            [Console]::OutputEncoding = [System.Text.Encoding]::UTF8
            [Console]::Out.Write($dialog.SelectedPath)
        }
    "#;

    let output = Command::new("powershell")
        .args(["-STA", "-NoProfile", "-Command", script])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| format!("開啟資料夾瀏覽視窗失敗: {}", e))?;

    let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if path_str.is_empty() {
        Ok(None)
    } else {
        Ok(Some(path_str))
    }
}

// List directories inside a cloud remote (e.g. GDrive: or GDrive:folder)
#[tauri::command]
fn list_remote_dirs(
    rclone_path: Option<String>,
    remote_path: String,
) -> Result<Vec<String>, String> {
    let exe = resolve_rclone_path(rclone_path);
    let mut cmd = Command::new(&exe);
    cmd.args(["lsf", &remote_path, "--dirs-only"]);
    cmd.creation_flags(CREATE_NO_WINDOW);

    let output = cmd.output().map_err(|e| format!("讀取雲端目錄失敗: {}", e))?;
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(format!("列出目錄失敗: {}", err));
    }

    let out_str = String::from_utf8_lossy(&output.stdout);
    let dirs: Vec<String> = out_str
        .lines()
        .map(|l| l.trim().trim_end_matches('/').to_string())
        .filter(|l| !l.is_empty())
        .collect();

    Ok(dirs)
}

fn format_file_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

fn format_system_time(st: std::time::SystemTime) -> String {
    if let Ok(dur) = st.duration_since(std::time::UNIX_EPOCH) {
        let secs = dur.as_secs();
        let local_secs = secs + 8 * 3600;
        let days = local_secs / 86400;
        let day_secs = local_secs % 86400;
        let hours = day_secs / 3600;
        let mins = (day_secs % 3600) / 60;

        let mut y = 1970;
        let mut d = days;
        loop {
            let leap = (y % 4 == 0 && y % 100 != 0) || (y % 400 == 0);
            let y_days = if leap { 366 } else { 365 };
            if d < y_days {
                break;
            }
            d -= y_days;
            y += 1;
        }
        let leap = (y % 4 == 0 && y % 100 != 0) || (y % 400 == 0);
        let m_days = [
            31, if leap { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31
        ];
        let mut m = 1;
        for &dim in &m_days {
            if d < dim {
                break;
            }
            d -= dim;
            m += 1;
        }
        let day = d + 1;
        format!("{:04}/{:02}/{:02} {:02}:{:02}", y, m, day, hours, mins)
    } else {
        "-".to_string()
    }
}

fn query_file_meta(root: &str, rel: &str) -> (Option<String>, Option<String>) {
    let p = Path::new(root).join(rel.replace('/', "\\"));
    if let Ok(meta) = std::fs::metadata(&p) {
        let size = format_file_size(meta.len());
        let mtime = meta.modified().map(format_system_time).unwrap_or_else(|_| "-".to_string());
        (Some(size), Some(mtime))
    } else {
        (None, None)
    }
}

// RcloneView Plus Compare / Check Diff
#[tauri::command]
fn check_folder_diff(
    rclone_path: Option<String>,
    source: String,
    dest: String,
    exclude_patterns: Option<Vec<String>>,
) -> Result<DiffSummary, String> {
    let exe = resolve_rclone_path(rclone_path);
    let mut cmd = Command::new(&exe);
    cmd.args(["check", &source, &dest, "--combined", "-"]);

    if let Some(excludes) = exclude_patterns {
        for pattern in excludes {
            let p = pattern.trim();
            if !p.is_empty() {
                cmd.args(["--exclude", p]);
            }
        }
    }

    cmd.creation_flags(CREATE_NO_WINDOW);

    let output = cmd.output().map_err(|e| format!("執行比對失敗: {}", e))?;
    let out_str = String::from_utf8_lossy(&output.stdout);

    let mut items = Vec::new();
    let mut diffs = Vec::new();
    let mut total_s = 0; // source only (+)
    let mut total_d = 0; // dest only (-)
    let mut total_diff = 0; // different (*)
    let mut total_eq = 0; // equal (=)
    let mut total_err = 0; // error (!)

    for line in out_str.lines() {
        let trimmed = line.trim_end();
        if trimmed.len() < 2 {
            continue;
        }
        let first_char = trimmed.chars().next().unwrap();
        let path = trimmed[1..].trim_start().to_string();

        let (src_size, src_mtime) = query_file_meta(&source, &path);
        let (dst_size, dst_mtime) = query_file_meta(&dest, &path);

        match first_char {
            '+' => {
                total_s += 1;
                diffs.push(trimmed.to_string());
                items.push(DiffItem {
                    path,
                    status: "source_only".to_string(),
                    symbol: "+".to_string(),
                    size_src: src_size,
                    mtime_src: src_mtime,
                    size_dest: None,
                    mtime_dest: None,
                });
            }
            '-' => {
                total_d += 1;
                diffs.push(trimmed.to_string());
                items.push(DiffItem {
                    path,
                    status: "dest_only".to_string(),
                    symbol: "-".to_string(),
                    size_src: None,
                    mtime_src: None,
                    size_dest: dst_size,
                    mtime_dest: dst_mtime,
                });
            }
            '*' => {
                total_diff += 1;
                diffs.push(trimmed.to_string());
                items.push(DiffItem {
                    path,
                    status: "different".to_string(),
                    symbol: "*".to_string(),
                    size_src: src_size,
                    mtime_src: src_mtime,
                    size_dest: dst_size,
                    mtime_dest: dst_mtime,
                });
            }
            '=' => {
                total_eq += 1;
                items.push(DiffItem {
                    path,
                    status: "equal".to_string(),
                    symbol: "=".to_string(),
                    size_src: src_size.clone(),
                    mtime_src: src_mtime.clone(),
                    size_dest: dst_size.or(src_size),
                    mtime_dest: dst_mtime.or(src_mtime),
                });
            }
            '!' => {
                total_err += 1;
                diffs.push(trimmed.to_string());
                items.push(DiffItem {
                    path,
                    status: "error".to_string(),
                    symbol: "!".to_string(),
                    size_src: src_size,
                    mtime_src: src_mtime,
                    size_dest: dst_size,
                    mtime_dest: dst_mtime,
                });
            }
            _ => {}
        }
    }

    let message = if total_s == 0 && total_d == 0 && total_diff == 0 && total_err == 0 {
        "兩端檔案完全一致，無任何差異！".to_string()
    } else {
        format!(
            "比對完成：來源待同步 {} 檔，目的端多出 {} 檔，內容差異 {} 檔，一致 {} 檔",
            total_s, total_d, total_diff, total_eq
        )
    };

    Ok(DiffSummary {
        total_source_files: total_s,
        total_dest_files: total_d,
        total_different: total_diff,
        total_equal: total_eq,
        total_error: total_err,
        differences: diffs.into_iter().take(200).collect(),
        items,
        message,
    })
}

// Copy Specific Selected Files
#[tauri::command]
fn copy_specific_files(
    rclone_path: Option<String>,
    source: String,
    dest: String,
    files: Vec<String>,
) -> Result<String, String> {
    if files.is_empty() {
        return Err("請先勾選要複製的檔案項目！".to_string());
    }

    let exe = resolve_rclone_path(rclone_path);
    let mut cmd = Command::new(&exe);
    cmd.arg("copy");
    cmd.arg(&source);
    cmd.arg(&dest);

    for file in files {
        let f = file.replace('\\', "/");
        cmd.args(["--include", &format!("/{}", f.trim_start_matches('/'))]);
    }
    cmd.arg("-v");
    cmd.creation_flags(CREATE_NO_WINDOW);

    let output = cmd.output().map_err(|e| format!("執行複製失敗: {}", e))?;
    if output.status.success() {
        let err_log = String::from_utf8_lossy(&output.stderr);
        let out_log = String::from_utf8_lossy(&output.stdout);
        let combined = format!("{}\n{}", out_log.trim(), err_log.trim());
        Ok(if combined.trim().is_empty() { "檔案已成功複製完成！".to_string() } else { combined.trim().to_string() })
    } else {
        let err = String::from_utf8_lossy(&output.stderr);
        Err(format!("複製失敗: {}", err))
    }
}

// RcloneView Plus Run Sync Job
#[tauri::command]
fn run_sync_task(
    rclone_path: Option<String>,
    action: String,
    source: String,
    dest: String,
    exclude_patterns: Option<Vec<String>>,
) -> Result<String, String> {
    let exe = resolve_rclone_path(rclone_path);
    let mut cmd = Command::new(&exe);
    let act = match action.as_str() {
        "copy" => "copy",
        "move" => "move",
        _ => "sync",
    };
    cmd.arg(act);
    cmd.arg(&source);
    cmd.arg(&dest);

    if let Some(excludes) = exclude_patterns {
        for pattern in excludes {
            let p = pattern.trim();
            if !p.is_empty() {
                cmd.args(["--exclude", p]);
            }
        }
    }

    cmd.arg("-v");
    cmd.creation_flags(CREATE_NO_WINDOW);

    let output = cmd.output().map_err(|e| format!("執行同步/備份任務失敗: {}", e))?;
    if output.status.success() {
        let err_log = String::from_utf8_lossy(&output.stderr);
        let out_log = String::from_utf8_lossy(&output.stdout);
        let combined = format!("{}\n{}", out_log.trim(), err_log.trim());
        Ok(if combined.trim().is_empty() { "任務執行成功完成！".to_string() } else { combined.trim().to_string() })
    } else {
        let err = String::from_utf8_lossy(&output.stderr);
        Err(format!("任務失敗: {}", err))
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(AppState::default())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--minimized"]),
        ))
        .setup(|app| {
            let args: Vec<String> = std::env::args().collect();
            let is_minimized_arg = args.iter().any(|a| a == "--minimized" || a == "-m");
            let settings = load_app_settings();

            let should_minimize = is_minimized_arg && settings.start_minimized_to_tray;

            // Ensure drive icons directory exists
            let _ = ensure_drive_icons();

            let quit_i = MenuItemBuilder::with_id("quit", "結束 (Quit)").build(app)?;
            let show_i = MenuItemBuilder::with_id("show", "開啟儀表板 (Show Dashboard)").build(app)?;
            let webgui_i = MenuItemBuilder::with_id("webgui", "開啟 Rclone Web-GUI (瀏覽器)").build(app)?;
            let menu = MenuBuilder::new(app)
                .items(&[&show_i, &webgui_i, &quit_i])
                .build()?;

            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .tooltip("Rclone Drive - 雲端硬碟掛載與同步系統")
                .on_menu_event(|app, event| {
                    match event.id().as_ref() {
                        "show" => {
                            if let Some(win) = app.get_webview_window("main") {
                                let _ = win.show();
                                let _ = win.set_focus();
                            }
                        }
                        "webgui" => {
                            let _ = open_browser_url("http://127.0.0.1:5572/".to_string());
                        }
                        "quit" => {
                            let state = app.state::<AppState>();
                            let _ = unmount_all(state);
                            app.exit(0);
                        }
                        _ => {}
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(win) = app.get_webview_window("main") {
                            let _ = win.show();
                            let _ = win.set_focus();
                        }
                    }
                })
                .build(app)?;

            if let Some(win) = app.get_webview_window("main") {
                let win_clone = win.clone();
                win.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = win_clone.hide();
                    }
                });

                if should_minimize {
                    let _ = win.hide();
                } else {
                    let _ = win.show();
                    let _ = win.set_focus();
                }
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            check_environment,
            get_available_drives,
            get_remotes,
            mount_remote,
            unmount_remote,
            open_in_explorer,
            unmount_all,
            create_remote_gui,
            update_remote_gui,
            reconnect_remote_gui,
            get_remote_detail,
            delete_remote_gui,
            toggle_rclone_webgui,
            open_browser_url,
            check_folder_diff,
            run_sync_task,
            select_local_folder,
            list_remote_dirs,
            copy_specific_files,
            get_app_settings,
            save_app_settings,
            apply_mounted_drive_icons,
            auto_install_rclone,
            auto_install_winfsp,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
