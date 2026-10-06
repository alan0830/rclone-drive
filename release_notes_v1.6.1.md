# Rclone Drive & Sync v1.6.1 更新日誌

## 🚀 版本亮點概述

此版本主要針對**雲端服務支援完整度**、**Amazon S3 與相容物件儲存認證體驗**進行重大重構與升級，並提供多項人機交互改善：

---

### 1. 🌟 全面擴充雲端服務類型（支援 Rclone 原版 60+ 種儲存提供商）
- **分類清晰的提供商選單**：
  - **熱門個人與企業雲端**：Google Drive、Microsoft OneDrive、Dropbox、Box、pCloud、Mega、Proton Drive、Yandex Disk、Koofr 等。
  - **物件儲存 (S3 & Cloud)**：Amazon S3、Backblaze B2、Google Cloud Storage (GCS)、Microsoft Azure Blob 等。
  - **標準通訊協定與區網芳鄰**：WebDAV、SMB/CIFS (Windows 網路芳鄰/Samba)、SFTP/SSH、FTP 伺服器等。
  - **進階虛擬儲存**：本機路徑 (Local)、透明加密保險箱 (Crypt)、聯合磁碟 (Union) 等。
- **即時搜尋過濾功能**：新增雲端時可輸入關鍵字（例如 `s3`、`b2`、`smb`、`nas`、`drive`）即時篩選目標雲端服務。
- **⚡ 一鍵開啟 Rclone 原版互動設定終端 (rclone config)**：對話框右上角提供快捷按鈕，隨時可啟動原版命令列互動精靈進行最進階的客製配置。

---

### 2. 🛡️ 徹底完善 Amazon S3 / 相容物件儲存認證與配置流程
- **直覺金鑰直連認證**：
  - 明確提示 S3 與物件儲存使用 API **存取金鑰 ID (Access Key ID)** 與 **私密金鑰 (Secret Access Key)** 進行連線，不使用亦不需要開啟瀏覽器 OAuth 跳轉。
  - 修正按鈕提示，杜絕原先「請在瀏覽器登入」的混淆指引。
- **支援豐富的 S3 提供商與端點預設**：
  - AWS S3 官方、Cloudflare R2、MinIO 自架、Wasabi、Backblaze B2 (S3 相容)、阿里雲 OSS、騰訊雲 COS、DigitalOcean Spaces、Ceph 等。
  - 切換提供商時自動帶入預設 Region 與端點提示（例如 Cloudflare R2 自動帶入 `auto` 區域，MinIO 自動啟用路徑樣式）。
- **完整金鑰安全管理**：
  - 私密金鑰支援快速顯示/隱藏切換眼睛圖示。
  - 支援進階選項：強制路徑樣式 (Force Path Style) 及環境變數/AWS IAM 憑證 (Env Auth)。
- **設定修改支援**：修改遠端對話框中亦完整支援檢視與調整 S3 參數。

---

### 3. 🔧 系統與體驗改進
- **版本號升級為 v1.6.1**：全域版號統一更新（Tauri, Cargo, Vue）。
- **遠端命名防呆保護**：建立或更名時自動過濾特殊無效字元（`: / \ [ ] * ? < > | "`）。

---

## 🔒 檔案校驗雜湊值 (SHA-256 Checksums)

為確保您下載的安裝檔為官方發布且未受任何第三方竄改，請比對以下 SHA-256 雜湊值：

| 檔案名稱 | SHA-256 雜湊值 |
| :--- | :--- |
| `RcloneDrive_v1.6.1_x64_Setup.exe` | `68ec83ba3991cd0ca6e708835646b54b183ca55c6a22156ccedf978cb43ffa3e` |
| `RcloneDrive_v1.6.1_Portable_x64.exe` | `a6e6544ae1d67c59b997394f19aed2872802436537e10f57f7940eccada5a9c7` |
| `RcloneDrive_v1.6.1_x64.msi` | `24284ad936f4cb85ca326b33010060c7604ae8df2e843465b2648b31ba9d8639` |

---

## 🛡️ 首次下載與防毒軟體提示說明

若您在下載或執行時遇到防毒軟體（如賽門鐵克 Symantec Endpoint Protection、趨勢科技）提示「資訊結果不明」、「極少使用者」、「極新」或 Windows Defender SmartScreen 提示：
1. **解除提示**：
   - Symantec：點選 **「允許此檔案」**。
   - Windows SmartScreen：點選 **「其他資訊」** ➔ **「仍要執行」**。
2. **雜湊比對驗證**：在 PowerShell 執行 `Get-FileHash <檔案路徑> -Algorithm SHA256`，比對輸出與上方 Checksum 一致，即可 100% 確保安全性。

