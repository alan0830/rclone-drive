# Rclone Drive & Sync v1.5.1 更新日誌

本版本主要修復 Microsoft OneDrive 雲端服務類型識別與磁碟圖示匹配問題。

---

## 🛠️ 問題修復與體驗優化

### 1. 修復 Microsoft OneDrive 被誤判為 Google Drive 的問題
- **類型識別優先順序修正**：原先關鍵字判定順序中，因 `OneDrive` 包含 `drive` 字串而被優先比對為 Google Drive；現已調整比對權重，確保 OneDrive 正確識別並顯示 Microsoft 專屬藍色風格與標籤。
- **修改設定介面提示優化**：在「修改雲端設定」對話框中，若為 OneDrive 雲端，自訂 Client ID / Secret 之提示文字動態調整為 Microsoft / Azure API 說明。

### 2. 修復掛載 OneDrive 時的 Windows 檔案總管磁碟圖示
- **專屬圖示綁定修正**：同步調整後端磁碟圖示關聯邏輯，掛載 OneDrive 時在「我的電腦 / 檔案總管」中將精準套用專屬的 `onedrive.ico` 雲端硬碟圖示，告別 Google Drive 圖示誤植問題。
