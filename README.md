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

- **🎛️ Windows 系統匣常駐 (System Tray)**：
  - 點擊視窗關閉 (X) 自動最小化至右下角系統匣。
  - 雙擊系統匣圖示即可秒開主介面，右鍵提供快捷操作選單。
  - 支援開機自動在背景靜默啟動。

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

- **單一免安裝版 (`rclone-drive.exe`)**：下載後隨開即用，適合放在桌面或隨身碟。
- **安裝引導版 (`RcloneDrive_1.0.0_x64-setup.exe`)**：標準安裝精靈，自動建立桌面與開始功能表捷徑。

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
