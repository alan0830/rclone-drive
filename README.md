# Rclone Drive & Sync

<div align="center">

![Platform](https://img.shields.io/badge/Platform-Windows%2010%20%2F%2011-blue?style=flat-square)
![Tauri](https://img.shields.io/badge/Tauri-v2-24c8db?style=flat-square&logo=tauri)
![Vue](https://img.shields.io/badge/Vue-3.5-42b883?style=flat-square&logo=vue.js)
![Rust](https://img.shields.io/badge/Rust-2021-DEA584?style=flat-square&logo=rust)
![License](https://img.shields.io/badge/License-MIT-green?style=flat-square)

**輕量、現代化的全功能 Rclone 雲端硬碟虛擬掛載與同步排程工具（RaiDrive 替代方案）**

</div>

---

## 🌟 特色亮點

- **💽 虛擬硬碟掛載 (RaiDrive 體驗)**：
  - 將 Google Drive、OneDrive、Dropbox、WebDAV、SFTP、S3 等雲端空間掛載為 Windows 本機磁碟代號（如 `Z:`、`Y:`、`X:`）。
  - **各雲端專屬磁碟圖示**：掛載後檔案總管自動套用各雲端專屬圖示（如 Google Drive、Dropbox、OneDrive、WebDAV 等），告別空白白紙圖示。
  - 全面採用穩定的網路磁碟機制（`--network-mode`），一般使用者權限即可秒速掛載，相容 Office 隨選即開與各類檔案讀寫。
  - 支援 VFS 快取模式調整（`full` / `writes` / `minimal` / `off`）。

- **🎨 全圖形化雲端管理 (告別黑視窗)**：
  - 內建精美圖形精靈，無需開啟命令提示字元黑視窗。
  - 支援 Google Drive / OneDrive 預設瀏覽器一鍵 OAuth 登入授權與重新授權。
  - 每張硬碟卡片均提供「修改設定」功能，可隨時調整密碼、帳號或連線網址。

- **🔄 檔案同步與差異比對 (RcloneView Plus 風格)**：
  - 支援兩端路徑快速比對（`Check Diff`），視覺化標註新增、修改與缺失檔案。
  - 提供單向鏡像同步（`Sync`）與安全增量備份（`Copy`）模式。

- **⏰ 背景定時排程任務**：
  - 可建立多組排程任務，定時自動將指定資料夾備份至雲端。
  - 即使關閉視窗，只要常駐系統匣即可於背景自動執行。

- **🎛️ Windows 系統匣常駐與開機自啟 (System Tray)**：
  - 點擊視窗關閉 (X) 自動最小化至右下角系統匣。
  - 雙擊系統匣圖示即可秒開主介面，右鍵提供快捷操作選單。
  - **開機自動啟動與縮小至系統匣**：支援登入 Windows 自動啟動，並可自由切換「開機啟動後直接縮小至系統匣常駐」或顯示主視窗。

- **🌐 官方 Web-GUI 一鍵整合**：
  - 內建 Rclone 原生 Web-GUI 伺服器啟動與控制，支援直接於瀏覽器分頁中進行進階操作。

- **⚡ 體積極致輕巧**：
  - 基於 Tauri v2 + WebView2，單一執行檔僅約 **4.5 MB**，記憶體佔用極低。

---

## 📋 系統要求

1. **Windows 10 / 11** (64-bit)
2. **[WinFsp](https://winfsp.dev/)**（Windows 虛擬檔案系統核心，掛載磁碟代號必備依賴）
3. **[Rclone](https://rclone.org/downloads/)**（預設路徑建議放於 `C:\rclone\rclone.exe`，亦可在軟體中自訂路徑）

---

## 🚀 下載與使用

前往 [Releases 頁面](../../releases) 下載最新版本：

- **標準安裝引導版 (`RcloneDrive_vX.X.X_x64_Setup.exe`)**：推薦！標準 NSIS 安裝精靈，自動建立桌面與開始功能表捷徑，支援原地覆蓋升級。
- **單一免安裝便攜版 (`RcloneDrive_vX.X.X_Portable_x64.exe`)**：下載後隨開即用，免安裝，適合隨身碟或即時測試。
- **MSI 安裝套件 (`RcloneDrive_vX.X.X_x64.msi`)**：企業大量部署適用的 Windows Installer 封裝。

---

## 🛡️ 首次下載安全性提示（SmartScreen / 防毒軟體告警說明）

由於本專案為免費開源軟體，且每當新版本剛釋出時尚未累積大量下載信譽，部分防毒軟體（如 Symantec 賽門鐵克、趨勢科技）或 Windows Defender SmartScreen 可能會跳出安全性提示：
- **「關於此檔案的資訊結果不明」**、**「極少使用者」**、**「極新」**
- **「Windows 已保護您的電腦」**（SmartScreen 提示未知的發行者）

### 💡 這是正常現象，請放心使用：
1. **為什麼會跳出提示？**
   商業軟體通常每年花費高額費用購買企業 EV 程式碼簽名憑證；開源專案在釋出全新版本時，防毒軟體信譽庫（Reputation-based Protection）因「樣本剛發布、在該防毒軟體使用者社群中累積下載次數較少」而主動提醒，並非含有惡意程式碼。本專案程式碼完全開源透明，歡迎檢閱審計。
2. **如何允許執行？**
   - **Symantec (賽門鐵克)**：點擊提示視窗中的 **「允許此檔案」** 即可正常執行。
   - **Windows Defender SmartScreen**：點擊提示畫面上的 **「其他資訊」**（More info） ➔ 點選 **「仍要執行」**（Run anyway）。
   - **Chrome / Edge 瀏覽器**：若下載時顯示安全性封鎖，點擊右側 `...` 選單 ➔ 點選 **「保留」** ➔ **「仍要保留」**。

### 🔍 雜湊值比對驗證 (SHA-256 Checksum)
為了確保您下載的檔案完整且未遭受任何第三方竄改，本專案在每次發行的 [GitHub Releases 頁面](../../releases) 均提供各檔案的 SHA-256 雜湊值（以及 `checksums.txt`）。

您可以在 Windows PowerShell 中執行以下指令檢驗下載檔案的雜湊值：

```powershell
# 檢驗安裝檔雜湊值
Get-FileHash .\RcloneDrive_v1.5.5_x64_Setup.exe -Algorithm SHA256

# 檢驗便攜版雜湊值
Get-FileHash .\RcloneDrive_v1.5.5_Portable_x64.exe -Algorithm SHA256
```

比對輸出的 `Hash` 字串是否與 GitHub Releases 頁面上公布的一致，即可 100% 確保檔案未受損壞或竄改！

---

## 🛠️ 開發與編譯

本專案採用 **Tauri v2** + **Vue 3**：

### 前置需求
- [Node.js](https://nodejs.org/) (>= 18)
- [Rust & Cargo](https://rustup.rs/) (MSVC toolchain)
- Microsoft Visual Studio C++ Build Tools

### 本地開發
```bash
# 1. 安裝前端依賴
npm install

# 2. 啟動桌面開發預覽
npm run tauri dev
```

### 打包正式發行版
```bash
# 打包產出單一獨立 .exe 與安裝程式 (NSIS / MSI)
npm run tauri build
```
產出的檔案將位於 `src-tauri/target/release/` 與 `src-tauri/target/release/bundle/nsis/`。

---

## 📄 開源授權

本專案基於 [MIT License](LICENSE) 開源。
