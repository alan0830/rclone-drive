# Rclone Drive & Sync v1.6.2 更新日誌

## 🚀 版本亮點概述

此版本專注於**掛載效能與目錄瀏覽體驗極限調校**，全面解決在 Windows 掛載包含大量小檔案的雲端硬碟（尤其是 Google Drive）時遇到的目錄載入緩慢、點擊凍結與檔案總管白屏假死問題：

---

### 1. ⚡ 目錄與屬性快取優化（解決目錄瀏覽卡頓與假死）
- **長效目錄快取（`--dir-cache-time 72h`）**：預設啟用 72 小時長效快取，目錄結構建立後直接自記憶體響應，消除頻繁重複遍歷雲端的延遲。
- **雲端變更輪詢（`--poll-interval 1m`）**：在快取期間透過 API 輪詢感知雲端異動，維持高一致性。
- **核心屬性快取超時（`--attr-timeout 10m`）**：延長檔案大小與屬性的核心快取期限，大幅減輕 Windows 檔案總管密集查詢時的負載。
- **抑制修改時間頻繁查詢（`--no-modtime`）**：列舉目錄時不重複讀取/核對遠端 mtime，大量小檔案情境下開啟資料夾速度提升數倍。

---

### 2. 💽 完整 VFS 磁碟快取與 Google Drive 節流突破（解決點擊/預覽凍結）
- **完整 VFS 磁碟快取模式**：預設 `--vfs-cache-mode full`，搭配 `--vfs-cache-max-size 50G` 與 `--vfs-cache-max-age 24h`，支援隨機讀寫並全面相容 Office、播放器等桌面軟體。
- **漸進式切片讀取（Chunked Reading）**：`--vfs-read-chunk-size 64M` 搭配 `--vfs-read-chunk-size-limit 1G`，小檔案秒開、大檔串流吞吐極致流暢。
- **Google Drive Pacer 節流調校**：針對 Google Drive 遠端自動啟用 `--drive-pacer-min-sleep 10ms` 與 `--drive-pacer-burst 200`，突破原廠 100ms 節流延遲瓶頸。

---

### 3. 🔥 背景非阻塞預熱機制 (Background Cache Warm-up)
- **動態 Port 隔離**：每次掛載自動取得未佔用之本機 TCP Port，完全杜絕多磁碟掛載或 Web-GUI (5572) 埠位衝突。
- **非阻塞預熱執行**：掛載就緒後，在後台自動非同步調用 Remote Control API：
  ```bash
  vfs/refresh recursive=true _async=true
  ```
  在背景完成完整雲端目錄樹快取預先載入，使用者首次點入各級子資料夾完全無需等待。

---

### 4. 🎛️ 模式切換設計（Preset Mode）與全方位相容支援
- **一鍵切換預設集 (Preset)**：
  - **⚡ 極速推薦 (Fast)**：一鍵啟用所有推薦最佳化配置（預設模式）。
  - **⚖️ 官方平衡 (Default)**：回退至 Rclone 官方標準配置。
  - **🛠️ 自訂進階 (Custom)**：可於摺疊面板中逐項自由微調各項參數與自訂 Flags。
- **多元設定方式**：支援 UI 介面微調、外部設定檔 `rclone_mount_config.json` 以及系統環境變數覆寫（如 `RCLONE_MOUNT_PRESET=fast`）。

---

## 🔒 檔案校驗雜湊值 (SHA-256 Checksums)

為確保您下載的安裝檔為官方發布且未受任何第三方竄改，請比對以下 SHA-256 雜湊值：

| 檔案名稱 | SHA-256 雜湊值 |
| :--- | :--- |
| `RcloneDrive_v1.6.2_x64_Setup.exe` | `2de2b529b3ae2b029e8974ebf0e1ff13d45dfc97a67b02c91b66f6de0d019a06` |
| `RcloneDrive_v1.6.2_Portable_x64.exe` | `dea5886fabd975cf1e3fc9c4971f8fd84d6dd27f5ba30bc0f7fe7ad3204e0a0c` |
| `RcloneDrive_v1.6.2_x64.msi` | `b16b77d88e23172188b3bb359189d7aa6f36bba62eb0a56c5e2c4bfae6617122` |

---

## 🛡️ 首次下載與防毒軟體提示說明

若您在下載或執行時遇到防毒軟體提示「資訊結果不明」或 Windows Defender SmartScreen 提示：
1. **解除提示**：
   - Windows SmartScreen：點選 **「其他資訊」** ➔ **「仍要執行」**。
2. **雜湊比對驗證**：在 PowerShell 執行 `Get-FileHash <檔案路徑> -Algorithm SHA256`，比對輸出與上方 Checksum 一致，即可 100% 確保安全性。
