use std::collections::HashMap;
use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::Command;
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffSummary {
    pub total_source_files: usize,
    pub total_dest_files: usize,
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
    "rclone".to_string()
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

    let winfsp_paths = [
        "C:\\Program Files (x86)\\WinFsp",
        "C:\\Program Files\\WinFsp",
    ];
    for p in winfsp_paths {
        if Path::new(p).exists() {
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

#[tauri::command]
fn mount_remote(
    state: State<'_, AppState>,
    rclone_path: Option<String>,
    remote: String,
    drive_letter: String,
    volname: Option<String>,
    cache_mode: Option<String>,
    read_only: Option<bool>,
) -> Result<MountInstance, String> {
    let exe = resolve_rclone_path(rclone_path);

    {
        let mounts = state.mounts.lock().unwrap();
        if mounts.contains_key(&remote) {
            return Err(format!("遠端 '{}' 已經掛載中！", remote));
        }
    }

    let clean_letter = drive_letter.trim().to_uppercase();
    let root_path = format!("{clean_letter}\\");
    if Path::new(&root_path).exists() {
        return Err(format!("磁碟機代號 {} 已被本機其他裝置佔用！", clean_letter));
    }

    let v_name = volname.unwrap_or_else(|| remote.clone());
    let c_mode = cache_mode.unwrap_or_else(|| "full".to_string());

    let mut cmd = Command::new(&exe);
    cmd.arg("mount");
    cmd.arg(format!("{}:", remote));
    cmd.arg(&clean_letter);
    cmd.arg("--vfs-cache-mode");
    cmd.arg(&c_mode);
    cmd.arg("--volname");
    cmd.arg(&v_name);
    cmd.arg("--network-mode"); // Always mount as Windows network drive for 100% stability without Admin UAC!

    if read_only.unwrap_or(false) {
        cmd.arg("--read-only");
    }

    cmd.creation_flags(CREATE_NO_WINDOW);

    let mut child = cmd.spawn().map_err(|e| format!("啟動 rclone 掛載行程失敗: {}", e))?;
    let pid = child.id();

    let mut mounted = false;
    for _ in 0..30 {
        thread::sleep(Duration::from_millis(300));
        if let Ok(Some(status)) = child.try_wait() {
            return Err(format!("掛載失敗: rclone 行程已退出 (狀態碼: {:?})。請確認代號是否衝突或設定是否有效。", status.code()));
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
        return Err(format!("掛載逾時，磁碟機 {} 未能及時就緒。請確認 WinFsp 正常運行。", clean_letter));
    }

    let instance = MountInstance {
        remote: remote.clone(),
        drive_letter: clean_letter,
        pid,
    };

    let mut mounts = state.mounts.lock().unwrap();
    mounts.insert(remote, instance.clone());

    Ok(instance)
}

#[tauri::command]
fn unmount_remote(state: State<'_, AppState>, remote: String) -> Result<bool, String> {
    let mut mounts = state.mounts.lock().unwrap();
    if let Some(instance) = mounts.remove(&remote) {
        let mut kill_cmd = Command::new("taskkill");
        kill_cmd.args(["/F", "/PID", &instance.pid.to_string(), "/T"]);
        kill_cmd.creation_flags(CREATE_NO_WINDOW);
        let _ = kill_cmd.output();

        let root_path = format!("{}\\", instance.drive_letter);
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
    for (_, instance) in mounts.drain() {
        let mut kill_cmd = Command::new("taskkill");
        kill_cmd.args(["/F", "/PID", &instance.pid.to_string(), "/T"]);
        kill_cmd.creation_flags(CREATE_NO_WINDOW);
        let _ = kill_cmd.output();
    }
    Ok(count)
}

// GUI Remote Creation
#[tauri::command]
fn create_remote_gui(
    rclone_path: Option<String>,
    name: String,
    remote_type: String,
    params: HashMap<String, String>,
) -> Result<String, String> {
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

    cmd.creation_flags(CREATE_NO_WINDOW);

    let output = cmd.output().map_err(|e| format!("執行 rclone config create 失敗: {}", e))?;
    if output.status.success() {
        Ok(format!("成功建立遠端 '{}'！", name))
    } else {
        let err = String::from_utf8_lossy(&output.stderr);
        Err(format!("建立失敗: {}", err))
    }
}

// GUI Remote Config Update
#[tauri::command]
fn update_remote_gui(
    rclone_path: Option<String>,
    name: String,
    params: HashMap<String, String>,
) -> Result<String, String> {
    let exe = resolve_rclone_path(rclone_path);
    let mut cmd = Command::new(&exe);
    cmd.arg("config");
    cmd.arg("update");
    cmd.arg(&name);

    for (k, v) in params {
        if !v.trim().is_empty() {
            cmd.arg(k);
            cmd.arg(v);
        }
    }

    cmd.creation_flags(CREATE_NO_WINDOW);

    let output = cmd.output().map_err(|e| format!("執行 rclone config update 失敗: {}", e))?;
    if output.status.success() {
        Ok(format!("成功更新遠端 '{}' 的設定！", name))
    } else {
        let err = String::from_utf8_lossy(&output.stderr);
        Err(format!("更新失敗: {}", err))
    }
}

// GUI Remote Reconnect (OAuth Refresh via Browser)
#[tauri::command]
fn reconnect_remote_gui(
    rclone_path: Option<String>,
    name: String,
) -> Result<String, String> {
    let exe = resolve_rclone_path(rclone_path);
    let mut cmd = Command::new(&exe);
    cmd.args(["config", "reconnect", &format!("{}:", name)]);
    cmd.creation_flags(CREATE_NO_WINDOW);

    let output = cmd.output().map_err(|e| format!("執行重新連線失敗: {}", e))?;
    if output.status.success() {
        Ok(format!("遠端 '{}' 重新授權登入成功！", name))
    } else {
        let err = String::from_utf8_lossy(&output.stderr);
        Err(format!("重新授權失敗: {}", err))
    }
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

// RcloneView Plus Compare / Check Diff
#[tauri::command]
fn check_folder_diff(
    rclone_path: Option<String>,
    source: String,
    dest: String,
) -> Result<DiffSummary, String> {
    let exe = resolve_rclone_path(rclone_path);
    let mut cmd = Command::new(&exe);
    cmd.args(["check", &source, &dest, "--one-way", "--combined", "-"]);
    cmd.creation_flags(CREATE_NO_WINDOW);

    let output = cmd.output().map_err(|e| format!("執行比對失敗: {}", e))?;
    let out_str = String::from_utf8_lossy(&output.stdout);

    let mut diffs = Vec::new();
    let mut total_s = 0;
    let mut total_d = 0;

    for line in out_str.lines() {
        if line.starts_with('+') || line.starts_with('-') || line.starts_with('*') || line.starts_with('!') {
            diffs.push(line.to_string());
            if line.starts_with('+') {
                total_d += 1;
            } else if line.starts_with('-') {
                total_s += 1;
            }
        }
    }

    Ok(DiffSummary {
        total_source_files: total_s,
        total_dest_files: total_d,
        differences: diffs.into_iter().take(100).collect(),
        message: if output.status.success() {
            "兩端檔案完全一致，無任何差異！".to_string()
        } else {
            "偵測到檔案差異。".to_string()
        },
    })
}

// RcloneView Plus Run Sync Job
#[tauri::command]
fn run_sync_task(
    rclone_path: Option<String>,
    action: String,
    source: String,
    dest: String,
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
    cmd.arg("-v");
    cmd.creation_flags(CREATE_NO_WINDOW);

    let output = cmd.output().map_err(|e| format!("執行同步/備份任務失敗: {}", e))?;
    if output.status.success() {
        let err_log = String::from_utf8_lossy(&output.stderr);
        Ok(format!("任務完成！\n{}", err_log.lines().rev().take(5).collect::<Vec<_>>().join("\n")))
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
