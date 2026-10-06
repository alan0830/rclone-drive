use serde::{Deserialize, Serialize};
use std::net::TcpListener;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::Duration;

const CREATE_NO_WINDOW: u32 = 0x08000000;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MountPerformanceOptions {
    #[serde(alias = "preset_mode", alias = "preset")]
    pub preset: Option<String>, // "fast" | "default" | "custom"

    // 1. 目錄與屬性快取
    #[serde(alias = "dir_cache_time")]
    pub dir_cache_time: Option<String>,
    #[serde(alias = "poll_interval")]
    pub poll_interval: Option<String>,
    #[serde(alias = "attr_timeout")]
    pub attr_timeout: Option<String>,
    #[serde(alias = "no_modtime")]
    pub no_modtime: Option<bool>,

    // 2. VFS 磁碟快取與讀取切片
    #[serde(alias = "vfs_cache_mode")]
    pub vfs_cache_mode: Option<String>,
    #[serde(alias = "vfs_cache_max_size")]
    pub vfs_cache_max_size: Option<String>,
    #[serde(alias = "vfs_cache_max_age")]
    pub vfs_cache_max_age: Option<String>,
    #[serde(alias = "vfs_read_chunk_size")]
    pub vfs_read_chunk_size: Option<String>,
    #[serde(alias = "vfs_read_chunk_size_limit")]
    pub vfs_read_chunk_size_limit: Option<String>,

    // Google Drive Pacer
    #[serde(alias = "drive_pacer_min_sleep")]
    pub drive_pacer_min_sleep: Option<String>,
    #[serde(alias = "drive_pacer_burst")]
    pub drive_pacer_burst: Option<i32>,
    #[serde(alias = "enable_drive_pacer")]
    pub enable_drive_pacer: Option<bool>,

    // 3. 背景預熱 (Warm-up)
    #[serde(alias = "warm_up")]
    pub warm_up: Option<bool>,

    // 4. 自訂額外參數
    #[serde(alias = "extra_flags")]
    pub extra_flags: Option<String>,
}

impl Default for MountPerformanceOptions {
    fn default() -> Self {
        Self::fast_preset()
    }
}

impl MountPerformanceOptions {
    /// 官方推薦最佳化預設 (Fast Preset)
    pub fn fast_preset() -> Self {
        Self {
            preset: Some("fast".to_string()),
            dir_cache_time: Some("72h".to_string()),
            poll_interval: Some("1m".to_string()),
            attr_timeout: Some("10m".to_string()),
            no_modtime: Some(true),
            vfs_cache_mode: Some("full".to_string()),
            vfs_cache_max_size: Some("50G".to_string()),
            vfs_cache_max_age: Some("24h".to_string()),
            vfs_read_chunk_size: Some("64M".to_string()),
            vfs_read_chunk_size_limit: Some("1G".to_string()),
            drive_pacer_min_sleep: Some("10ms".to_string()),
            drive_pacer_burst: Some(200),
            enable_drive_pacer: Some(true),
            warm_up: Some(true),
            extra_flags: Some("".to_string()),
        }
    }

    /// Rclone 官方標準原生預設 (Standard / Default Preset)
    pub fn default_preset() -> Self {
        Self {
            preset: Some("default".to_string()),
            dir_cache_time: Some("5m".to_string()),
            poll_interval: Some("1m".to_string()),
            attr_timeout: Some("1s".to_string()),
            no_modtime: Some(false),
            vfs_cache_mode: Some("full".to_string()),
            vfs_cache_max_size: Some("off".to_string()),
            vfs_cache_max_age: Some("1h".to_string()),
            vfs_read_chunk_size: Some("128M".to_string()),
            vfs_read_chunk_size_limit: Some("off".to_string()),
            drive_pacer_min_sleep: Some("100ms".to_string()),
            drive_pacer_burst: Some(100),
            enable_drive_pacer: Some(false),
            warm_up: Some(false),
            extra_flags: Some("".to_string()),
        }
    }

    /// 從檔案載入設定檔 (rclone_mount_config.json)
    pub fn load_from_config_file() -> Option<Self> {
        let candidate_paths = [
            PathBuf::from("rclone_mount_config.json"),
            std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|p| p.join("rclone_mount_config.json")))
                .unwrap_or_else(|| PathBuf::from("")),
            if let Ok(appdata) = std::env::var("APPDATA") {
                Path::new(&appdata).join("com.rclonedrive.desktop").join("rclone_mount_config.json")
            } else {
                PathBuf::from("")
            },
        ];

        for path in &candidate_paths {
            if path.exists() {
                if let Ok(content) = std::fs::read_to_string(path) {
                    if let Ok(opts) = serde_json::from_str::<MountPerformanceOptions>(&content) {
                        return Some(opts);
                    }
                }
            }
        }
        None
    }

    /// 依優先順序合併：傳入選項 -> 系統環境變數 -> 設定檔 -> 推薦預設值
    pub fn resolve(passed: Option<MountPerformanceOptions>, fallback_cache_mode: Option<String>) -> Self {
        let base = match passed.as_ref().and_then(|p| p.preset.as_deref()) {
            Some("default") => Self::default_preset(),
            Some("fast") => Self::fast_preset(),
            _ => {
                // 若沒有指定特定 preset，先檢查外部設定檔或預設 fast
                Self::load_from_config_file().unwrap_or_else(Self::fast_preset)
            }
        };

        let mut final_opts = base;

        // 1. 如果傳入了明確的選項欄位，逐項覆寫
        if let Some(p) = passed {
            if let Some(v) = p.preset { final_opts.preset = Some(v); }
            if let Some(v) = p.dir_cache_time { final_opts.dir_cache_time = Some(v); }
            if let Some(v) = p.poll_interval { final_opts.poll_interval = Some(v); }
            if let Some(v) = p.attr_timeout { final_opts.attr_timeout = Some(v); }
            if let Some(v) = p.no_modtime { final_opts.no_modtime = Some(v); }
            if let Some(v) = p.vfs_cache_mode { final_opts.vfs_cache_mode = Some(v); }
            if let Some(v) = p.vfs_cache_max_size { final_opts.vfs_cache_max_size = Some(v); }
            if let Some(v) = p.vfs_cache_max_age { final_opts.vfs_cache_max_age = Some(v); }
            if let Some(v) = p.vfs_read_chunk_size { final_opts.vfs_read_chunk_size = Some(v); }
            if let Some(v) = p.vfs_read_chunk_size_limit { final_opts.vfs_read_chunk_size_limit = Some(v); }
            if let Some(v) = p.drive_pacer_min_sleep { final_opts.drive_pacer_min_sleep = Some(v); }
            if let Some(v) = p.drive_pacer_burst { final_opts.drive_pacer_burst = Some(v); }
            if let Some(v) = p.enable_drive_pacer { final_opts.enable_drive_pacer = Some(v); }
            if let Some(v) = p.warm_up { final_opts.warm_up = Some(v); }
            if let Some(v) = p.extra_flags { final_opts.extra_flags = Some(v); }
        }

        // 2. 如果呼叫端傳入傳統 cache_mode 參數，且使用者沒特別在 options 指定
        if let Some(cm) = fallback_cache_mode {
            if final_opts.vfs_cache_mode.is_none() {
                final_opts.vfs_cache_mode = Some(cm);
            }
        }

        // 3. 環境變數覆寫 (最高全域覆寫層級)
        if let Ok(val) = std::env::var("RCLONE_MOUNT_PRESET") {
            if !val.trim().is_empty() { final_opts.preset = Some(val); }
        }
        if let Ok(val) = std::env::var("RCLONE_DIR_CACHE_TIME") {
            if !val.trim().is_empty() { final_opts.dir_cache_time = Some(val); }
        }
        if let Ok(val) = std::env::var("RCLONE_POLL_INTERVAL") {
            if !val.trim().is_empty() { final_opts.poll_interval = Some(val); }
        }
        if let Ok(val) = std::env::var("RCLONE_ATTR_TIMEOUT") {
            if !val.trim().is_empty() { final_opts.attr_timeout = Some(val); }
        }
        if let Ok(val) = std::env::var("RCLONE_NO_MODTIME") {
            final_opts.no_modtime = Some(parse_env_bool(&val, true));
        }
        if let Ok(val) = std::env::var("RCLONE_VFS_CACHE_MODE") {
            if !val.trim().is_empty() { final_opts.vfs_cache_mode = Some(val); }
        }
        if let Ok(val) = std::env::var("RCLONE_VFS_CACHE_MAX_SIZE") {
            if !val.trim().is_empty() { final_opts.vfs_cache_max_size = Some(val); }
        }
        if let Ok(val) = std::env::var("RCLONE_VFS_CACHE_MAX_AGE") {
            if !val.trim().is_empty() { final_opts.vfs_cache_max_age = Some(val); }
        }
        if let Ok(val) = std::env::var("RCLONE_VFS_READ_CHUNK_SIZE") {
            if !val.trim().is_empty() { final_opts.vfs_read_chunk_size = Some(val); }
        }
        if let Ok(val) = std::env::var("RCLONE_VFS_READ_CHUNK_SIZE_LIMIT") {
            if !val.trim().is_empty() { final_opts.vfs_read_chunk_size_limit = Some(val); }
        }
        if let Ok(val) = std::env::var("RCLONE_DRIVE_PACER_MIN_SLEEP") {
            if !val.trim().is_empty() { final_opts.drive_pacer_min_sleep = Some(val); }
        }
        if let Ok(val) = std::env::var("RCLONE_DRIVE_PACER_BURST") {
            if let Ok(b) = val.trim().parse::<i32>() { final_opts.drive_pacer_burst = Some(b); }
        }
        if let Ok(val) = std::env::var("RCLONE_WARM_UP") {
            final_opts.warm_up = Some(parse_env_bool(&val, true));
        }
        if let Ok(val) = std::env::var("RCLONE_EXTRA_FLAGS") {
            if !val.trim().is_empty() { final_opts.extra_flags = Some(val); }
        }

        final_opts
    }

    /// 將所有最佳化旗標加到 rclone mount 命令，並在需要 warm-up 時配置專屬 RC port
    pub fn apply_to_command(&self, cmd: &mut Command, remote_type: Option<&str>) -> (bool, Option<u16>) {
        // 1. 目錄與屬性快取
        if let Some(ref val) = self.dir_cache_time {
            if !val.trim().is_empty() {
                cmd.arg("--dir-cache-time");
                cmd.arg(val.trim());
            }
        }
        if let Some(ref val) = self.poll_interval {
            if !val.trim().is_empty() {
                cmd.arg("--poll-interval");
                cmd.arg(val.trim());
            }
        }
        if let Some(ref val) = self.attr_timeout {
            if !val.trim().is_empty() {
                cmd.arg("--attr-timeout");
                cmd.arg(val.trim());
            }
        }
        if self.no_modtime.unwrap_or(false) {
            cmd.arg("--no-modtime");
        }

        // 2. VFS 磁碟快取與切片
        if let Some(ref val) = self.vfs_cache_max_size {
            if !val.trim().is_empty() {
                cmd.arg("--vfs-cache-max-size");
                cmd.arg(val.trim());
            }
        }
        if let Some(ref val) = self.vfs_cache_max_age {
            if !val.trim().is_empty() {
                cmd.arg("--vfs-cache-max-age");
                cmd.arg(val.trim());
            }
        }
        if let Some(ref val) = self.vfs_read_chunk_size {
            if !val.trim().is_empty() {
                cmd.arg("--vfs-read-chunk-size");
                cmd.arg(val.trim());
            }
        }
        if let Some(ref val) = self.vfs_read_chunk_size_limit {
            if !val.trim().is_empty() {
                cmd.arg("--vfs-read-chunk-size-limit");
                cmd.arg(val.trim());
            }
        }

        // 3. Google Drive Pacer
        let is_gdrive = remote_type
            .map(|t| t.eq_ignore_ascii_case("drive"))
            .unwrap_or(false);
        if is_gdrive || self.enable_drive_pacer.unwrap_or(false) {
            if let Some(ref val) = self.drive_pacer_min_sleep {
                if !val.trim().is_empty() {
                    cmd.arg("--drive-pacer-min-sleep");
                    cmd.arg(val.trim());
                }
            }
            if let Some(burst) = self.drive_pacer_burst {
                if burst > 0 {
                    cmd.arg("--drive-pacer-burst");
                    cmd.arg(burst.to_string());
                }
            }
        }

        // 4. 背景預熱 RC API 支援
        let do_warmup = self.warm_up.unwrap_or(true);
        let rc_port = if do_warmup {
            let port = get_available_port().unwrap_or(5575);
            cmd.arg("--rc");
            cmd.arg("--rc-addr");
            cmd.arg(format!("127.0.0.1:{}", port));
            cmd.arg("--rc-no-auth");
            Some(port)
        } else {
            None
        };

        // 5. 額外使用者 Flags
        if let Some(ref extra) = self.extra_flags {
            for arg in split_shell_args(extra) {
                if !arg.trim().is_empty() {
                    cmd.arg(arg.trim());
                }
            }
        }

        (do_warmup, rc_port)
    }

    /// 在背景執行緒非阻塞執行快取預熱 (vfs/refresh recursive=true)
    pub fn trigger_background_warmup(exe_path: String, rc_port: u16) {
        thread::spawn(move || {
            // 等待掛載行程完全穩定並啟動 RC 服務
            thread::sleep(Duration::from_millis(1500));
            let rc_url = format!("http://127.0.0.1:{}", rc_port);
            let mut warm_cmd = Command::new(&exe_path);
            warm_cmd.args([
                "rc",
                "--url",
                &rc_url,
                "--rc-no-auth",
                "vfs/refresh",
                "recursive=true",
                "_async=true",
            ]);
            warm_cmd.creation_flags(CREATE_NO_WINDOW);
            let _ = warm_cmd.output();
        });
    }
}

/// 取得一個可用的本機 TCP Port，避免多掛載或 WebGUI 端口碰撞
fn get_available_port() -> Option<u16> {
    TcpListener::bind("127.0.0.1:0")
        .ok()?
        .local_addr()
        .ok()
        .map(|addr| addr.port())
}

fn parse_env_bool(val: &str, default: bool) -> bool {
    match val.trim().to_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => true,
        "0" | "false" | "no" | "off" => false,
        _ => default,
    }
}

/// 解析 extra_flags 命令列字串
fn split_shell_args(input: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut quote_char = ' ';

    for c in input.chars() {
        match c {
            '"' | '\'' if !in_quotes => {
                in_quotes = true;
                quote_char = c;
            }
            c if in_quotes && c == quote_char => {
                in_quotes = false;
            }
            ' ' | '\t' if !in_quotes => {
                if !current.is_empty() {
                    args.push(current);
                    current = String::new();
                }
            }
            _ => {
                current.push(c);
            }
        }
    }
    if !current.is_empty() {
        args.push(current);
    }
    args
}
