# Rclone Drive & Sync v1.5.5 更新日誌

本版本重點強化安裝與更新時的程序管理與磁碟釋放機制：手動執行安裝程式時，將自動卸載所有既有掛載的雲端磁碟、終止先前執行的主程式及背景程序，確保無任何檔案鎖定與衝突。

---

## 🌟 新增功能與改進重點

### 1. 🛡️ 手動執行安裝時自動安全卸載與關閉舊版程式
- **安裝啟動即刻卸載 (`.onInit`)**：
  - 雙擊執行 `Setup.exe` 安裝檔時，安裝精靈啟動第一時間即自動執行強制卸載機制，終止 `rclone.exe` 背景掛載行程，使 WinFsp 網路虛擬磁碟乾淨解除掛載。
- **終止舊版程式避免檔案佔用**：
  - 自動結束正在背景或系統匣執行的 `rclone-drive.exe` 與 `RcloneDrive.exe`，避免覆蓋舊檔案時跳出「檔案被另一個程序使用中」或 RestartManager 的錯誤中斷提示。
- **清除虛擬磁碟圖示快取**：
  - 自動清理 Windows 註冊表中暫存的掛載磁碟圖示機碼 (`DriveIcons`) 並喚醒 Windows 檔案總管重新整理，杜絕幽靈磁碟機圖示殘留。
- **檔案覆蓋前二次保險 (`Section Install`)**：
  - 在真正寫入檔案至安裝目錄前再次驗證並釋放鎖定，確保安裝過程 100% 順暢穩定。
- **反安裝同步安全清理 (`Function un.onInit`)**：
  - 執行解除安裝時同樣會優先卸載所有雲端磁碟並關閉程序，乾淨刪除程式檔案。

---

## 🔒 檔案校驗雜湊值 (SHA-256 Checksums)

為確保您下載的安裝檔為官方發布且未受任何第三方竄改，請比對以下 SHA-256 雜湊值：

| 檔案名稱 | SHA-256 雜湊值 |
| :--- | :--- |
| `RcloneDrive_v1.5.5_x64_Setup.exe` | `bc1f12ebab86f8737d95c8eaca594c7d4e033639f1dbca73f5fe52d167ceef6d` |
| `RcloneDrive_v1.5.5_Portable_x64.exe` | `be7f472788b45d1afb22739ee12b2612cd9d4845c511d9d7669c537361fbec8f` |
| `RcloneDrive_v1.5.5_x64.msi` | `6d54b4aeb03596282aaa8cf8ee286c3a22a065eb216802820ae7de0e9ac4bd0e` |

---

## 🛡️ 首次下載與防毒軟體提示說明

若您在下載或執行時遇到防毒軟體（如賽門鐵克 Symantec Endpoint Protection、趨勢科技）提示「資訊結果不明」、「極少使用者」、「極新」或 Windows Defender SmartScreen 提示：
1. **解除提示**：
   - Symantec：點選 **「允許此檔案」**。
   - Windows SmartScreen：點選 **「其他資訊」** ➔ **「仍要執行」**。
2. **雜湊比對驗證**：在 PowerShell 執行 `Get-FileHash <檔案路徑> -Algorithm SHA256`，比對輸出與上方 Checksum 一致，即可 100% 確保安全性。
