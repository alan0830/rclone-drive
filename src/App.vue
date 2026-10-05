<script setup>
import { ref, reactive, computed, onMounted, onUnmounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { enable, isEnabled, disable } from "@tauri-apps/plugin-autostart";
import {
  HardDrive,
  Cloud,
  FolderOpen,
  Power,
  RefreshCw,
  Settings,
  PlusCircle,
  ExternalLink,
  CheckCircle2,
  AlertTriangle,
  XCircle,
  Check,
  X,
  Sliders,
  ShieldCheck,
  Disc3,
  Layers,
  Info,
  ArrowRightLeft,
  CalendarClock,
  Globe,
  Trash2,
  FileCheck,
  Play,
  RotateCw,
  FolderSync,
  Pencil,
  KeyRound,
  DownloadCloud,
  ArrowRight,
  ArrowLeft,
  Equal,
  Folder,
  Search,
  Filter,
  Copy,
  ChevronRight,
  FolderPlus,
  SlidersHorizontal,
  Sun,
  Moon,
  Zap
} from "lucide-vue-next";

// Theme State (Dark / Light)
const isLightMode = ref(localStorage.getItem("rclone_theme") === "light");

function applyTheme(isLight) {
  if (isLight) {
    document.documentElement.setAttribute("data-theme", "light");
  } else {
    document.documentElement.removeAttribute("data-theme");
  }
}

function toggleTheme() {
  isLightMode.value = !isLightMode.value;
  localStorage.setItem("rclone_theme", isLightMode.value ? "light" : "dark");
  applyTheme(isLightMode.value);
  showToast(isLightMode.value ? "已切換為淺色主題 ☀️" : "已切換為深色主題 🌙", "info");
}

// Active Tab
const activeTab = ref("mounts"); // 'mounts' | 'sync' | 'scheduler' | 'webgui'

// Environment & State
const envStatus = ref({
  rclone_found: false,
  rclone_path: "C:\\rclone\\rclone.exe",
  rclone_version: "",
  winfsp_found: false,
  webgui_running: false,
  webgui_url: "http://127.0.0.1:5572/"
});

const remotes = ref([]);
const availableDrives = ref([]);
const isRefreshing = ref(false);
const showSettings = ref(false);
const showAddRemoteModal = ref(false);
const showEditRemoteModal = ref(false);
const autostartActive = ref(false);
const startMinimizedActive = ref(true);
const customRclonePath = ref(localStorage.getItem("rclone_custom_path") || "C:\\rclone\\rclone.exe");

// Configuration map for remotes (Persisted to localStorage)
const savedRemoteConfigs = JSON.parse(localStorage.getItem("rclone_remote_configs") || "{}");
const remoteConfigs = reactive(savedRemoteConfigs);

function saveRemoteConfigs() {
  localStorage.setItem("rclone_remote_configs", JSON.stringify(remoteConfigs));
}

const loadingRemotes = reactive(new Set());
const autoMountList = ref(JSON.parse(localStorage.getItem("rclone_auto_mount_list") || "[]"));

// Add Remote Modal State
const newRemoteForm = reactive({
  name: "",
  type: "drive",
  url: "",
  user: "",
  pass: "",
  host: "",
  port: "21",
  clientId: "",
  clientSecret: "",
  isCreating: false
});

// Edit Remote Modal State
const editRemoteForm = reactive({
  name: "",
  newName: "",
  type: "",
  url: "",
  user: "",
  pass: "",
  host: "",
  port: "21",
  clientId: "",
  clientSecret: "",
  isSaving: false,
  isReconnecting: false
});

// Sync & Compare State (RcloneView Plus)
const syncForm = reactive({
  source: "",
  dest: "", // 主要目的路徑 (相容既有排程與比對)
  extraDests: [], // 額外目的路徑清單: [ { id: string, path: string } ]
  action: "copy", // 預設使用 copy (增量備份，更安全)
  excludeFilter: "",
  searchKeyword: "",
  filterModes: {
    source_only: true, // ➡️ 來源待備份
    dest_only: true,   // ⬅️ 目的端多出
    different: true,   // ≠ 差異
    equal: false,      // ＝ 相同
    error: true        // ❗ 異常
  },
  selectedItems: new Set(),
  autoAddToScheduler: false, // 是否在設定/同步完成後自動加入排程
  isChecking: false,
  isRunning: false,
  isCopyingSelected: false,
  diffResult: null,
  taskLog: ""
});

// Cloud Directory Browser Modal State
const showCloudBrowseModal = ref(false);
const cloudBrowseTarget = ref("source"); // 'source' | 'dest'
const cloudBrowseRemote = ref("");
const cloudBrowseCurrentSubpath = ref("");
const cloudBrowseDirs = ref([]);
const isLoadingCloudDirs = ref(false);

// Scheduler State
const scheduledTasks = ref(JSON.parse(localStorage.getItem("rclone_scheduled_tasks") || "[]"));
const showAddTaskModal = ref(false);
const showEditTaskModal = ref(false);
const newTask = reactive({
  name: "",
  source: "",
  dest: "",
  action: "copy",
  excludeFilter: "",
  intervalMinutes: 60,
  enabled: true
});
const editTaskForm = reactive({
  id: "",
  name: "",
  source: "",
  dest: "",
  action: "copy",
  excludeFilter: "",
  intervalMinutes: 60,
  enabled: true
});

let schedulerTimer = null;

// GitHub Auto-Update State & Settings
const CURRENT_VERSION = "1.5.3";
const GITHUB_REPO = "alan0830/rclone-drive";

const savedUpdateSettings = JSON.parse(localStorage.getItem("rclone_update_settings") || "{}");
const updateSettings = reactive({
  enabled: savedUpdateSettings.enabled !== false, // 預設開啟自動檢查
  intervalDays: Number(savedUpdateSettings.intervalDays) || 7, // 預設一週 (7 天)
  lastCheckTs: Number(savedUpdateSettings.lastCheckTs) || 0,
  lastCheckTimeStr: savedUpdateSettings.lastCheckTimeStr || "從未檢查"
});

function saveUpdateSettings() {
  localStorage.setItem("rclone_update_settings", JSON.stringify(updateSettings));
}

function toggleUpdateEnabled() {
  updateSettings.enabled = !updateSettings.enabled;
  saveUpdateSettings();
  showToast(updateSettings.enabled ? "已啟用自動檢查版本更新 🔔" : "已關閉自動檢查更新 🔕", "info");
}

const showUpdateModal = ref(false);
const isCheckingUpdate = ref(false);
const isDownloadingUpdate = ref(false);
const updateInfo = reactive({
  latestVersion: "",
  currentVersion: CURRENT_VERSION,
  releaseTitle: "",
  releaseNotes: "",
  publishedAt: "",
  htmlUrl: "",
  downloadUrl: "",
  assetName: ""
});

function compareSemver(v1, v2) {
  const clean1 = (v1 || "").replace(/^v/i, "").trim().split(".").map((n) => parseInt(n, 10) || 0);
  const clean2 = (v2 || "").replace(/^v/i, "").trim().split(".").map((n) => parseInt(n, 10) || 0);
  const len = Math.max(clean1.length, clean2.length);
  for (let i = 0; i < len; i++) {
    const num1 = clean1[i] || 0;
    const num2 = clean2[i] || 0;
    if (num1 > num2) return 1;
    if (num1 < num2) return -1;
  }
  return 0;
}

async function checkForUpdates(manual = false) {
  if (!manual) {
    if (!updateSettings.enabled) return;
    const intervalMs = (updateSettings.intervalDays || 7) * 24 * 60 * 60 * 1000;
    if (Date.now() - (updateSettings.lastCheckTs || 0) < intervalMs) {
      return; // 檢查間隔未到，略過
    }
  }

  isCheckingUpdate.value = true;
  try {
    const res = await fetch(`https://api.github.com/repos/${GITHUB_REPO}/releases/latest`, {
      headers: { Accept: "application/vnd.github.v3+json" }
    });
    if (!res.ok) {
      throw new Error(`GitHub API 回應錯誤 (${res.status} ${res.statusText})`);
    }
    const data = await res.json();
    const latestTag = data.tag_name || "";

    updateSettings.lastCheckTs = Date.now();
    updateSettings.lastCheckTimeStr = new Date().toLocaleString();
    saveUpdateSettings();

    if (compareSemver(latestTag, CURRENT_VERSION) > 0) {
      updateInfo.latestVersion = latestTag.replace(/^v/i, "");
      updateInfo.currentVersion = CURRENT_VERSION;
      updateInfo.releaseTitle = data.name || latestTag;
      updateInfo.releaseNotes = data.body || "本次更新包含功能改進與問題修復。";
      updateInfo.publishedAt = data.published_at ? new Date(data.published_at).toLocaleDateString() : "";
      updateInfo.htmlUrl = data.html_url || `https://github.com/${GITHUB_REPO}/releases/latest`;

      let assetUrl = "";
      let assetName = "";
      if (Array.isArray(data.assets)) {
        const exeAsset = data.assets.find(
          (a) => a.name.toLowerCase().endsWith(".exe") || a.name.toLowerCase().endsWith(".msi")
        );
        if (exeAsset) {
          assetUrl = exeAsset.browser_download_url;
          assetName = exeAsset.name;
        }
      }
      updateInfo.downloadUrl = assetUrl || updateInfo.htmlUrl;
      updateInfo.assetName = assetName;

      showUpdateModal.value = true;
    } else {
      if (manual) {
        showToast(`目前已是最新版本 (v${CURRENT_VERSION})！`, "success");
      }
    }
  } catch (err) {
    if (manual) {
      showToast(`檢查更新失敗: ${err.message || err}`, "error");
    }
  } finally {
    isCheckingUpdate.value = false;
  }
}

async function executeUpdate() {
  if (!updateInfo.downloadUrl || (!updateInfo.downloadUrl.endsWith(".exe") && !updateInfo.downloadUrl.endsWith(".msi"))) {
    await openUrl(updateInfo.htmlUrl);
    showUpdateModal.value = false;
    return;
  }

  isDownloadingUpdate.value = true;
  showToast("正在安全解除所有已掛載磁碟，並下載安裝更新...", "info");
  try {
    // 1. 先安全解除所有已掛載的磁碟機
    try {
      await invoke("unmount_all");
      await refreshAll();
    } catch (_) {}

    const msg = await invoke("download_and_install_update", {
      downloadUrl: updateInfo.downloadUrl
    });
    showToast(msg || "更新安裝程式已成功啟動，主程式即將退出...", "success");
    showUpdateModal.value = false;
  } catch (err) {
    showToast(`自動啟動更新失敗: ${err}，已為您在瀏覽器開啟下載頁面`, "error");
    await openUrl(updateInfo.htmlUrl);
    showUpdateModal.value = false;
  } finally {
    isDownloadingUpdate.value = false;
  }
}

// Toast notification
const toast = ref({ show: false, message: "", type: "info" });
let toastTimer = null;
function showToast(message, type = "info") {
  if (toastTimer) clearTimeout(toastTimer);
  toast.value = { show: true, message, type };
  toastTimer = setTimeout(() => {
    toast.value.show = false;
  }, 4500);
}

// Open External URL
async function openUrl(url) {
  try {
    await invoke("open_browser_url", { url });
  } catch (err) {
    showToast(`開啟網頁失敗: ${err}`, "error");
  }
}

// Provider details helper
function getProviderDetails(type = "") {
  const t = type.toLowerCase();
  if (t.includes("onedrive")) return { name: "OneDrive", color: "#0078D4", bg: "rgba(0, 120, 212, 0.15)" };
  if (t.includes("drive")) return { name: "Google Drive", color: "#4285F4", bg: "rgba(66, 133, 244, 0.15)" };
  if (t.includes("photo")) return { name: "Google Photos", color: "#EA4335", bg: "rgba(234, 67, 53, 0.15)" };
  if (t.includes("dropbox")) return { name: "Dropbox", color: "#0061FF", bg: "rgba(0, 97, 255, 0.15)" };
  if (t.includes("s3")) return { name: "Amazon S3", color: "#FF9900", bg: "rgba(255, 153, 0, 0.15)" };
  if (t.includes("webdav")) return { name: "WebDAV", color: "#10B981", bg: "rgba(16, 185, 129, 0.15)" };
  if (t.includes("ftp")) return { name: "FTP / SFTP", color: "#8B5CF6", bg: "rgba(139, 92, 246, 0.15)" };
  return { name: type || "雲端儲存", color: "#38BDF8", bg: "rgba(56, 189, 248, 0.15)" };
}

// Autostart & Start Minimized
async function checkAutostart() {
  try {
    autostartActive.value = await isEnabled();
    const settings = await invoke("get_app_settings");
    if (settings && typeof settings.start_minimized_to_tray === "boolean") {
      startMinimizedActive.value = settings.start_minimized_to_tray;
    }
  } catch (err) {
    console.error("檢查開機自啟與偏好失敗:", err);
  }
}

async function toggleAutostart() {
  try {
    if (autostartActive.value) {
      await disable();
      autostartActive.value = false;
      showToast("已關閉開機自動啟動", "info");
    } else {
      await enable();
      autostartActive.value = true;
      showToast("已開啟開機自動啟動！", "success");
    }
  } catch (err) {
    showToast(`設定開機自啟失敗: ${err}`, "error");
  }
}

async function toggleStartMinimized() {
  try {
    startMinimizedActive.value = !startMinimizedActive.value;
    await invoke("save_app_settings", {
      settings: {
        start_minimized_to_tray: startMinimizedActive.value
      }
    });
    showToast(
      startMinimizedActive.value
        ? "已啟用「開機啟動後縮小至系統匣」"
        : "已停用「開機啟動後縮小至系統匣」（開機自啟時將直接顯示主視窗）",
      "success"
    );
  } catch (err) {
    showToast(`設定縮小至系統匣失敗: ${err}`, "error");
  }
}

// Refresh status and remotes
async function refreshAll(isInitial = false) {
  isRefreshing.value = true;
  try {
    const env = await invoke("check_environment", {
      rclonePath: customRclonePath.value.trim() || null
    });
    envStatus.value = env;

    const drives = await invoke("get_available_drives");
    availableDrives.value = drives;

    const remoteList = await invoke("get_remotes", {
      rclonePath: customRclonePath.value.trim() || null
    });
    remotes.value = remoteList;

    try {
      await invoke("apply_mounted_drive_icons", { remotes: remoteList });
    } catch (e) {
      console.warn("套用磁碟機圖示警告:", e);
    }

    let driveIndex = 0;
    remoteList.forEach((r) => {
      if (!remoteConfigs[r.name]) {
        // 全新遠端，尚未設定過磁碟代號，避開已被其他遠端選取的代號
        const usedLetters = Object.values(remoteConfigs)
          .map((c) => c?.driveLetter)
          .filter(Boolean);
        const assigned =
          availableDrives.value.find((d) => !usedLetters.includes(d)) ||
          availableDrives.value[driveIndex] ||
          "Z:";
        driveIndex++;

        remoteConfigs[r.name] = {
          driveLetter: r.mounted_drive || assigned,
          volname: r.name,
          cacheMode: "full",
          readOnly: false,
          autoMount: autoMountList.value.includes(r.name)
        };
      } else {
        // 已有儲存的使用者自訂設定，絕對保留使用者的 driveLetter！
        if (r.is_mounted && r.mounted_drive) {
          remoteConfigs[r.name].driveLetter = r.mounted_drive;
        }
        remoteConfigs[r.name].autoMount = autoMountList.value.includes(r.name);
      }
    });
    saveRemoteConfigs();

    if (isInitial && autoMountList.value.length > 0) {
      for (const r of remoteList) {
        if (!r.is_mounted && autoMountList.value.includes(r.name)) {
          mountDrive(r.name, false);
        }
      }
    }
  } catch (err) {
    showToast(`載入失敗: ${err}`, "error");
  } finally {
    isRefreshing.value = false;
  }
}

// Mount Remote (Network Mode exclusively)
async function mountDrive(remoteName, notify = true) {
  if (!envStatus.value.winfsp_found) {
    showToast("掛載需要 WinFsp，請先點選上方按鈕安裝 WinFsp！", "error");
    return;
  }
  if (!envStatus.value.rclone_found) {
    showToast("找不到 rclone.exe，請先下載安裝！", "error");
    return;
  }

  const conf = remoteConfigs[remoteName];
  if (!conf || !conf.driveLetter) {
    showToast("請先選擇磁碟機代號！", "error");
    return;
  }

  loadingRemotes.add(remoteName);
  try {
    const targetRemote = remotes.value.find((r) => r.name === remoteName);
    const remoteType = targetRemote ? targetRemote.remote_type : null;

    await invoke("mount_remote", {
      rclonePath: customRclonePath.value.trim() || null,
      remote: remoteName,
      driveLetter: conf.driveLetter,
      volname: conf.volname || remoteName,
      cacheMode: conf.cacheMode || "full",
      readOnly: conf.readOnly || false,
      remoteType: remoteType
    });

    if (notify) {
      showToast(`已成功掛載 ${remoteName} 至磁碟機 ${conf.driveLetter}`, "success");
    }
    await refreshAll();
  } catch (err) {
    showToast(`掛載失敗: ${err}`, "error");
  } finally {
    loadingRemotes.delete(remoteName);
  }
}

// Unmount Remote
async function unmountDrive(remoteName) {
  loadingRemotes.add(remoteName);
  try {
    await invoke("unmount_remote", { remote: remoteName });
    showToast(`已卸載 ${remoteName}`, "info");
    await refreshAll();
  } catch (err) {
    showToast(`卸載失敗: ${err}`, "error");
  } finally {
    loadingRemotes.delete(remoteName);
  }
}

// Open in File Explorer
async function openExplorer(driveLetter) {
  try {
    await invoke("open_in_explorer", { driveLetter });
  } catch (err) {
    showToast(`無法開啟檔案總管: ${err}`, "error");
  }
}

// Mount All
async function mountAll() {
  const unmounted = remotes.value.filter((r) => !r.is_mounted);
  if (unmounted.length === 0) {
    showToast("所有雲端硬碟都已處於掛載狀態", "info");
    return;
  }
  for (const r of unmounted) {
    await mountDrive(r.name, false);
  }
  showToast("已執行掛載全部操作", "success");
  await refreshAll();
}

// Unmount All
async function unmountAll() {
  try {
    const count = await invoke("unmount_all");
    showToast(`已成功卸載 ${count} 個雲端硬碟`, "info");
    await refreshAll();
  } catch (err) {
    showToast(`卸載全部失敗: ${err}`, "error");
  }
}

// GUI Add Remote
async function submitAddRemote() {
  if (!newRemoteForm.name.trim()) {
    showToast("請輸入遠端名稱！", "error");
    return;
  }

  newRemoteForm.isCreating = true;
  const params = {};

  if (newRemoteForm.type === "drive" || newRemoteForm.type === "onedrive") {
    if (newRemoteForm.clientId.trim()) params.client_id = newRemoteForm.clientId.trim();
    if (newRemoteForm.clientSecret.trim()) params.client_secret = newRemoteForm.clientSecret.trim();
  } else if (newRemoteForm.type === "webdav") {
    params.url = newRemoteForm.url;
    params.vendor = "other";
    params.user = newRemoteForm.user;
    params.pass = newRemoteForm.pass;
  } else if (newRemoteForm.type === "ftp") {
    params.host = newRemoteForm.host;
    params.port = newRemoteForm.port || "21";
    params.user = newRemoteForm.user;
    params.pass = newRemoteForm.pass;
  } else if (newRemoteForm.type === "sftp") {
    params.host = newRemoteForm.host;
    params.user = newRemoteForm.user;
    params.pass = newRemoteForm.pass;
  }

  try {
    const msg = await invoke("create_remote_gui", {
      rclonePath: customRclonePath.value.trim() || null,
      name: newRemoteForm.name.trim(),
      remoteType: newRemoteForm.type,
      params
    });

    showToast(msg || "遠端建立成功！", "success");
    showAddRemoteModal.value = false;
    newRemoteForm.name = "";
    newRemoteForm.url = "";
    newRemoteForm.user = "";
    newRemoteForm.pass = "";
    newRemoteForm.host = "";
    newRemoteForm.clientId = "";
    newRemoteForm.clientSecret = "";
    await refreshAll();
  } catch (err) {
    showToast(`建立失敗: ${err}`, "error");
  } finally {
    newRemoteForm.isCreating = false;
  }
}

// Open Edit Remote Modal
async function openEditModal(remote) {
  editRemoteForm.name = remote.name;
  editRemoteForm.newName = remote.name;
  editRemoteForm.type = remote.remote_type;
  editRemoteForm.url = "";
  editRemoteForm.user = "";
  editRemoteForm.pass = "";
  editRemoteForm.host = "";
  editRemoteForm.port = "21";
  editRemoteForm.clientId = "";
  editRemoteForm.clientSecret = "";

  try {
    const details = await invoke("get_remote_detail", {
      rclonePath: customRclonePath.value.trim() || null,
      name: remote.name
    });
    if (details.url) editRemoteForm.url = details.url;
    if (details.user) editRemoteForm.user = details.user;
    if (details.host) editRemoteForm.host = details.host;
    if (details.port) editRemoteForm.port = details.port;
    if (details.client_id) editRemoteForm.clientId = details.client_id;
    if (details.client_secret) editRemoteForm.clientSecret = details.client_secret;
  } catch (err) {
    console.error("載入設定詳情失敗:", err);
  }

  showEditRemoteModal.value = true;
}

// Submit Edit Remote
async function submitEditRemote() {
  const trimmedNewName = (editRemoteForm.newName || "").trim();
  if (!trimmedNewName) {
    showToast("雲端硬碟名稱不能為空！", "error");
    return;
  }
  if (/[\\/:*?"<>|\[\]]/.test(trimmedNewName)) {
    showToast("雲端硬碟名稱不可包含特殊字元 (: / \\ [ ] * ? < > | \")", "error");
    return;
  }

  editRemoteForm.isSaving = true;
  const params = {};

  if (editRemoteForm.type.includes("drive") || editRemoteForm.type.includes("onedrive")) {
    params.client_id = editRemoteForm.clientId.trim();
    params.client_secret = editRemoteForm.clientSecret.trim();
  } else if (editRemoteForm.type === "webdav") {
    if (editRemoteForm.url) params.url = editRemoteForm.url;
    if (editRemoteForm.user) params.user = editRemoteForm.user;
    if (editRemoteForm.pass) params.pass = editRemoteForm.pass;
  } else if (editRemoteForm.type === "ftp" || editRemoteForm.type === "sftp") {
    if (editRemoteForm.host) params.host = editRemoteForm.host;
    if (editRemoteForm.port) params.port = editRemoteForm.port;
    if (editRemoteForm.user) params.user = editRemoteForm.user;
    if (editRemoteForm.pass) params.pass = editRemoteForm.pass;
  }

  try {
    const oldName = editRemoteForm.name;
    const msg = await invoke("update_remote_gui", {
      rclonePath: customRclonePath.value.trim() || null,
      name: oldName,
      newName: trimmedNewName,
      params
    });

    // If renamed, migrate local storage configs, auto-mount, and scheduled tasks
    if (trimmedNewName !== oldName) {
      if (remoteConfigs[oldName]) {
        remoteConfigs[trimmedNewName] = { ...remoteConfigs[oldName] };
        delete remoteConfigs[oldName];
        saveRemoteConfigs();
      }
      const autoIdx = autoMountList.value.indexOf(oldName);
      if (autoIdx !== -1) {
        autoMountList.value[autoIdx] = trimmedNewName;
        localStorage.setItem("rclone_auto_mount_list", JSON.stringify(autoMountList.value));
      }
      let changedTasks = false;
      for (const t of scheduledTasks.value) {
        if (t.source && t.source.startsWith(`${oldName}:`)) {
          t.source = t.source.replace(`${oldName}:`, `${trimmedNewName}:`);
          changedTasks = true;
        }
        if (t.dest && t.dest.startsWith(`${oldName}:`)) {
          t.dest = t.dest.replace(`${oldName}:`, `${trimmedNewName}:`);
          changedTasks = true;
        }
      }
      if (changedTasks) {
        localStorage.setItem("rclone_scheduled_tasks", JSON.stringify(scheduledTasks.value));
      }
    }

    showToast(msg || "設定更新成功！", "success");
    showEditRemoteModal.value = false;
    await refreshAll();
  } catch (err) {
    showToast(`更新失敗: ${err}`, "error");
  } finally {
    editRemoteForm.isSaving = false;
  }
}

// Re-authenticate OAuth in Browser
async function handleReconnectRemote() {
  editRemoteForm.isReconnecting = true;
  try {
    showToast("已啟動瀏覽器授權登入頁面，請在瀏覽器中同意授權...", "info");
    const msg = await invoke("reconnect_remote_gui", {
      rclonePath: customRclonePath.value.trim() || null,
      name: editRemoteForm.name
    });
    showToast(msg || "重新登入授權成功！", "success");
    await refreshAll();
  } catch (err) {
    showToast(`重新授權失敗: ${err}`, "error");
  } finally {
    editRemoteForm.isReconnecting = false;
  }
}

// Delete Remote
async function handleDeleteRemote(name) {
  if (!confirm(`確定要刪除雲端設定【${name}】嗎？這不會刪除雲端上的檔案。`)) return;
  try {
    await invoke("delete_remote_gui", {
      rclonePath: customRclonePath.value.trim() || null,
      name
    });
    if (remoteConfigs[name]) {
      delete remoteConfigs[name];
      saveRemoteConfigs();
    }
    showToast(`已成功刪除 ${name}`, "info");
    await refreshAll();
  } catch (err) {
    showToast(`刪除失敗: ${err}`, "error");
  }
}

// Web-GUI Toggle & Browser Launch
async function toggleWebGui() {
  try {
    const nextState = !envStatus.value.webgui_running;
    const running = await invoke("toggle_rclone_webgui", {
      rclonePath: customRclonePath.value.trim() || null,
      enable: nextState
    });
    envStatus.value.webgui_running = running;
    showToast(running ? "已在背景啟動 Rclone 官方 Web-GUI！" : "已停止 Web-GUI 服務", "info");
  } catch (err) {
    showToast(`Web-GUI 操作失敗: ${err}`, "error");
  }
}

async function openWebGuiInBrowser() {
  if (!envStatus.value.webgui_running) {
    await toggleWebGui();
  }
  openUrl(envStatus.value.webgui_url);
}

// Multi-destination Helpers
function addExtraDestination() {
  if (!syncForm.extraDests) syncForm.extraDests = [];
  syncForm.extraDests.push({
    id: Date.now().toString() + Math.random().toString(36).slice(2, 6),
    path: ""
  });
}

function removeExtraDestination(index) {
  if (syncForm.extraDests && syncForm.extraDests[index] !== undefined) {
    syncForm.extraDests.splice(index, 1);
  }
}

function getAllDestinations() {
  const extras = (syncForm.extraDests || []).map((d) => (d.path || "").trim());
  const list = [syncForm.dest ? syncForm.dest.trim() : "", ...extras];
  return list.filter(Boolean);
}

// Browse Native Local Folder
async function pickLocalFolder(target = "source", extraIndex = null) {
  try {
    const selected = await invoke("select_local_folder");
    if (selected) {
      if (target === "source") {
        syncForm.source = selected;
      } else if (target === "extraDest" && extraIndex !== null) {
        if (syncForm.extraDests && syncForm.extraDests[extraIndex]) {
          syncForm.extraDests[extraIndex].path = selected;
        }
      } else {
        syncForm.dest = selected;
      }
      showToast(`已選取資料夾: ${selected}`, "success");
    }
  } catch (err) {
    showToast(`開啟資料夾選擇視窗失敗: ${err}`, "error");
  }
}

// Quick Select Remote Directly
function pickRemoteDirect(remoteName, target = "source", extraIndex = null) {
  const remotePath = `${remoteName}:`;
  if (target === "source") {
    syncForm.source = remotePath;
  } else if (target === "extraDest" && extraIndex !== null) {
    if (syncForm.extraDests && syncForm.extraDests[extraIndex]) {
      syncForm.extraDests[extraIndex].path = remotePath;
    }
  } else {
    syncForm.dest = remotePath;
  }
}

const cloudBrowseExtraIndex = ref(null);

// Cloud Directory Browser
async function openCloudBrowseModal(target = "source", extraIndex = null) {
  cloudBrowseTarget.value = target;
  cloudBrowseExtraIndex.value = extraIndex;
  let currentVal = "";
  if (target === "source") {
    currentVal = syncForm.source;
  } else if (target === "extraDest" && extraIndex !== null && syncForm.extraDests && syncForm.extraDests[extraIndex]) {
    currentVal = syncForm.extraDests[extraIndex].path;
  } else {
    currentVal = syncForm.dest;
  }
  if (currentVal && currentVal.includes(":") && !currentVal.startsWith("C:") && !currentVal.startsWith("D:")) {
    const parts = currentVal.split(":");
    cloudBrowseRemote.value = parts[0];
    cloudBrowseCurrentSubpath.value = parts.slice(1).join(":") || "";
  } else if (remotes.value.length > 0) {
    cloudBrowseRemote.value = remotes.value[0].name;
    cloudBrowseCurrentSubpath.value = "";
  } else {
    showToast("尚未建立任何雲端硬碟，請先在第一頁新增！", "error");
    return;
  }
  showCloudBrowseModal.value = true;
  await fetchCloudDirs();
}

async function fetchCloudDirs() {
  if (!cloudBrowseRemote.value) return;
  isLoadingCloudDirs.value = true;
  cloudBrowseDirs.value = [];
  try {
    const fullRemotePath = cloudBrowseCurrentSubpath.value
      ? `${cloudBrowseRemote.value}:${cloudBrowseCurrentSubpath.value}`
      : `${cloudBrowseRemote.value}:`;
    const dirs = await invoke("list_remote_dirs", {
      rclonePath: customRclonePath.value.trim() || null,
      remotePath: fullRemotePath
    });
    cloudBrowseDirs.value = dirs;
  } catch (err) {
    showToast(`讀取雲端目錄失敗: ${err}`, "error");
  } finally {
    isLoadingCloudDirs.value = false;
  }
}

function enterCloudSubdir(dirName) {
  if (cloudBrowseCurrentSubpath.value) {
    cloudBrowseCurrentSubpath.value = `${cloudBrowseCurrentSubpath.value}/${dirName}`;
  } else {
    cloudBrowseCurrentSubpath.value = dirName;
  }
  fetchCloudDirs();
}

function goCloudParentDir() {
  if (!cloudBrowseCurrentSubpath.value) return;
  const parts = cloudBrowseCurrentSubpath.value.split("/");
  parts.pop();
  cloudBrowseCurrentSubpath.value = parts.join("/");
  fetchCloudDirs();
}

function confirmCloudBrowseSelect() {
  const finalPath = cloudBrowseCurrentSubpath.value
    ? `${cloudBrowseRemote.value}:${cloudBrowseCurrentSubpath.value}`
    : `${cloudBrowseRemote.value}:`;
  if (cloudBrowseTarget.value === "source") {
    syncForm.source = finalPath;
  } else if (cloudBrowseTarget.value === "extraDest" && cloudBrowseExtraIndex.value !== null) {
    if (syncForm.extraDests && syncForm.extraDests[cloudBrowseExtraIndex.value]) {
      syncForm.extraDests[cloudBrowseExtraIndex.value].path = finalPath;
    }
  } else {
    syncForm.dest = finalPath;
  }
  showCloudBrowseModal.value = false;
  showToast(`已選取雲端路徑: ${finalPath}`, "success");
}

// Filtered Diff Items (RcloneView style)
const filteredDiffItems = computed(() => {
  if (!syncForm.diffResult || !syncForm.diffResult.items) return [];
  const keyword = syncForm.searchKeyword.trim().toLowerCase();
  return syncForm.diffResult.items.filter((item) => {
    // Status display toggle
    if (!syncForm.filterModes[item.status]) return false;
    // Keyword search
    if (keyword && !item.path.toLowerCase().includes(keyword)) return false;
    return true;
  });
});

function toggleFilterMode(mode) {
  syncForm.filterModes[mode] = !syncForm.filterModes[mode];
}

function toggleSelectAllVisible() {
  const visible = filteredDiffItems.value;
  const allSelected = visible.length > 0 && visible.every((item) => syncForm.selectedItems.has(item.path));
  if (allSelected) {
    visible.forEach((item) => syncForm.selectedItems.delete(item.path));
  } else {
    visible.forEach((item) => syncForm.selectedItems.add(item.path));
  }
}

function toggleItemSelect(path) {
  if (syncForm.selectedItems.has(path)) {
    syncForm.selectedItems.delete(path);
  } else {
    syncForm.selectedItems.add(path);
  }
}

// RcloneView Plus Compare / Check Diff
async function handleCheckDiff() {
  if (!syncForm.source || !syncForm.dest) {
    showToast("請先選擇或填寫來源與目標路徑！", "error");
    return;
  }
  syncForm.isChecking = true;
  syncForm.diffResult = null;
  syncForm.selectedItems.clear();

  const excludePatterns = syncForm.excludeFilter
    ? syncForm.excludeFilter.split(",").map((s) => s.trim()).filter(Boolean)
    : null;

  try {
    const res = await invoke("check_folder_diff", {
      rclonePath: customRclonePath.value.trim() || null,
      source: syncForm.source.trim(),
      dest: syncForm.dest.trim(),
      excludePatterns
    });
    syncForm.diffResult = res;
    showToast(res.message, "success");
  } catch (err) {
    showToast(`比對失敗: ${err}`, "error");
  } finally {
    syncForm.isChecking = false;
  }
}

// Copy Selected Items
async function handleCopySelected() {
  if (syncForm.selectedItems.size === 0) {
    showToast("請先勾選要複製的檔案項目！", "error");
    return;
  }
  syncForm.isCopyingSelected = true;
  try {
    const filesToCopy = Array.from(syncForm.selectedItems);
    const log = await invoke("copy_specific_files", {
      rclonePath: customRclonePath.value.trim() || null,
      source: syncForm.source.trim(),
      dest: syncForm.dest.trim(),
      files: filesToCopy
    });
    syncForm.taskLog = log;
    showToast(`成功複製 ${filesToCopy.length} 個檔案！`, "success");
    syncForm.selectedItems.clear();
    await handleCheckDiff();
  } catch (err) {
    showToast(`複製失敗: ${err}`, "error");
  } finally {
    syncForm.isCopyingSelected = false;
  }
}

// Copy Single File
async function handleCopySingleFile(filePath) {
  syncForm.isCopyingSelected = true;
  try {
    const log = await invoke("copy_specific_files", {
      rclonePath: customRclonePath.value.trim() || null,
      source: syncForm.source.trim(),
      dest: syncForm.dest.trim(),
      files: [filePath]
    });
    syncForm.taskLog = log;
    showToast(`成功複製: ${filePath}`, "success");
    await handleCheckDiff();
  } catch (err) {
    showToast(`複製失敗: ${err}`, "error");
  } finally {
    syncForm.isCopyingSelected = false;
  }
}

// Open Schedule Modal or Batch Create from Sync Page
function openScheduleFromSync() {
  const allDests = getAllDestinations();
  if (!syncForm.source || allDests.length === 0) {
    showToast("請先選擇或填寫來源與至少一個目的路徑！", "error");
    return;
  }
  const cleanSrc = syncForm.source.replace(/[\\/]+$/, "");
  const srcName = cleanSrc.split(/[\\/:]/).pop() || cleanSrc;

  if (allDests.length === 1) {
    const cleanDst = allDests[0].replace(/[\\/]+$/, "");
    const dstName = cleanDst.split(/[\\/:]/).pop() || cleanDst;
    newTask.name = `定時${syncForm.action === 'sync' ? '鏡像同步' : '增量備份'}: ${srcName} ➔ ${dstName}`;
    newTask.source = syncForm.source.trim();
    newTask.dest = allDests[0];
    newTask.action = syncForm.action;
    newTask.excludeFilter = syncForm.excludeFilter ? syncForm.excludeFilter.trim() : "";
    newTask.intervalMinutes = 60;
    newTask.enabled = true;
    showAddTaskModal.value = true;
  } else {
    let addedCount = 0;
    for (const d of allDests) {
      const cleanDst = d.replace(/[\\/]+$/, "");
      const dstName = cleanDst.split(/[\\/:]/).pop() || cleanDst;
      scheduledTasks.value.push({
        id: Date.now().toString() + Math.random().toString(36).slice(2, 5),
        name: `定時${syncForm.action === 'sync' ? '同步' : '備份'}: ${srcName} ➔ ${dstName}`,
        source: syncForm.source.trim(),
        dest: d,
        action: syncForm.action,
        excludeFilter: syncForm.excludeFilter ? syncForm.excludeFilter.trim() : "",
        intervalMinutes: 60,
        enabled: true,
        lastRun: "從未執行",
        status: "待命"
      });
      addedCount++;
    }
    saveScheduledTasks();
    showToast(`已成功為 ${addedCount} 個目的地建立背景排程任務！`, "success");
    activeTab.value = "scheduler";
  }
}

// RcloneView Plus Run Sync Job (Supports Multi-Destination)
async function handleRunSync() {
  const allDests = getAllDestinations();
  if (!syncForm.source || allDests.length === 0) {
    showToast("請先選擇或填寫來源與至少一個目的路徑！", "error");
    return;
  }
  syncForm.isRunning = true;
  syncForm.taskLog = `正在執行同步任務中（共 ${allDests.length} 個目的地）...`;

  const excludePatterns = syncForm.excludeFilter
    ? syncForm.excludeFilter.split(",").map((s) => s.trim()).filter(Boolean)
    : null;

  let successCount = 0;
  let failCount = 0;
  const logs = [];

  try {
    for (let i = 0; i < allDests.length; i++) {
      const curDest = allDests[i];
      const prefix = `[${i + 1}/${allDests.length}]`;
      showToast(`正在傳輸至目的地 ${prefix}: ${curDest}...`, "info");
      logs.push(`==================================================`);
      logs.push(`${prefix} 開始同步: ${syncForm.source.trim()} ➔ ${curDest}`);
      logs.push(`==================================================`);
      syncForm.taskLog = logs.join("\n");

      try {
        const log = await invoke("run_sync_task", {
          rclonePath: customRclonePath.value.trim() || null,
          action: syncForm.action,
          source: syncForm.source.trim(),
          dest: curDest,
          excludePatterns
        });
        logs.push(log || "任務執行完成！");
        successCount++;
      } catch (destErr) {
        logs.push(`❌ 此目的地同步失敗: ${destErr}`);
        failCount++;
      }
      syncForm.taskLog = logs.join("\n");
    }

    if (failCount === 0) {
      showToast(`同步/備份全數成功！共傳輸至 ${successCount} 個目的地！`, "success");
    } else {
      showToast(`同步完成：${successCount} 個成功，${failCount} 個失敗，請檢視日誌！`, "warning");
    }

    // 若勾選自動加入排程，為所有目的地加入背景定時排程
    if (syncForm.autoAddToScheduler) {
      let addedCount = 0;
      for (const curDest of allDests) {
        const cleanSrc = syncForm.source.replace(/[\\/]+$/, "");
        const cleanDst = curDest.replace(/[\\/]+$/, "");
        const srcName = cleanSrc.split(/[\\/:]/).pop() || cleanSrc;
        const dstName = cleanDst.split(/[\\/:]/).pop() || cleanDst;

        const exists = scheduledTasks.value.some(
          (t) => t.source === syncForm.source.trim() && t.dest === curDest
        );
        if (!exists) {
          const task = {
            id: Date.now().toString() + Math.random().toString(36).slice(2, 5),
            name: `自動${syncForm.action === 'sync' ? '同步' : '備份'}: ${srcName} ➔ ${dstName}`,
            source: syncForm.source.trim(),
            dest: curDest,
            action: syncForm.action,
            excludeFilter: syncForm.excludeFilter ? syncForm.excludeFilter.trim() : "",
            intervalMinutes: 60,
            enabled: true,
            lastRun: new Date().toLocaleTimeString(),
            lastRunTs: Date.now(),
            status: "成功完成"
          };
          scheduledTasks.value.push(task);
          addedCount++;
        }
      }
      if (addedCount > 0) {
        saveScheduledTasks();
        showToast(`已為您將 ${addedCount} 個目的地自動加入背景定時排程！`, "success");
      }
    }

    await handleCheckDiff();
  } catch (err) {
    syncForm.taskLog = `整體任務錯誤: ${err}`;
    showToast(`任務失敗: ${err}`, "error");
  } finally {
    syncForm.isRunning = false;
  }
}

// Scheduler: Add Task
function submitAddTask() {
  if (!newTask.name.trim() || !newTask.source.trim() || !newTask.dest.trim()) {
    showToast("請填寫完整的任務資訊！", "error");
    return;
  }
  const task = {
    id: Date.now().toString(),
    name: newTask.name.trim(),
    source: newTask.source.trim(),
    dest: newTask.dest.trim(),
    action: newTask.action,
    excludeFilter: newTask.excludeFilter ? newTask.excludeFilter.trim() : "",
    intervalMinutes: Number(newTask.intervalMinutes) || 60,
    enabled: true,
    lastRun: "從未執行",
    status: "待命"
  };
  scheduledTasks.value.push(task);
  saveScheduledTasks();
  showAddTaskModal.value = false;
  newTask.name = "";
  newTask.source = "";
  newTask.dest = "";
  newTask.excludeFilter = "";
  showToast("已成功建立排程任務！程式常駐右下角時將自動定時執行", "success");
}

function removeTask(id) {
  scheduledTasks.value = scheduledTasks.value.filter((t) => t.id !== id);
  saveScheduledTasks();
  showToast("已刪除任務", "info");
}

function toggleTaskEnabled(task) {
  task.enabled = !task.enabled;
  saveScheduledTasks();
}

function openEditTaskModal(task) {
  editTaskForm.id = task.id;
  editTaskForm.name = task.name || "";
  editTaskForm.source = task.source || "";
  editTaskForm.dest = task.dest || "";
  editTaskForm.action = task.action || "copy";
  editTaskForm.excludeFilter = task.excludeFilter || "";
  editTaskForm.intervalMinutes = task.intervalMinutes || 60;
  editTaskForm.enabled = task.enabled !== false;
  showEditTaskModal.value = true;
}

function submitEditTask() {
  if (!editTaskForm.name.trim() || !editTaskForm.source.trim() || !editTaskForm.dest.trim()) {
    showToast("請填寫完整的任務資訊！", "error");
    return;
  }
  const task = scheduledTasks.value.find((t) => t.id === editTaskForm.id);
  if (!task) {
    showToast("找不到對應的排程任務！", "error");
    return;
  }
  task.name = editTaskForm.name.trim();
  task.source = editTaskForm.source.trim();
  task.dest = editTaskForm.dest.trim();
  task.action = editTaskForm.action;
  task.excludeFilter = editTaskForm.excludeFilter ? editTaskForm.excludeFilter.trim() : "";
  task.intervalMinutes = Math.max(1, Number(editTaskForm.intervalMinutes) || 60);
  task.enabled = editTaskForm.enabled;
  saveScheduledTasks();
  showEditTaskModal.value = false;
  showToast("已成功更新排程任務設定！", "success");
}

async function executeTaskNow(task) {
  if (task.status === "執行中...") {
    showToast("任務正在執行中，請稍候...", "info");
    return;
  }
  task.status = "執行中...";
  task.lastRun = new Date().toLocaleTimeString();
  task.lastRunTs = Date.now();
  saveScheduledTasks();
  showToast(`開始執行任務「${task.name}」...`, "info");
  try {
    const excludePatterns = task.excludeFilter
      ? task.excludeFilter.split(",").map((s) => s.trim()).filter(Boolean)
      : null;
    await invoke("run_sync_task", {
      rclonePath: customRclonePath.value.trim() || null,
      action: task.action,
      source: task.source,
      dest: task.dest,
      excludePatterns
    });
    task.status = "成功完成";
    saveScheduledTasks();
    showToast(`排程任務「${task.name}」執行成功！`, "success");
  } catch (err) {
    task.status = "執行失敗";
    saveScheduledTasks();
    showToast(`任務「${task.name}」失敗: ${err}`, "error");
  }
}

function saveScheduledTasks() {
  localStorage.setItem("rclone_scheduled_tasks", JSON.stringify(scheduledTasks.value));
}

// Scheduler cycle
function runSchedulerCycle() {
  const now = Date.now();
  scheduledTasks.value.forEach(async (task) => {
    if (!task.enabled) return;
    const intervalMs = (task.intervalMinutes || 60) * 60 * 1000;
    const lastRunTs = task.lastRunTs || 0;
    if (now - lastRunTs >= intervalMs) {
      task.status = "執行中...";
      task.lastRunTs = now;
      task.lastRun = new Date().toLocaleTimeString();
      try {
        const excludePatterns = task.excludeFilter
          ? task.excludeFilter.split(",").map((s) => s.trim()).filter(Boolean)
          : null;
        await invoke("run_sync_task", {
          rclonePath: customRclonePath.value.trim() || null,
          action: task.action,
          source: task.source,
          dest: task.dest,
          excludePatterns
        });
        task.status = "成功完成";
      } catch (err) {
        task.status = "執行失敗";
      }
      saveScheduledTasks();
    }
  });
}

function toggleRemoteAutoMount(remoteName) {
  const conf = remoteConfigs[remoteName];
  if (!conf) return;
  conf.autoMount = !conf.autoMount;
  let list = [...autoMountList.value];
  if (conf.autoMount) {
    if (!list.includes(remoteName)) list.push(remoteName);
  } else {
    list = list.filter((n) => n !== remoteName);
  }
  autoMountList.value = list;
  localStorage.setItem("rclone_auto_mount_list", JSON.stringify(list));
  saveRemoteConfigs();
}

function saveCustomPath() {
  localStorage.setItem("rclone_custom_path", customRclonePath.value);
  showToast("已儲存 Rclone 路徑設定！", "success");
  refreshAll();
}

const isInstallingRclone = ref(false);
const isInstallingWinFsp = ref(false);

async function handleAutoInstallRclone() {
  if (isInstallingRclone.value) return;
  isInstallingRclone.value = true;
  showToast("正在自官方下載並配置 Rclone 最新版，請稍候...", "info");
  try {
    const installedPath = await invoke("auto_install_rclone");
    customRclonePath.value = installedPath;
    localStorage.setItem("rclone_custom_path", installedPath);
    showToast(`Rclone 自動配置成功！已安裝於：${installedPath}`, "success");
    await refreshAll();
  } catch (err) {
    showToast(`自動安裝 Rclone 失敗: ${err}`, "error");
  } finally {
    isInstallingRclone.value = false;
  }
}

async function handleAutoInstallWinFsp() {
  if (isInstallingWinFsp.value) return;
  isInstallingWinFsp.value = true;
  showToast("正在下載 WinFsp 官方安裝程式，下載完成後將自動啟動安裝精靈...", "info");
  try {
    await invoke("auto_install_winfsp");
    showToast("WinFsp 安裝精靈已啟動！請於 Windows 安裝精靈中點擊下一步完成安裝。", "success");
    await refreshAll();
  } catch (err) {
    showToast(`下載或安裝 WinFsp 失敗: ${err}`, "error");
  } finally {
    isInstallingWinFsp.value = false;
  }
}

async function handleAutoInstallAll() {
  if (!envStatus.value.rclone_found) {
    await handleAutoInstallRclone();
  }
  if (!envStatus.value.winfsp_found) {
    await handleAutoInstallWinFsp();
  }
}

const mountedCount = computed(() => remotes.value.filter((r) => r.is_mounted).length);
const totalCount = computed(() => remotes.value.length);
const isPrerequisiteMissing = computed(() => !envStatus.value.rclone_found || !envStatus.value.winfsp_found);

onMounted(async () => {
  applyTheme(isLightMode.value);
  await checkAutostart();
  await refreshAll(true);
  schedulerTimer = setInterval(runSchedulerCycle, 60000);
  setTimeout(() => {
    checkForUpdates(false);
  }, 2500);
});

onUnmounted(() => {
  if (schedulerTimer) clearInterval(schedulerTimer);
});
</script>

<template>
  <div class="app-layout" :data-theme="isLightMode ? 'light' : 'dark'">
    <!-- Header -->
    <header class="app-header">
      <div class="brand">
        <div class="logo-box">
          <HardDrive class="logo-icon" />
          <span class="logo-pulse"></span>
        </div>
        <div>
          <h1 class="brand-title">Rclone Drive & Sync</h1>
          <p class="brand-subtitle">雲端硬碟掛載 • 參數修改 • 差異比對 • 定時排程</p>
        </div>
      </div>

      <!-- Navigation Tabs -->
      <nav class="nav-tabs">
        <button
          class="tab-btn"
          :class="{ active: activeTab === 'mounts' }"
          @click="activeTab = 'mounts'"
        >
          <HardDrive class="tab-icon" />
          <span>硬碟掛載 (RaiDrive)</span>
        </button>

        <button
          class="tab-btn"
          :class="{ active: activeTab === 'sync' }"
          @click="activeTab = 'sync'"
        >
          <ArrowRightLeft class="tab-icon" />
          <span>同步與比對 (Sync & Diff)</span>
        </button>

        <button
          class="tab-btn"
          :class="{ active: activeTab === 'scheduler' }"
          @click="activeTab = 'scheduler'"
        >
          <CalendarClock class="tab-icon" />
          <span>排程任務 ({{ scheduledTasks.length }})</span>
        </button>

        <button
          class="tab-btn"
          :class="{ active: activeTab === 'webgui' }"
          @click="activeTab = 'webgui'"
        >
          <Globe class="tab-icon" />
          <span>Rclone Web-GUI</span>
        </button>
      </nav>

      <!-- Global Actions -->
      <div class="header-actions">
        <button
          class="btn btn-secondary"
          @click="checkForUpdates(true)"
          :disabled="isCheckingUpdate"
          title="檢查 GitHub 最新版本"
        >
          <DownloadCloud class="btn-icon" :class="{ 'spin-anim': isCheckingUpdate }" />
          <span class="btn-text-hide-sm">檢查更新</span>
        </button>

        <button class="btn btn-secondary" @click="toggleTheme" :title="isLightMode ? '切換為深色主題' : '切換為淺色主題'">
          <Sun v-if="isLightMode" class="btn-icon text-amber" />
          <Moon v-else class="btn-icon" />
        </button>

        <button class="btn btn-secondary" @click="refreshAll(false)" :disabled="isRefreshing" title="重新整理">
          <RefreshCw class="btn-icon" :class="{ 'spin-anim': isRefreshing }" />
        </button>

        <button class="btn btn-secondary" @click="showSettings = true" title="偏好設定">
          <Settings class="btn-icon" />
        </button>
      </div>
    </header>

    <!-- Missing Prerequisites Warning Banner -->
    <div v-if="isPrerequisiteMissing" class="prereq-alert-banner">
      <div class="prereq-content">
        <div class="prereq-header">
          <AlertTriangle class="prereq-warn-icon" />
          <span class="prereq-title">系統偵測到缺少必要元件，雲端硬碟掛載功能需要以下工具：</span>
          <button
            v-if="!envStatus.rclone_found && !envStatus.winfsp_found"
            class="btn btn-emerald btn-sm ml-auto"
            :disabled="isInstallingRclone || isInstallingWinFsp"
            @click="handleAutoInstallAll"
          >
            <Zap class="btn-icon" />
            <span>⚡ 一鍵自動安裝全部必要元件</span>
          </button>
        </div>

        <div class="prereq-cards-row">
          <!-- Rclone Missing Card -->
          <div v-if="!envStatus.rclone_found" class="prereq-item-card">
            <div class="prereq-card-text">
              <span class="prereq-badge">必要元件 1</span>
              <h4>Rclone 核心執行檔</h4>
              <p>預設檢查路徑 <code>C:\rclone\rclone.exe</code> 尚未找到執行檔。</p>
            </div>
            <div class="prereq-btn-group">
              <button
                class="btn btn-download"
                :disabled="isInstallingRclone"
                @click="handleAutoInstallRclone"
              >
                <DownloadCloud class="btn-icon" :class="{ 'spin-anim': isInstallingRclone }" />
                <span>{{ isInstallingRclone ? '正在自動下載配置中...' : '⚡ 一鍵自動下載配置 Rclone' }}</span>
              </button>
              <button class="btn btn-download-alt" @click="openUrl('https://rclone.org/downloads/')">
                <ExternalLink class="btn-icon" />
                <span>官網手動下載</span>
              </button>
            </div>
          </div>

          <!-- WinFsp Missing Card -->
          <div v-if="!envStatus.winfsp_found" class="prereq-item-card">
            <div class="prereq-card-text">
              <span class="prereq-badge">必要元件 2</span>
              <h4>WinFsp 檔案系統核心</h4>
              <p>Windows 虛擬檔案系統驅動，未安裝將無法建立本機磁碟代號。</p>
            </div>
            <div class="prereq-btn-group">
              <button
                class="btn btn-download"
                :disabled="isInstallingWinFsp"
                @click="handleAutoInstallWinFsp"
              >
                <DownloadCloud class="btn-icon" :class="{ 'spin-anim': isInstallingWinFsp }" />
                <span>{{ isInstallingWinFsp ? '正在下載並啟動安裝程式...' : '⚡ 一鍵下載安裝 WinFsp' }}</span>
              </button>
              <button class="btn btn-download-alt" @click="openUrl('https://winfsp.dev/')">
                <ExternalLink class="btn-icon" />
                <span>官網手動下載</span>
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- Quick Status Bar -->
    <div class="env-banner">
      <div class="env-item" :class="{ 'env-ok': envStatus.rclone_found, 'env-warn': !envStatus.rclone_found }">
        <component :is="envStatus.rclone_found ? CheckCircle2 : AlertTriangle" class="env-icon" />
        <span v-if="envStatus.rclone_found">Rclone: {{ envStatus.rclone_version || '已連線' }}</span>
        <span v-else class="clickable-link" @click="handleAutoInstallRclone" title="點擊直接背景下載並自動配置 Rclone">
          Rclone: 尚未安裝 (⚡ 點此一鍵安裝)
        </span>
      </div>

      <div class="env-item" :class="{ 'env-ok': envStatus.winfsp_found, 'env-warn': !envStatus.winfsp_found }">
        <component :is="envStatus.winfsp_found ? ShieldCheck : AlertTriangle" class="env-icon" />
        <span v-if="envStatus.winfsp_found">WinFsp: 已就緒 (正常運作)</span>
        <span v-else class="clickable-link" @click="handleAutoInstallWinFsp" title="點擊直接下載 WinFsp 並啟動安裝精靈">
          WinFsp: 尚未安裝 (⚡ 點此一鍵安裝)
        </span>
      </div>

      <div class="env-item" :class="{ 'env-ok': envStatus.webgui_running, 'env-neutral': !envStatus.webgui_running }">
        <Globe class="env-icon" />
        <span>Web-GUI: {{ envStatus.webgui_running ? '127.0.0.1:5572 (運行中)' : '已停止' }}</span>
      </div>

      <div class="env-item env-neutral" v-if="autostartActive">
        <Check class="env-icon text-emerald" />
        <span>開機自啟動: 已啟用</span>
      </div>

      <div class="env-item env-neutral ml-auto">
        <span class="count-pill">
          <span class="indicator-dot active"></span>
          已掛載: {{ mountedCount }} / {{ totalCount }}
        </span>
      </div>
    </div>

    <!-- Main Content Area -->
    <main class="main-content">
      <!-- TAB 1: MOUNTS (RaiDrive Clone) -->
      <section v-if="activeTab === 'mounts'" class="tab-pane">
        <!-- Sub Bar -->
        <div class="sub-bar">
          <div class="sub-title-group">
            <h2>雲端硬碟掛載列表</h2>
            <p>已掛載為標準 Windows 網路磁碟機，支援隨選即用與在線讀寫，點選右上角 ✏️ 可直接修正設定與重新授權</p>
          </div>
          <div class="sub-actions">
            <button class="btn btn-primary" @click="showAddRemoteModal = true">
              <PlusCircle class="btn-icon" />
              <span>新增雲端硬碟</span>
            </button>
            <button class="btn btn-secondary" @click="mountAll" :disabled="mountedCount === totalCount || totalCount === 0 || isPrerequisiteMissing">
              <Disc3 class="btn-icon" />
              <span>掛載全部</span>
            </button>
            <button class="btn btn-danger-outline" @click="unmountAll" :disabled="mountedCount === 0">
              <Power class="btn-icon" />
              <span>卸載全部</span>
            </button>
          </div>
        </div>

        <!-- Cards Grid -->
        <div v-if="remotes.length > 0" class="cards-grid">
          <div
            v-for="remote in remotes"
            :key="remote.name"
            class="drive-card"
            :class="{ 'card-mounted': remote.is_mounted, 'card-loading': loadingRemotes.has(remote.name) }"
          >
            <!-- Card Header -->
            <div class="card-header">
              <div class="provider-badge" :style="{ backgroundColor: getProviderDetails(remote.remote_type).bg }">
                <Cloud class="provider-icon" :style="{ color: getProviderDetails(remote.remote_type).color }" />
              </div>

              <div class="remote-meta">
                <div class="remote-name" :title="remote.name">{{ remote.name }}</div>
                <div class="remote-type-tag">
                  {{ getProviderDetails(remote.remote_type).name }}
                </div>
              </div>

              <div class="mount-status-badge" :class="remote.is_mounted ? 'status-mounted' : 'status-unmounted'">
                <span class="status-dot"></span>
                {{ remote.is_mounted ? `已掛載 (${remote.mounted_drive})` : '未掛載' }}
              </div>

              <!-- Edit & Delete Buttons -->
              <div class="card-header-actions" v-if="!remote.is_mounted">
                <button
                  class="icon-btn-action"
                  @click="openEditModal(remote)"
                  title="修改此雲端硬碟設定"
                >
                  <Pencil class="action-icon" />
                </button>
                <button
                  class="icon-btn-delete"
                  @click="handleDeleteRemote(remote.name)"
                  title="刪除此雲端設定"
                >
                  <Trash2 class="trash-icon" />
                </button>
              </div>
            </div>

            <!-- Card Body / Settings -->
            <div class="card-body">
              <!-- Drive Letter -->
              <div class="setting-row">
                <label class="field-label">磁碟機代號</label>
                <div class="drive-select-wrapper">
                  <select
                    v-if="!remote.is_mounted"
                    v-model="remoteConfigs[remote.name].driveLetter"
                    @change="saveRemoteConfigs"
                    class="drive-select"
                  >
                    <option v-for="letter in availableDrives" :key="letter" :value="letter">
                      磁碟機 {{ letter }}
                    </option>
                    <option v-if="!availableDrives.includes(remoteConfigs[remote.name]?.driveLetter)" :value="remoteConfigs[remote.name]?.driveLetter">
                      磁碟機 {{ remoteConfigs[remote.name]?.driveLetter }}
                    </option>
                  </select>
                  <div v-else class="drive-locked">
                    <HardDrive class="drive-letter-icon" />
                    <span>磁碟機 {{ remote.mounted_drive }}</span>
                  </div>
                </div>
              </div>

              <!-- Volume Name -->
              <div class="setting-row" v-if="!remote.is_mounted">
                <label class="field-label">硬碟顯示名稱</label>
                <input
                  type="text"
                  v-model="remoteConfigs[remote.name].volname"
                  @change="saveRemoteConfigs"
                  @blur="saveRemoteConfigs"
                  class="field-input"
                  placeholder="檔案總管中顯示的名稱"
                />
              </div>

              <!-- Cache Mode -->
              <div class="setting-row" v-if="!remote.is_mounted">
                <label class="field-label">快取模式</label>
                <select
                  v-model="remoteConfigs[remote.name].cacheMode"
                  @change="saveRemoteConfigs"
                  class="field-select"
                >
                  <option value="full">Full (推薦, 支援 Office/多數軟體)</option>
                  <option value="writes">Writes (僅寫入快取)</option>
                  <option value="minimal">Minimal (最低快取)</option>
                  <option value="off">關閉 (唯讀或純串流)</option>
                </select>
              </div>

              <div class="checkbox-row" @click="toggleRemoteAutoMount(remote.name)">
                <div class="custom-checkbox" :class="{ checked: remoteConfigs[remote.name]?.autoMount }">
                  <Check v-if="remoteConfigs[remote.name]?.autoMount" class="check-icon" />
                </div>
                <span class="checkbox-label">開機自動掛載此磁碟</span>
              </div>
            </div>

            <!-- Card Footer -->
            <div class="card-footer">
              <template v-if="remote.is_mounted">
                <button
                  class="btn btn-emerald"
                  @click="openExplorer(remote.mounted_drive)"
                  title="開啟磁碟機"
                >
                  <FolderOpen class="btn-icon" />
                  <span>開啟檔案總管</span>
                </button>

                <button
                  class="btn btn-danger"
                  @click="unmountDrive(remote.name)"
                  :disabled="loadingRemotes.has(remote.name)"
                  title="卸載此磁碟機"
                >
                  <Power class="btn-icon" />
                  <span>{{ loadingRemotes.has(remote.name) ? '正在卸載...' : '卸載' }}</span>
                </button>
              </template>

              <template v-else>
                <button
                  class="btn btn-mount-primary"
                  @click="mountDrive(remote.name)"
                  :disabled="loadingRemotes.has(remote.name) || isPrerequisiteMissing"
                >
                  <Disc3 class="btn-icon" :class="{ 'spin-anim': loadingRemotes.has(remote.name) }" />
                  <span>{{ loadingRemotes.has(remote.name) ? '正在掛載...' : '立即掛載' }}</span>
                </button>
              </template>
            </div>
          </div>
        </div>

        <!-- Empty State -->
        <div v-else class="empty-state">
          <div class="empty-icon-box">
            <Cloud class="empty-icon" />
          </div>
          <h2>尚未設定任何雲端空間</h2>
          <p>點選下方按鈕直接透過圖形化引導新增您的第一個 Google Drive、OneDrive 或 WebDAV。</p>
          <button class="btn btn-primary btn-lg" @click="showAddRemoteModal = true">
            <PlusCircle class="btn-icon" />
            <span>新增雲端硬碟</span>
          </button>
        </div>
      </section>

      <!-- TAB 2: SYNC & COMPARE (RcloneView Plus) -->
      <section v-if="activeTab === 'sync'" class="tab-pane">
        <div class="sub-bar">
          <div class="sub-title-group">
            <h2>檔案同步與差異比對 (RcloneView Plus)</h2>
            <p>設定來源與目的路徑，執行雙向內容比對，支援狀態篩選 (Display Filter)、單檔複製、選取傳輸與定時排程</p>
          </div>
        </div>

        <!-- Upper: Task Setup Card (上下排列之上方卡片) -->
        <div class="glass-card setup-card-compact">
          <div class="setup-compact-header">
            <div class="setup-compact-title">
              <FolderSync class="card-title-icon" />
              <span>任務路徑與參數設定</span>
            </div>
            <div class="setup-compact-actions">
              <button
                class="btn btn-secondary"
                @click="handleCheckDiff"
                :disabled="syncForm.isChecking || syncForm.isRunning || isPrerequisiteMissing"
              >
                <FileCheck class="btn-icon" :class="{ 'spin-anim': syncForm.isChecking }" />
                <span>{{ syncForm.isChecking ? '比對中...' : '🔍 比對兩端差異 (Check Diff)' }}</span>
              </button>

              <button
                class="btn btn-primary"
                @click="handleRunSync"
                :disabled="syncForm.isRunning || syncForm.isChecking || isPrerequisiteMissing"
              >
                <Play class="btn-icon" :class="{ 'spin-anim': syncForm.isRunning }" />
                <span>{{ syncForm.isRunning ? '執行中...' : '⚡ 立即開始同步/備份' }}</span>
              </button>

              <button
                class="btn btn-schedule"
                @click="openScheduleFromSync"
                :disabled="!syncForm.source || !syncForm.dest"
                title="直接將此任務設定儲存為背景定時排程"
              >
                <CalendarClock class="btn-icon" />
                <span>🕒 直接加入排程任務</span>
              </button>
            </div>
          </div>

          <!-- Path Inputs Row -->
          <div class="setup-inputs-grid">
            <!-- Source Input -->
            <div class="form-group-compact">
              <div class="field-label-row">
                <label class="field-label">來源路徑 (Source)</label>
                <div class="quick-links-group" v-if="remotes.length > 0">
                  <span class="quick-link-label">快速雲端:</span>
                  <button
                    v-for="r in remotes.slice(0, 3)"
                    :key="r.name"
                    class="btn-tag"
                    @click="pickRemoteDirect(r.name, 'source')"
                    :title="`將來源設定為 ${r.name}:`"
                  >
                    {{ r.name }}:
                  </button>
                </div>
              </div>
              <div class="input-with-actions">
                <input
                  type="text"
                  v-model="syncForm.source"
                  class="modal-input"
                  placeholder="例如：C:\同步資料夾 或 GDrive:Documents"
                />
                <button
                  class="btn btn-browse"
                  @click="pickLocalFolder('source')"
                  title="從本機檔案總管瀏覽選擇資料夾"
                >
                  <FolderOpen class="btn-icon" />
                  <span>瀏覽本機</span>
                </button>
                <button
                  class="btn btn-browse-cloud"
                  @click="openCloudBrowseModal('source')"
                  title="選擇並深入瀏覽已建立的雲端硬碟目錄"
                >
                  <Cloud class="btn-icon" />
                  <span>選擇雲端</span>
                </button>
              </div>
            </div>

            <!-- Destination Column (Right Column: Primary + Extras + Add Button) -->
            <div class="destinations-col">
              <!-- Destination Input (Primary) -->
              <div class="form-group-compact">
                <div class="field-label-row">
                  <div class="dest-label-with-tag">
                    <label class="field-label">目的路徑 1 (主要 Destination)</label>
                    <span class="badge-tag">主要</span>
                  </div>
                  <div class="quick-links-group" v-if="remotes.length > 0">
                    <span class="quick-link-label">快速雲端:</span>
                    <button
                      v-for="r in remotes.slice(0, 3)"
                      :key="r.name"
                      class="btn-tag"
                      @click="pickRemoteDirect(r.name, 'dest')"
                      :title="`將目的設定為 ${r.name}:`"
                    >
                      {{ r.name }}:
                    </button>
                  </div>
                </div>
                <div class="input-with-actions">
                  <input
                    type="text"
                    v-model="syncForm.dest"
                    class="modal-input"
                    placeholder="例如：D:\Backup 或 GDrive:Backup"
                  />
                  <button
                    class="btn btn-browse"
                    @click="pickLocalFolder('dest')"
                    title="從本機檔案總管瀏覽選擇資料夾"
                  >
                    <FolderOpen class="btn-icon" />
                    <span>瀏覽本機</span>
                  </button>
                  <button
                    class="btn btn-browse-cloud"
                    @click="openCloudBrowseModal('dest')"
                    title="選擇並深入瀏覽已建立的雲端硬碟目錄"
                  >
                    <Cloud class="btn-icon" />
                    <span>選擇雲端</span>
                  </button>
                </div>
              </div>

              <!-- Extra Destinations (Multi-Destination Support) -->
              <div
                v-for="(extra, idx) in syncForm.extraDests"
                :key="extra.id"
                class="form-group-compact extra-dest-group"
              >
                <div class="field-label-row">
                  <div class="dest-label-with-tag">
                    <label class="field-label">目的路徑 {{ idx + 2 }} (其他遠端/本機資料夾)</label>
                    <span class="badge-tag badge-cyan">額外</span>
                  </div>
                  <div class="quick-links-group" v-if="remotes.length > 0">
                    <span class="quick-link-label">快速雲端:</span>
                    <button
                      v-for="r in remotes.slice(0, 3)"
                      :key="r.name"
                      class="btn-tag"
                      @click="pickRemoteDirect(r.name, 'extraDest', idx)"
                      :title="`將此目的設定為 ${r.name}:`"
                    >
                      {{ r.name }}:
                    </button>
                  </div>
                </div>
                <div class="input-with-actions">
                  <input
                    type="text"
                    v-model="extra.path"
                    class="modal-input"
                    :placeholder="`例如：E:\\Backup 或 OneDrive:Backup${idx + 2}`"
                  />
                  <button
                    class="btn btn-browse"
                    @click="pickLocalFolder('extraDest', idx)"
                    title="從本機檔案總管瀏覽選擇資料夾"
                  >
                    <FolderOpen class="btn-icon" />
                    <span>瀏覽本機</span>
                  </button>
                  <button
                    class="btn btn-browse-cloud"
                    @click="openCloudBrowseModal('extraDest', idx)"
                    title="選擇並深入瀏覽已建立的雲端硬碟目錄"
                  >
                    <Cloud class="btn-icon" />
                    <span>選擇雲端</span>
                  </button>
                  <button
                    class="btn btn-danger-outline"
                    @click="removeExtraDestination(idx)"
                    title="移除此目的地"
                  >
                    <Trash2 class="btn-icon" />
                    <span>移除</span>
                  </button>
                </div>
              </div>

              <!-- Add Destination Button Action Row (Directly below destination inputs) -->
              <div class="add-destination-row">
                <button
                  type="button"
                  class="btn btn-add-dest"
                  @click="addExtraDestination"
                >
                  <FolderPlus class="btn-icon" />
                  <span>＋ 新增目的地 (Add Destination)</span>
                </button>
                <span class="dest-count-hint" v-if="syncForm.extraDests && syncForm.extraDests.length > 0">
                  目前共設定 {{ (syncForm.extraDests ? syncForm.extraDests.length : 0) + 1 }} 個同步目的地，執行時將依序傳輸至各目標位置。
                </span>
              </div>
            </div>
          </div>

          <!-- Secondary Options Row (Action, Filters, Auto-Schedule) -->
          <div class="setup-options-row">
            <div class="option-item flex-2">
              <label class="field-label-compact">任務模式</label>
              <select v-model="syncForm.action" class="modal-input-compact">
                <option value="copy">Copy (增量複製備份: 推薦！僅複製新檔/變更檔，不刪除目的端檔案)</option>
                <option value="sync">Sync (單向鏡像同步: 目的端檔案會完全與來源一致)</option>
                <option value="move">Move (移動傳輸: 檔案成功複製到目的端後自來源端刪除)</option>
              </select>
            </div>

            <div class="option-item flex-3">
              <label class="field-label-compact">排除過濾 (Filter / Exclude)</label>
              <input
                type="text"
                v-model="syncForm.excludeFilter"
                class="modal-input-compact"
                placeholder="例如：*.tmp, *.bak, thumbs.db, node_modules/**"
              />
            </div>

            <div class="option-item flex-2 checkbox-center" @click="syncForm.autoAddToScheduler = !syncForm.autoAddToScheduler">
              <div class="custom-checkbox" :class="{ checked: syncForm.autoAddToScheduler }">
                <Check v-if="syncForm.autoAddToScheduler" class="check-icon" />
              </div>
              <span class="checkbox-label text-cyan font-medium text-xs">
                同步成功後自動加入定時排程 (每 60 分鐘)
              </span>
            </div>
          </div>
        </div>

        <!-- Lower: RcloneView Full Width Compare Card (上下排列之全寬比對檢視視窗) -->
        <div class="glass-card rcloneview-main-card">
          <!-- Top Header: Compare Status & Reload -->
          <div class="rcloneview-header">
            <div class="rcloneview-title-group">
              <h3 class="rcloneview-title">Compare (比對檢視視窗)</h3>
              <span v-if="syncForm.diffResult" class="rcloneview-count-badge">
                All data retrieved ({{ syncForm.diffResult.items ? syncForm.diffResult.items.length : 0 }} / {{ syncForm.diffResult.items ? syncForm.diffResult.items.length : 0 }} compared)
              </span>
            </div>
            <div class="rcloneview-header-right">
              <button
                v-if="syncForm.diffResult"
                class="btn btn-xs btn-schedule-outline"
                @click="openScheduleFromSync"
                title="比對無誤，直接將此任務建立為背景定時排程"
              >
                <CalendarClock class="btn-icon-xs" />
                <span>建立為排程任務</span>
              </button>
              <button
                class="btn btn-xs btn-outline"
                @click="handleCheckDiff"
                :disabled="syncForm.isChecking || !syncForm.source || !syncForm.dest"
                title="重新比對兩端資料夾內容"
              >
                <RefreshCw class="btn-icon-xs" :class="{ 'spin-anim': syncForm.isChecking }" />
                <span>{{ syncForm.isChecking ? '比對中...' : 'Reload (重新比對)' }}</span>
              </button>
            </div>
          </div>

          <!-- RcloneView Blue Path Indicator Bar (如 RcloneView 截圖之頂部路徑條) -->
          <div class="rcloneview-path-header">
            <div class="path-col path-src" :title="syncForm.source || '未設定來源路徑'">
              <span class="path-star">⭐</span>
              <span class="path-name">{{ syncForm.source || '請於上方指定來源路徑 (Source)' }}</span>
              <Folder class="path-icon-right" />
            </div>
            <div class="path-col path-dest" :title="syncForm.dest || '未設定目的路徑'">
              <span class="path-triangle">☁️</span>
              <span class="path-name">{{ syncForm.dest || '請於上方指定目的路徑 (Destination)' }}</span>
              <Folder class="path-icon-right" />
            </div>
          </div>

          <!-- RcloneView Toolbar: Display Filters, Actions, Search (如同截圖所示) -->
          <div class="rcloneview-action-toolbar">
            <!-- Display Toggles -->
            <div class="display-group">
              <span class="toolbar-label">Display:</span>
              <button
                class="rv-toggle-btn toggle-src"
                :class="{ active: syncForm.filterModes.source_only }"
                @click="toggleFilterMode('source_only')"
                title="來源獨有 (待複製至目標端 ➡️)"
              >
                <ArrowRight class="rv-icon text-green" />
                <span>來源獨有 ({{ syncForm.diffResult?.total_source_files || 0 }})</span>
              </button>

              <button
                class="rv-toggle-btn toggle-dest"
                :class="{ active: syncForm.filterModes.dest_only }"
                @click="toggleFilterMode('dest_only')"
                title="目的端多出 (⬅️)"
              >
                <ArrowLeft class="rv-icon text-blue" />
                <span>目的多出 ({{ syncForm.diffResult?.total_dest_files || 0 }})</span>
              </button>

              <button
                class="rv-toggle-btn toggle-eq"
                :class="{ active: syncForm.filterModes.equal }"
                @click="toggleFilterMode('equal')"
                title="兩端完全一致 (＝)"
              >
                <Equal class="rv-icon text-neutral" />
                <span>完全一致 ({{ syncForm.diffResult?.total_equal || 0 }})</span>
              </button>

              <button
                class="rv-toggle-btn toggle-diff"
                :class="{ active: syncForm.filterModes.different }"
                @click="toggleFilterMode('different')"
                title="檔案內容或時間不同 (≠)"
              >
                <SlidersHorizontal class="rv-icon text-purple" />
                <span>內容差異 ({{ syncForm.diffResult?.total_different || 0 }})</span>
              </button>

              <button
                v-if="syncForm.diffResult?.total_error > 0"
                class="rv-toggle-btn toggle-err"
                :class="{ active: syncForm.filterModes.error }"
                @click="toggleFilterMode('error')"
                title="讀取或雜湊異常 (❗)"
              >
                <AlertTriangle class="rv-icon text-red" />
                <span>異常 ({{ syncForm.diffResult?.total_error || 0 }})</span>
              </button>
            </div>

            <!-- Middle Batch Actions (Copy Selected) -->
            <div class="actions-group">
              <button
                class="btn btn-sm btn-emerald"
                @click="handleCopySelected"
                :disabled="syncForm.selectedItems.size === 0 || syncForm.isCopyingSelected"
                title="將勾選的檔案從來源複製到目標端"
              >
                <Copy class="btn-icon-xs" />
                <span>Copy ➡️ ({{ syncForm.selectedItems.size }})</span>
              </button>
            </div>

            <!-- Right: Search Filter -->
            <div class="search-group ml-auto">
              <div class="rv-search-box">
                <Search class="rv-search-icon" />
                <input
                  type="text"
                  v-model="syncForm.searchKeyword"
                  class="rv-search-input"
                  placeholder="Find Folder/File..."
                />
              </div>
            </div>
          </div>

          <!-- RcloneView Classical Two-Column Table -->
          <div class="rcloneview-table-container">
            <table class="rv-table">
              <thead>
                <tr class="rv-thead-row">
                  <!-- Source Side Headers -->
                  <th class="col-chk">
                    <input
                      type="checkbox"
                      @change="toggleSelectAllVisible"
                      :checked="filteredDiffItems.length > 0 && filteredDiffItems.every(i => syncForm.selectedItems.has(i.path))"
                      title="全選 / 取消全選可見項目"
                    />
                  </th>
                  <th class="col-src-name">Name (來源端)</th>
                  <th class="col-src-size">Size</th>
                  <th class="col-src-date">Date Modified</th>

                  <!-- Center Direction Indicator Header -->
                  <th class="col-dir">Dir</th>

                  <!-- Destination Side Headers -->
                  <th class="col-dst-name">Name (目的端)</th>
                  <th class="col-dst-size">Size</th>
                  <th class="col-dst-date">Date Modified</th>

                  <!-- Quick Action Header -->
                  <th class="col-act">操作</th>
                </tr>
              </thead>

              <!-- Diff Items List -->
              <tbody v-if="syncForm.diffResult && filteredDiffItems.length > 0">
                <tr
                  v-for="item in filteredDiffItems"
                  :key="item.path"
                  class="rv-row"
                  :class="{
                    'rv-row-selected': syncForm.selectedItems.has(item.path),
                    'status-src-only': item.status === 'source_only',
                    'status-dst-only': item.status === 'dest_only',
                    'status-diff': item.status === 'different',
                    'status-equal': item.status === 'equal'
                  }"
                  @click="toggleItemSelect(item.path)"
                >
                  <!-- Checkbox -->
                  <td class="col-chk" @click.stop>
                    <input
                      type="checkbox"
                      :checked="syncForm.selectedItems.has(item.path)"
                      @change="toggleItemSelect(item.path)"
                    />
                  </td>

                  <!-- Source: Name -->
                  <td class="col-src-name">
                    <div class="rv-file-entry" :class="{ 'entry-empty': item.status === 'dest_only' }">
                      <Folder v-if="item.path.endsWith('/')" class="entry-icon text-amber" />
                      <span class="entry-name" :title="item.path">{{ item.path }}</span>
                    </div>
                  </td>

                  <!-- Source: Size -->
                  <td class="col-src-size">
                    <span v-if="item.size_src">{{ item.size_src }}</span>
                    <span v-else class="text-muted">-</span>
                  </td>

                  <!-- Source: Date Modified -->
                  <td class="col-src-date">
                    <span v-if="item.mtime_src">{{ item.mtime_src }}</span>
                    <span v-else class="text-muted">-</span>
                  </td>

                  <!-- Center: Direction Symbol (like RcloneView middle icons) -->
                  <td class="col-dir" @click.stop>
                    <div class="dir-icon-badge" :class="item.status">
                      <ArrowRight v-if="item.status === 'source_only'" class="dir-icon text-green" title="來源獨有，待複製到目標端 ➡️" />
                      <ArrowLeft v-else-if="item.status === 'dest_only'" class="dir-icon text-blue" title="目的端多出 ⬅️" />
                      <SlidersHorizontal v-else-if="item.status === 'different'" class="dir-icon text-purple" title="兩端內容或時間差異 ≠" />
                      <Equal v-else-if="item.status === 'equal'" class="dir-icon text-neutral" title="兩端完全一致 ＝" />
                      <AlertTriangle v-else class="dir-icon text-red" title="異常 ❗" />
                    </div>
                  </td>

                  <!-- Dest: Name -->
                  <td class="col-dst-name">
                    <div class="rv-file-entry" :class="{ 'entry-empty': item.status === 'source_only' }">
                      <span v-if="item.status === 'source_only'" class="entry-placeholder">(目標端尚未存在此檔)</span>
                      <template v-else>
                        <Folder v-if="item.path.endsWith('/')" class="entry-icon text-amber" />
                        <span class="entry-name" :title="item.path">{{ item.path }}</span>
                      </template>
                    </div>
                  </td>

                  <!-- Dest: Size -->
                  <td class="col-dst-size">
                    <span v-if="item.size_dest">{{ item.size_dest }}</span>
                    <span v-else class="text-muted">-</span>
                  </td>

                  <!-- Dest: Date Modified -->
                  <td class="col-dst-date">
                    <span v-if="item.mtime_dest">{{ item.mtime_dest }}</span>
                    <span v-else class="text-muted">-</span>
                  </td>

                  <!-- Quick Action Button -->
                  <td class="col-act" @click.stop>
                    <button
                      v-if="item.status === 'source_only' || item.status === 'different'"
                      class="btn-row-action"
                      @click="handleCopySingleFile(item.path)"
                      :disabled="syncForm.isCopyingSelected"
                      title="單獨複製此檔案至目的端"
                    >
                      <span>複製 ➡️</span>
                    </button>
                  </td>
                </tr>
              </tbody>

              <!-- Empty state inside Diff Table -->
              <tbody v-else-if="syncForm.diffResult && filteredDiffItems.length === 0">
                <tr>
                  <td colspan="9" class="rv-empty-cell">
                    <div class="rv-empty-box">
                      <CheckCircle2 class="empty-icon text-emerald" />
                      <span>在目前篩選條件下無符合項目（請嘗試開啟其他 Display 篩選按鈕或清除搜尋條件）</span>
                    </div>
                  </td>
                </tr>
              </tbody>

              <!-- Empty state before diff execution -->
              <tbody v-else>
                <tr>
                  <td colspan="9" class="rv-empty-cell placeholder-height">
                    <div class="rv-empty-box">
                      <FileCheck class="empty-placeholder-icon text-cyan" />
                      <h4>尚未執行差異比對</h4>
                      <p>請設定好上方來源與目的路徑後，點擊上方「🔍 比對兩端差異 (Check Diff)」按鈕</p>
                    </div>
                  </td>
                </tr>
              </tbody>
            </table>
          </div>

          <!-- RcloneView Classical Status Bar (如截圖底部之統計列) -->
          <div class="rcloneview-statusbar">
            <div class="status-left">
              <span>來源項目: {{ syncForm.diffResult?.total_source_files || 0 }} 待同步</span>
            </div>
            <div class="status-center">
              <span class="selected-count-tag">
                {{ syncForm.selectedItems.size }} item(s) selected.
              </span>
            </div>
            <div class="status-right">
              <span>目的端多出: {{ syncForm.diffResult?.total_dest_files || 0 }} 檔</span>
            </div>
          </div>

          <!-- Collapsible Task Log Drawer -->
          <details class="task-log-details" :open="Boolean(syncForm.taskLog)">
            <summary class="log-summary">
              <span>即時執行記錄與日誌</span>
              <span class="log-summary-hint">{{ syncForm.isRunning ? '（任務正在執行中...）' : '' }}</span>
            </summary>
            <pre class="log-content-pre">{{ syncForm.taskLog || '尚未執行任務，點選「立即開始同步/備份」或「Copy ➡️」即可檢視進度。' }}</pre>
          </details>
        </div>
      </section>

      <!-- TAB 3: SCHEDULER -->
      <section v-if="activeTab === 'scheduler'" class="tab-pane">
        <div class="sub-bar">
          <div class="sub-title-group">
            <h2>背景定時排程任務</h2>
            <p>即使關閉視窗，只要程式常駐在右下角 System Tray，排程就會在背景自動執行</p>
          </div>
          <button class="btn btn-primary" @click="showAddTaskModal = true">
            <PlusCircle class="btn-icon" />
            <span>新增排程任務</span>
          </button>
        </div>

        <div v-if="scheduledTasks.length > 0" class="task-table-wrapper">
          <table class="task-table">
            <thead>
              <tr>
                <th>狀態</th>
                <th>任務名稱</th>
                <th>來源路徑</th>
                <th>目的路徑</th>
                <th>模式</th>
                <th>執行頻率</th>
                <th>上次執行</th>
                <th>操作</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="task in scheduledTasks" :key="task.id">
                <td>
                  <span class="status-pill" :class="task.enabled ? 'pill-active' : 'pill-disabled'">
                    {{ task.enabled ? (task.status || '待命') : '已停用' }}
                  </span>
                </td>
                <td class="font-bold">{{ task.name }}</td>
                <td class="font-mono text-cyan">{{ task.source }}</td>
                <td class="font-mono text-emerald">{{ task.dest }}</td>
                <td>{{ task.action.toUpperCase() }}</td>
                <td>每隔 {{ task.intervalMinutes }} 分鐘</td>
                <td>{{ task.lastRun || '從未執行' }}</td>
                <td>
                  <div class="table-actions">
                    <button
                      class="btn btn-secondary btn-sm"
                      @click="executeTaskNow(task)"
                      :disabled="task.status === '執行中...'"
                      title="立即執行一次此排程任務"
                    >
                      <Play class="btn-icon" />
                      <span>執行</span>
                    </button>
                    <button
                      class="btn btn-secondary btn-sm"
                      @click="openEditTaskModal(task)"
                      title="修改排程任務設定"
                    >
                      <Pencil class="btn-icon" />
                      <span>修改</span>
                    </button>
                    <button
                      class="btn btn-secondary btn-sm"
                      @click="toggleTaskEnabled(task)"
                      :title="task.enabled ? '暫停定時排程' : '啟用定時排程'"
                    >
                      {{ task.enabled ? '停用' : '啟用' }}
                    </button>
                    <button
                      class="btn btn-danger btn-sm"
                      @click="removeTask(task.id)"
                      title="刪除此排程任務"
                    >
                      <Trash2 class="btn-icon" />
                    </button>
                  </div>
                </td>
              </tr>
            </tbody>
          </table>
        </div>

        <div v-else class="empty-state">
          <div class="empty-icon-box">
            <CalendarClock class="empty-icon" />
          </div>
          <h2>目前沒有任何定時排程任務</h2>
          <p>設定定時備份任務，自動在背景把資料夾同步至 Google Drive、OneDrive 或 NAS。</p>
          <button class="btn btn-primary" @click="showAddTaskModal = true">
            <PlusCircle class="btn-icon" />
            <span>建立第一個排程任務</span>
          </button>
        </div>
      </section>

      <!-- TAB 4: WEB-GUI -->
      <section v-if="activeTab === 'webgui'" class="tab-pane">
        <div class="sub-bar">
          <div class="sub-title-group">
            <h2>Rclone 官方原生 Web-GUI</h2>
            <p>透過內建 Web 伺服器提供官方完整版管理介面（檔案總管、圖形儀表板）</p>
          </div>
          <div class="sub-actions">
            <button
              class="btn"
              :class="envStatus.webgui_running ? 'btn-danger' : 'btn-primary'"
              @click="toggleWebGui"
            >
              <Power class="btn-icon" />
              <span>{{ envStatus.webgui_running ? '停止 Web-GUI 服務' : '啟動 Web-GUI 服務' }}</span>
            </button>

            <button class="btn btn-emerald" @click="openWebGuiInBrowser">
              <ExternalLink class="btn-icon" />
              <span>以預設瀏覽器開啟 (http://127.0.0.1:5572)</span>
            </button>
          </div>
        </div>

        <!-- Embedded Web-GUI Frame -->
        <div class="webgui-frame-box">
          <iframe
            v-if="envStatus.webgui_running"
            src="http://127.0.0.1:5572/"
            class="webgui-iframe"
          ></iframe>
          <div v-else class="webgui-placeholder">
            <Globe class="placeholder-icon" />
            <h3>Web-GUI 服務尚未啟動</h3>
            <p>點選上方「啟動 Web-GUI 服務」即可在此內嵌或透過瀏覽器進行操作。</p>
            <button class="btn btn-primary btn-lg" @click="toggleWebGui">
              <Power class="btn-icon" />
              <span>立即啟動 Web-GUI 伺服器</span>
            </button>
          </div>
        </div>
      </section>
    </main>

    <!-- MODAL: ADD REMOTE (GUI WIZARD) -->
    <div v-if="showAddRemoteModal" class="modal-backdrop" @click.self="showAddRemoteModal = false">
      <div class="modal-card">
        <div class="modal-header">
          <div class="modal-title">
            <PlusCircle class="modal-title-icon" />
            <span>新增雲端硬碟連線 (純圖形精靈)</span>
          </div>
          <button class="close-btn" @click="showAddRemoteModal = false">
            <X class="close-icon" />
          </button>
        </div>

        <div class="modal-body">
          <div class="setting-group">
            <label class="group-title">雲端硬碟名稱 (Remote Name)</label>
            <input
              type="text"
              v-model="newRemoteForm.name"
              class="modal-input"
              placeholder="例如：MyGoogleDrive, CompanyNAS"
            />
          </div>

          <div class="setting-group">
            <label class="group-title">雲端服務類型 (Storage Provider)</label>
            <select v-model="newRemoteForm.type" class="modal-input">
              <option value="drive">Google Drive (個人 / 企業雲端硬碟)</option>
              <option value="onedrive">Microsoft OneDrive</option>
              <option value="dropbox">Dropbox</option>
              <option value="webdav">WebDAV (Nextcloud, Synology, QNAP...)</option>
              <option value="ftp">FTP 伺服器</option>
              <option value="sftp">SFTP / SSH 伺服器</option>
              <option value="s3">Amazon S3 或相容物件儲存</option>
            </select>
          </div>

          <!-- OAuth Notice for Google / OneDrive -->
          <div v-if="newRemoteForm.type === 'drive' || newRemoteForm.type === 'onedrive'" class="info-box">
            <div class="info-title">
              <Info class="info-icon" />
              <span>瀏覽器授權登入</span>
            </div>
            <p class="group-desc">
              點擊「立即建立」後，系統會自動在您的預設瀏覽器中開啟官方登入授權頁面。<br />
              登入完成並授權後，即可自動完成設定，完全無需輸入任何命令指令！
            </p>
          </div>

          <!-- WebDAV Inputs -->
          <template v-if="newRemoteForm.type === 'webdav'">
            <div class="setting-group">
              <label class="group-title">WebDAV URL</label>
              <input type="text" v-model="newRemoteForm.url" class="modal-input" placeholder="https://example.com/remote.php/webdav" />
            </div>
            <div class="setting-group">
              <label class="group-title">使用者名稱</label>
              <input type="text" v-model="newRemoteForm.user" class="modal-input" placeholder="Username" />
            </div>
            <div class="setting-group">
              <label class="group-title">密碼 / 應用程式密碼</label>
              <input type="password" v-model="newRemoteForm.pass" class="modal-input" placeholder="Password" />
            </div>
          </template>

          <!-- FTP / SFTP Inputs -->
          <template v-if="newRemoteForm.type === 'ftp' || newRemoteForm.type === 'sftp'">
            <div class="setting-group">
              <label class="group-title">主機位置 (Host)</label>
              <input type="text" v-model="newRemoteForm.host" class="modal-input" placeholder="192.168.1.100 或 ftp.example.com" />
            </div>
            <div class="setting-group" v-if="newRemoteForm.type === 'ftp'">
              <label class="group-title">連接埠 (Port)</label>
              <input type="text" v-model="newRemoteForm.port" class="modal-input" placeholder="21" />
            </div>
            <div class="setting-group">
              <label class="group-title">使用者帳號</label>
              <input type="text" v-model="newRemoteForm.user" class="modal-input" placeholder="Username" />
            </div>
            <div class="setting-group">
              <label class="group-title">密碼</label>
              <input type="password" v-model="newRemoteForm.pass" class="modal-input" placeholder="Password" />
            </div>
          </template>
        </div>

        <div class="modal-footer">
          <button class="btn btn-secondary" @click="showAddRemoteModal = false">取消</button>
          <button
            class="btn btn-primary"
            @click="submitAddRemote"
            :disabled="newRemoteForm.isCreating"
          >
            <Disc3 class="btn-icon" :class="{ 'spin-anim': newRemoteForm.isCreating }" />
            <span>{{ newRemoteForm.isCreating ? '建立中 (請在瀏覽器完成登入)...' : '立即建立' }}</span>
          </button>
        </div>
      </div>
    </div>

    <!-- MODAL: EDIT REMOTE (MODIFY CONFIGURATION) -->
    <div v-if="showEditRemoteModal" class="modal-backdrop" @click.self="showEditRemoteModal = false">
      <div class="modal-card">
        <div class="modal-header">
          <div class="modal-title">
            <Pencil class="modal-title-icon" />
            <span>修改雲端設定: {{ editRemoteForm.newName || editRemoteForm.name }}</span>
          </div>
          <button class="close-btn" @click="showEditRemoteModal = false">
            <X class="close-icon" />
          </button>
        </div>

        <div class="modal-body">
          <div class="setting-group">
            <label class="group-title">雲端硬碟名稱 (Remote Name)</label>
            <input
              type="text"
              v-model="editRemoteForm.newName"
              class="modal-input font-bold"
              placeholder="例如：xVideo"
            />
            <span class="field-hint">可直接修改名稱，儲存後掛載代號與設定將自動平移。</span>
          </div>

          <div class="setting-group">
            <label class="group-title">服務類型</label>
            <div class="text-cyan font-bold">{{ getProviderDetails(editRemoteForm.type).name }} ({{ editRemoteForm.type }})</div>
          </div>

          <!-- OAuth Re-authenticate for Drive / OneDrive -->
          <div v-if="editRemoteForm.type.includes('drive') || editRemoteForm.type.includes('onedrive')" class="setting-group info-box">
            <div class="info-title">
              <KeyRound class="info-icon" />
              <span>雲端帳號登入授權</span>
            </div>
            <p class="group-desc">
              若遇到憑證過期或無法存取，點擊下方按鈕即可重新在預設瀏覽器中進行 OAuth 帳號登入授權。
            </p>
            <button
              class="btn btn-emerald mt-2"
              @click="handleReconnectRemote"
              :disabled="editRemoteForm.isReconnecting"
            >
              <RotateCw class="btn-icon" :class="{ 'spin-anim': editRemoteForm.isReconnecting }" />
              <span>{{ editRemoteForm.isReconnecting ? '正在開啟瀏覽器授權...' : '重新至瀏覽器登入授權' }}</span>
            </button>
          </div>

          <!-- WebDAV Edit -->
          <template v-if="editRemoteForm.type === 'webdav'">
            <div class="setting-group">
              <label class="group-title">WebDAV URL</label>
              <input type="text" v-model="editRemoteForm.url" class="modal-input" />
            </div>
            <div class="setting-group">
              <label class="group-title">使用者名稱</label>
              <input type="text" v-model="editRemoteForm.user" class="modal-input" />
            </div>
            <div class="setting-group">
              <label class="group-title">新密碼 (若不修改請留空)</label>
              <input type="password" v-model="editRemoteForm.pass" class="modal-input" placeholder="留空代表不變更密碼" />
            </div>
          </template>

          <!-- FTP / SFTP Edit -->
          <template v-if="editRemoteForm.type === 'ftp' || editRemoteForm.type === 'sftp'">
            <div class="setting-group">
              <label class="group-title">主機位置 (Host)</label>
              <input type="text" v-model="editRemoteForm.host" class="modal-input" />
            </div>
            <div class="setting-group" v-if="editRemoteForm.type === 'ftp'">
              <label class="group-title">連接埠 (Port)</label>
              <input type="text" v-model="editRemoteForm.port" class="modal-input" />
            </div>
            <div class="setting-group">
              <label class="group-title">使用者帳號</label>
              <input type="text" v-model="editRemoteForm.user" class="modal-input" />
            </div>
            <div class="setting-group">
              <label class="group-title">新密碼 (若不修改請留空)</label>
              <input type="password" v-model="editRemoteForm.pass" class="modal-input" placeholder="留空代表不變更密碼" />
            </div>
          </template>

          <!-- Optional Client ID / Secret for Advanced Users -->
          <div v-if="editRemoteForm.type.includes('drive') || editRemoteForm.type.includes('onedrive')" class="setting-group">
            <label class="group-title">自訂 Client ID (選填)</label>
            <input
              type="text"
              v-model="editRemoteForm.clientId"
              class="modal-input"
              :placeholder="editRemoteForm.type.includes('onedrive') ? '自訂 Microsoft / Azure Client ID' : '自訂 Google API Client ID'"
            />
            <label class="group-title mt-2">自訂 Client Secret (選填)</label>
            <input
              type="password"
              v-model="editRemoteForm.clientSecret"
              class="modal-input"
              :placeholder="editRemoteForm.type.includes('onedrive') ? '自訂 Microsoft / Azure Client Secret' : '自訂 Google API Client Secret'"
            />
          </div>
        </div>

        <div class="modal-footer">
          <button class="btn btn-secondary" @click="showEditRemoteModal = false">取消</button>
          <button
            class="btn btn-primary"
            @click="submitEditRemote"
            :disabled="editRemoteForm.isSaving"
          >
            <Check class="btn-icon" />
            <span>{{ editRemoteForm.isSaving ? '儲存中...' : '儲存修改' }}</span>
          </button>
        </div>
      </div>
    </div>

    <!-- MODAL: ADD TASK (SCHEDULER) -->
    <div v-if="showAddTaskModal" class="modal-backdrop" @click.self="showAddTaskModal = false">
      <div class="modal-card">
        <div class="modal-header">
          <div class="modal-title">
            <CalendarClock class="modal-title-icon" />
            <span>建立定時排程同步任務</span>
          </div>
          <button class="close-btn" @click="showAddTaskModal = false">
            <X class="close-icon" />
          </button>
        </div>

        <div class="modal-body">
          <div class="setting-group">
            <label class="group-title">任務名稱</label>
            <input type="text" v-model="newTask.name" class="modal-input" placeholder="例如：每日照片自動備份" />
          </div>
          <div class="setting-group">
            <label class="group-title">來源路徑 (Source)</label>
            <input type="text" v-model="newTask.source" class="modal-input" placeholder="例如：D:\Photos 或 GDrive_alanytp100:Photos" />
          </div>
          <div class="setting-group">
            <label class="group-title">目的路徑 (Destination)</label>
            <input type="text" v-model="newTask.dest" class="modal-input" placeholder="例如：GPhoto_alanytp100:Backup" />
          </div>
          <div class="setting-group">
            <label class="group-title">動作模式</label>
            <select v-model="newTask.action" class="modal-input">
              <option value="copy">Copy (增量備份，不刪除目的端檔案)</option>
              <option value="sync">Sync (完全鏡像同步，目的端檔案與來源一致)</option>
            </select>
          </div>
          <div class="setting-group">
            <label class="group-title">排除檔案過濾規則 (選填)</label>
            <input type="text" v-model="newTask.excludeFilter" class="modal-input" placeholder="例如：*.tmp, *.bak, thumbs.db, node_modules/**" />
            <span class="field-hint">若有輸入，排程執行時將自動略過符合的檔案</span>
          </div>
          <div class="setting-group">
            <label class="group-title">執行頻率 (分鐘)</label>
            <input type="number" v-model="newTask.intervalMinutes" class="modal-input" min="5" placeholder="60" />
            <span class="field-hint">每隔多少分鐘自動執行一次 (建議 60 分鐘以上)</span>
          </div>
        </div>

        <div class="modal-footer">
          <button class="btn btn-secondary" @click="showAddTaskModal = false">取消</button>
          <button class="btn btn-primary" @click="submitAddTask">建立排程</button>
        </div>
      </div>
    </div>

    <!-- MODAL: EDIT TASK (SCHEDULER) -->
    <div v-if="showEditTaskModal" class="modal-backdrop" @click.self="showEditTaskModal = false">
      <div class="modal-card">
        <div class="modal-header">
          <div class="modal-title">
            <Pencil class="modal-title-icon" />
            <span>修改排程任務</span>
          </div>
          <button class="close-btn" @click="showEditTaskModal = false">
            <X class="close-icon" />
          </button>
        </div>

        <div class="modal-body">
          <div class="setting-group">
            <label class="group-title">任務名稱</label>
            <input type="text" v-model="editTaskForm.name" class="modal-input" placeholder="例如：每日照片自動備份" />
          </div>
          <div class="setting-group">
            <label class="group-title">來源路徑 (Source)</label>
            <input type="text" v-model="editTaskForm.source" class="modal-input" placeholder="例如：D:\Photos 或 GDrive_alanytp100:Photos" />
          </div>
          <div class="setting-group">
            <label class="group-title">目的路徑 (Destination)</label>
            <input type="text" v-model="editTaskForm.dest" class="modal-input" placeholder="例如：GPhoto_alanytp100:Backup" />
          </div>
          <div class="setting-group">
            <label class="group-title">動作模式</label>
            <select v-model="editTaskForm.action" class="modal-input">
              <option value="copy">Copy (增量備份，不刪除目的端檔案)</option>
              <option value="sync">Sync (完全鏡像同步，目的端檔案與來源一致)</option>
            </select>
          </div>
          <div class="setting-group">
            <label class="group-title">排除檔案過濾規則 (選填)</label>
            <input type="text" v-model="editTaskForm.excludeFilter" class="modal-input" placeholder="例如：*.tmp, *.bak, thumbs.db, node_modules/**" />
            <span class="field-hint">若有輸入，排程執行時將自動略過符合的檔案</span>
          </div>
          <div class="setting-group">
            <label class="group-title">執行頻率 (分鐘)</label>
            <input type="number" v-model="editTaskForm.intervalMinutes" class="modal-input" min="1" placeholder="60" />
            <span class="field-hint">每隔多少分鐘自動執行一次 (建議 60 分鐘以上)</span>
          </div>
          <div class="setting-group">
            <label class="group-title">任務狀態</label>
            <label class="checkbox-label" style="display: flex; align-items: center; gap: 8px; cursor: pointer; user-select: none;">
              <input type="checkbox" v-model="editTaskForm.enabled" />
              <span>啟用此定時排程任務</span>
            </label>
          </div>
        </div>

        <div class="modal-footer">
          <button class="btn btn-secondary" @click="showEditTaskModal = false">取消</button>
          <button class="btn btn-primary" @click="submitEditTask">儲存修改</button>
        </div>
      </div>
    </div>

    <!-- MODAL: SETTINGS -->
    <div v-if="showSettings" class="modal-backdrop" @click.self="showSettings = false">
      <div class="modal-card">
        <div class="modal-header">
          <div class="modal-title">
            <Sliders class="modal-title-icon" />
            <span>系統偏好與自訂路徑</span>
          </div>
          <button class="close-btn" @click="showSettings = false">
            <X class="close-icon" />
          </button>
        </div>

        <div class="modal-body">
          <div class="setting-group">
            <label class="group-title">Rclone 執行檔路徑</label>
            <p class="group-desc">預設為 C:\rclone\rclone.exe</p>
            <div class="input-with-button">
              <input type="text" v-model="customRclonePath" class="modal-input" />
              <button class="btn btn-secondary" @click="saveCustomPath">儲存路徑</button>
            </div>
          </div>

          <div class="setting-group">
            <div class="switch-row">
              <div>
                <div class="switch-title">開機自動啟動</div>
                <div class="group-desc">登入 Windows 時自動在後台啟動 Rclone Drive</div>
              </div>
              <button
                class="toggle-switch"
                :class="{ active: autostartActive }"
                @click="toggleAutostart"
              >
                <span class="toggle-slider"></span>
              </button>
            </div>
          </div>

          <div class="setting-group">
            <div class="switch-row">
              <div>
                <div class="switch-title">開機啟動後縮小至系統匣 (Tray)</div>
                <div class="group-desc">開機自啟動時保持在右下角通知區常駐，不主動彈出視窗</div>
              </div>
              <button
                class="toggle-switch"
                :class="{ active: startMinimizedActive }"
                @click="toggleStartMinimized"
              >
                <span class="toggle-slider"></span>
              </button>
            </div>
          </div>

          <div class="setting-group">
            <div class="switch-row">
              <div>
                <div class="switch-title">淺色外觀主題 (Light Mode)</div>
                <div class="group-desc">切換明亮淺色或深邃暗色介面外觀風格</div>
              </div>
              <button
                class="toggle-switch"
                :class="{ active: isLightMode }"
                @click="toggleTheme"
              >
                <span class="toggle-slider"></span>
              </button>
            </div>
          </div>

          <!-- GitHub Release Update Settings -->
          <div class="setting-group">
            <div class="switch-row">
              <div>
                <div class="switch-title">自動檢查 GitHub 最新版本</div>
                <div class="group-desc">
                  目前版本：v{{ CURRENT_VERSION }} • 上次檢查：{{ updateSettings.lastCheckTimeStr }}
                </div>
              </div>
              <button
                class="toggle-switch"
                :class="{ active: updateSettings.enabled }"
                @click="toggleUpdateEnabled"
              >
                <span class="toggle-slider"></span>
              </button>
            </div>
          </div>

          <div class="setting-group" v-if="updateSettings.enabled">
            <label class="group-title">版本檢查頻率</label>
            <p class="group-desc">設定背景向 GitHub Releases 自動檢查新版本的間隔週期</p>
            <select
              v-model.number="updateSettings.intervalDays"
              @change="saveUpdateSettings"
              class="modal-input"
            >
              <option :value="1">每天檢查一次</option>
              <option :value="3">每 3 天檢查一次</option>
              <option :value="7">每週檢查一次 (預設)</option>
              <option :value="14">每兩週檢查一次</option>
              <option :value="30">每月檢查一次</option>
            </select>
          </div>

          <div class="setting-group">
            <div class="switch-row">
              <div>
                <div class="switch-title">手動檢查最新版本</div>
                <div class="group-desc">即時連線 GitHub 查詢是否有新版釋出</div>
              </div>
              <button
                class="btn btn-secondary btn-sm"
                @click="checkForUpdates(true)"
                :disabled="isCheckingUpdate"
              >
                <RefreshCw class="btn-icon" :class="{ 'spin-anim': isCheckingUpdate }" />
                <span>{{ isCheckingUpdate ? '檢查中...' : '檢查更新' }}</span>
              </button>
            </div>
          </div>

          <div class="setting-group info-box">
            <div class="info-title">
              <Info class="info-icon" />
              <span>依賴元件下載資訊</span>
            </div>
            <p class="group-desc">
              • <b>WinFsp 檔案系統核心</b>：<a href="#" class="link" @click.prevent="openUrl('https://winfsp.dev/')">前往 winfsp.dev 下載</a><br />
              • <b>Rclone 官方最新版本</b>：<a href="#" class="link" @click.prevent="openUrl('https://rclone.org/downloads/')">前往 rclone.org 下載</a>
            </p>
          </div>
        </div>

        <div class="modal-footer">
          <button class="btn btn-secondary" @click="showSettings = false">關閉</button>
        </div>
      </div>
    </div>

    <!-- MODAL: CLOUD DIRECTORY BROWSER -->
    <div v-if="showCloudBrowseModal" class="modal-backdrop" @click.self="showCloudBrowseModal = false">
      <div class="modal-card cloud-browser-modal">
        <div class="modal-header">
          <div class="modal-title">
            <Cloud class="modal-title-icon text-cyan" />
            <span>選擇雲端硬碟目錄 ({{ cloudBrowseTarget === 'source' ? '來源路徑' : '目的路徑' }})</span>
          </div>
          <button class="close-btn" @click="showCloudBrowseModal = false">
            <X class="close-icon" />
          </button>
        </div>

        <div class="modal-body">
          <!-- Remote Selector -->
          <div class="setting-group">
            <label class="group-title">選擇雲端遠端 (Remote)</label>
            <select v-model="cloudBrowseRemote" class="modal-input" @change="cloudBrowseCurrentSubpath = ''; fetchCloudDirs()">
              <option v-for="r in remotes" :key="r.name" :value="r.name">
                {{ r.name }} ({{ r.remote_type }})
              </option>
            </select>
          </div>

          <!-- Current Path Breadcrumb -->
          <div class="cloud-breadcrumb-bar">
            <button class="btn btn-xs btn-outline" @click="goCloudParentDir" :disabled="!cloudBrowseCurrentSubpath">
              <span>⬅️ 上一層目錄</span>
            </button>
            <div class="cloud-current-path" :title="`${cloudBrowseRemote}:${cloudBrowseCurrentSubpath}`">
              📁 {{ cloudBrowseRemote }}:/{{ cloudBrowseCurrentSubpath }}
            </div>
          </div>

          <!-- Directory List -->
          <div class="cloud-dirs-container">
            <div v-if="isLoadingCloudDirs" class="cloud-dirs-loading">
              <RefreshCw class="btn-icon spin-anim text-cyan" />
              <span>正在讀取雲端目錄清單...</span>
            </div>
            <div v-else-if="cloudBrowseDirs.length === 0" class="cloud-dirs-empty">
              <span>此層無其他子資料夾（或可以直接選擇此層作為路徑）</span>
            </div>
            <div v-else class="cloud-dirs-list">
              <div
                v-for="d in cloudBrowseDirs"
                :key="d"
                class="cloud-dir-item"
                @dblclick="enterCloudSubdir(d)"
              >
                <Folder class="cloud-dir-icon" />
                <span class="cloud-dir-name">{{ d }}</span>
                <button class="btn btn-xs btn-outline ml-auto" @click="enterCloudSubdir(d)">
                  進入 ➔
                </button>
              </div>
            </div>
          </div>
          <span class="field-hint">可點擊「進入」進入子資料夾，確認好路徑後點擊下方「選擇此路徑」即可自動帶入！</span>
        </div>

        <div class="modal-footer">
          <button class="btn btn-secondary" @click="showCloudBrowseModal = false">取消</button>
          <button class="btn btn-primary" @click="confirmCloudBrowseSelect">
            <span>選擇此路徑 ({{ cloudBrowseRemote }}:{{ cloudBrowseCurrentSubpath ? '/' + cloudBrowseCurrentSubpath : '' }})</span>
          </button>
        </div>
      </div>
    </div>

    <!-- MODAL: UPDATE AVAILABLE -->
    <div v-if="showUpdateModal" class="modal-backdrop" @click.self="showUpdateModal = false">
      <div class="modal-card update-modal-card">
        <div class="modal-header">
          <div class="modal-title">
            <DownloadCloud class="modal-title-icon text-cyan" />
            <span>發現新版本更新 (v{{ updateInfo.latestVersion }})</span>
          </div>
          <button class="close-btn" @click="showUpdateModal = false">
            <X class="close-icon" />
          </button>
        </div>

        <div class="modal-body">
          <div class="update-banner">
            <div class="update-version-tag">
              <span class="old-ver">v{{ updateInfo.currentVersion }}</span>
              <ArrowRight class="ver-arrow" />
              <span class="new-ver">v{{ updateInfo.latestVersion }}</span>
            </div>
            <div class="update-date" v-if="updateInfo.publishedAt">
              發布日期：{{ updateInfo.publishedAt }}
            </div>
          </div>

          <div class="update-release-title" v-if="updateInfo.releaseTitle">
            {{ updateInfo.releaseTitle }}
          </div>

          <div class="update-notes-container">
            <div class="update-notes-header">更新說明與改版內容：</div>
            <pre class="update-notes-pre">{{ updateInfo.releaseNotes }}</pre>
          </div>

          <p class="update-tip">
            點擊下方「下載更新並執行」，系統將自動下載最新安裝程式並啟動更新。您也可以直接前往 GitHub 頁面查看或手動下載。
          </p>
        </div>

        <div class="modal-footer">
          <button class="btn btn-secondary" @click="showUpdateModal = false" :disabled="isDownloadingUpdate">
            稍後提醒
          </button>
          <button class="btn btn-secondary" @click="openUrl(updateInfo.htmlUrl)">
            <ExternalLink class="btn-icon" />
            <span>在 GitHub 查看</span>
          </button>
          <button class="btn btn-primary btn-update-action" @click="executeUpdate" :disabled="isDownloadingUpdate">
            <DownloadCloud class="btn-icon" :class="{ 'spin-anim': isDownloadingUpdate }" />
            <span>{{ isDownloadingUpdate ? '正在下載更新中...' : '下載更新並執行' }}</span>
          </button>
        </div>
      </div>
    </div>

    <!-- Toast Notification -->
    <transition name="toast">
      <div v-if="toast.show" class="toast-card" :class="`toast-${toast.type}`">
        <component
          :is="toast.type === 'success' ? CheckCircle2 : toast.type === 'error' ? XCircle : Info"
          class="toast-icon"
        />
        <span>{{ toast.message }}</span>
      </div>
    </transition>
  </div>
</template>

<style scoped>
.app-layout {
  display: flex;
  flex-direction: column;
  height: 100vh;
  background: var(--bg-app-gradient);
  color: var(--text-main);
}

/* Header */
.app-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 14px 28px;
  background: var(--bg-header);
  backdrop-filter: blur(12px);
  border-bottom: 1px solid var(--border-subtle);
  z-index: 10;
}

.brand {
  display: flex;
  align-items: center;
  gap: 14px;
}

.logo-box {
  position: relative;
  width: 42px;
  height: 42px;
  border-radius: var(--radius-md);
  background: linear-gradient(135deg, #0284c7, #6366f1);
  display: flex;
  align-items: center;
  justify-content: center;
  box-shadow: 0 4px 15px rgba(2, 132, 199, 0.4);
}

.logo-icon {
  width: 22px;
  height: 22px;
  color: #fff;
}

.logo-pulse {
  position: absolute;
  top: -2px;
  right: -2px;
  width: 10px;
  height: 10px;
  background: #10b981;
  border: 2px solid var(--bg-primary);
  border-radius: 50%;
}

.brand-title {
  font-size: 18px;
  font-weight: 700;
  background: var(--brand-title-gradient);
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
}

.brand-subtitle {
  font-size: 11px;
  color: var(--text-muted);
}

/* Navigation Tabs */
.nav-tabs {
  display: flex;
  align-items: center;
  gap: 6px;
  background: var(--bg-nav);
  padding: 4px;
  border-radius: var(--radius-md);
  border: 1px solid var(--border-subtle);
}

.tab-btn {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 14px;
  font-size: 13px;
  font-weight: 500;
  color: var(--text-secondary);
  background: transparent;
  border-radius: var(--radius-sm);
  transition: all 0.2s;
}

.tab-btn:hover {
  color: var(--text-main);
  background: rgba(255, 255, 255, 0.05);
}

.tab-btn.active {
  color: #fff;
  background: linear-gradient(135deg, #0284c7, #2563eb);
  box-shadow: 0 2px 8px rgba(2, 132, 199, 0.3);
}

.tab-icon {
  width: 16px;
  height: 16px;
}

.header-actions {
  display: flex;
  align-items: center;
  gap: 10px;
}

/* Missing Prerequisites Warning Banner */
.prereq-alert-banner {
  background: linear-gradient(90deg, rgba(245, 158, 11, 0.15), rgba(239, 68, 68, 0.15));
  border-bottom: 1px solid rgba(245, 158, 11, 0.35);
  padding: 16px 28px;
}

.prereq-content {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.prereq-header {
  display: flex;
  align-items: center;
  gap: 10px;
}

.prereq-warn-icon {
  width: 20px;
  height: 20px;
  color: #f59e0b;
}

.prereq-title {
  font-size: 14px;
  font-weight: 600;
  color: var(--text-warn-heading);
}

.prereq-cards-row {
  display: flex;
  gap: 16px;
  flex-wrap: wrap;
}

.prereq-item-card {
  flex: 1;
  min-width: 280px;
  background: var(--bg-prereq-card);
  border: 1px solid rgba(245, 158, 11, 0.3);
  border-radius: var(--radius-md);
  padding: 14px 18px;
  display: flex;
  flex-direction: column;
  justify-content: space-between;
  gap: 12px;
}

.prereq-card-text {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.prereq-badge {
  font-size: 10px;
  font-weight: 700;
  color: var(--text-warn);
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.prereq-item-card h4 {
  font-size: 14px;
  font-weight: 600;
  color: var(--text-main);
}

.prereq-item-card p {
  font-size: 12px;
  color: var(--text-secondary);
  line-height: 1.4;
}

.prereq-item-card code {
  font-family: 'JetBrains Mono', monospace;
  background: var(--bg-card-footer);
  padding: 2px 6px;
  border-radius: 4px;
  color: var(--accent-cyan);
}

.prereq-btn-group {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
}

.btn-download {
  background: linear-gradient(135deg, #f59e0b, #d97706);
  color: #000;
  font-weight: 600;
  padding: 8px 14px;
  font-size: 12px;
  border-radius: var(--radius-sm);
  display: inline-flex;
  align-items: center;
  gap: 6px;
}

.btn-download:hover {
  filter: brightness(1.15);
}

.btn-download-alt {
  background: var(--bg-btn-secondary);
  color: var(--text-main);
  font-weight: 500;
  padding: 8px 14px;
  font-size: 12px;
  border-radius: var(--radius-sm);
  border: 1px solid var(--border-subtle);
  display: inline-flex;
  align-items: center;
  gap: 6px;
}

.btn-download-alt:hover {
  background: var(--bg-btn-secondary-hover);
}

/* Environment Banner */
.env-banner {
  display: flex;
  align-items: center;
  gap: 20px;
  padding: 8px 28px;
  background: var(--bg-banner);
  border-bottom: 1px solid var(--border-subtle);
  font-size: 12px;
  color: var(--text-secondary);
}

.env-item {
  display: flex;
  align-items: center;
  gap: 6px;
}

.env-icon {
  width: 15px;
  height: 15px;
}

.env-ok {
  color: #34d399;
}

.env-warn {
  color: var(--text-warn);
}

.env-neutral {
  color: var(--text-secondary);
}

.clickable-link {
  color: var(--text-warn);
  text-decoration: underline;
  cursor: pointer;
}

.clickable-link:hover {
  color: var(--accent-amber);
}

.ml-auto {
  margin-left: auto;
}

.text-cyan {
  color: var(--accent-cyan);
}

.text-emerald {
  color: var(--accent-emerald);
}

.font-mono {
  font-family: 'JetBrains Mono', monospace;
}

.font-bold {
  font-weight: 600;
}

.mt-2 {
  margin-top: 8px;
}

.count-pill {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  padding: 4px 10px;
  background: rgba(255, 255, 255, 0.05);
  border: 1px solid var(--border-subtle);
  border-radius: 20px;
  font-size: 12px;
  color: var(--text-secondary);
}

.indicator-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: #64748b;
}

.indicator-dot.active {
  background: #10b981;
  box-shadow: 0 0 6px #10b981;
}

/* Main Content */
.main-content {
  flex: 1;
  overflow-y: auto;
  padding: 24px 28px;
}

.tab-pane {
  display: flex;
  flex-direction: column;
  gap: 20px;
  min-height: 100%;
}

.sub-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.sub-title-group h2 {
  font-size: 17px;
  font-weight: 700;
  color: var(--text-main);
}

.sub-title-group p {
  font-size: 12px;
  color: var(--text-muted);
  margin-top: 2px;
}

.sub-actions {
  display: flex;
  align-items: center;
  gap: 10px;
}

/* Cards Grid */
.cards-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(360px, 1fr));
  gap: 20px;
}

.drive-card {
  background: var(--bg-card);
  backdrop-filter: blur(16px);
  border: 1px solid var(--border-card);
  border-radius: var(--radius-lg);
  display: flex;
  flex-direction: column;
  transition: all 0.25s ease;
  overflow: hidden;
  box-shadow: var(--shadow-md);
}

.drive-card:hover {
  background: var(--bg-card-hover);
  border-color: var(--border-focus);
  transform: translateY(-2px);
  box-shadow: var(--shadow-lg);
}

.drive-card.card-mounted {
  border-color: rgba(16, 185, 129, 0.3);
  box-shadow: 0 4px 20px -2px rgba(16, 185, 129, 0.15);
}

.card-header {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 16px 18px;
  border-bottom: 1px solid var(--border-subtle);
}

.provider-badge {
  width: 42px;
  height: 42px;
  border-radius: var(--radius-md);
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}

.provider-icon {
  width: 22px;
  height: 22px;
}

.remote-meta {
  flex: 1;
  min-width: 0;
}

.remote-name {
  font-size: 15px;
  font-weight: 600;
  color: var(--text-main);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.remote-type-tag {
  font-size: 11px;
  color: var(--text-muted);
  margin-top: 2px;
}

.mount-status-badge {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 11px;
  font-weight: 500;
  padding: 4px 10px;
  border-radius: 12px;
}

.status-mounted {
  background: var(--bg-emerald-btn);
  color: var(--text-emerald-btn);
  border: 1px solid var(--border-emerald-btn);
}

.status-mounted .status-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: #10b981;
  box-shadow: 0 0 6px #10b981;
}

.status-unmounted {
  background: var(--bg-status-unmounted);
  color: var(--text-muted);
}

.status-unmounted .status-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: #64748b;
}

.card-header-actions {
  display: flex;
  align-items: center;
  gap: 4px;
}

.icon-btn-action,
.icon-btn-delete {
  background: transparent;
  color: var(--text-muted);
  padding: 6px;
  border-radius: 4px;
}

.icon-btn-action:hover {
  color: var(--accent-cyan);
  background: rgba(56, 189, 248, 0.1);
}

.icon-btn-delete:hover {
  color: #fb7185;
  background: rgba(244, 63, 94, 0.1);
}

.action-icon,
.trash-icon {
  width: 15px;
  height: 15px;
}

.card-body {
  padding: 16px 18px;
  display: flex;
  flex-direction: column;
  gap: 12px;
  flex: 1;
}

.setting-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}

.field-label {
  font-size: 12px;
  color: var(--text-secondary);
  font-weight: 500;
}

.field-hint {
  font-size: 11px;
  color: var(--text-muted);
  margin-top: 4px;
}

.drive-select-wrapper {
  min-width: 140px;
}

.drive-select,
.field-select,
.field-input,
.modal-input {
  background: var(--bg-input);
  border: 1px solid var(--border-subtle);
  border-radius: var(--radius-sm);
  color: var(--text-main);
  padding: 7px 10px;
  font-size: 13px;
  width: 100%;
  transition: all 0.2s;
}

.drive-select:focus,
.field-select:focus,
.field-input:focus,
.modal-input:focus {
  border-color: var(--accent-cyan);
  box-shadow: 0 0 0 2px var(--accent-cyan-glow);
}

.drive-locked {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 12px;
  background: var(--bg-emerald-btn);
  border: 1px solid var(--border-emerald-btn);
  border-radius: var(--radius-sm);
  font-size: 13px;
  font-weight: 600;
  color: var(--text-emerald-btn);
}

.drive-letter-icon {
  width: 16px;
  height: 16px;
}

.checkbox-row {
  display: flex;
  align-items: center;
  gap: 8px;
  cursor: pointer;
  margin-top: 4px;
}

.custom-checkbox {
  width: 16px;
  height: 16px;
  border-radius: 4px;
  border: 1px solid var(--border-card);
  background: var(--bg-input);
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.2s;
}

.custom-checkbox.checked {
  background: var(--accent-cyan);
  border-color: var(--accent-cyan);
}

.check-icon {
  width: 12px;
  height: 12px;
  color: #ffffff;
  stroke-width: 3;
}

.checkbox-label {
  font-size: 12px;
  color: var(--text-secondary);
}

.card-footer {
  padding: 14px 18px;
  background: var(--bg-card-footer);
  border-top: 1px solid var(--border-subtle);
  display: flex;
  align-items: center;
  gap: 10px;
}

/* Glass Card & Sync Grid */
.sync-panel-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 20px;
}

.glass-card {
  background: var(--bg-card);
  backdrop-filter: blur(16px);
  border: 1px solid var(--border-card);
  border-radius: var(--radius-lg);
  padding: 22px;
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.card-title {
  display: flex;
  align-items: center;
  gap: 10px;
  font-size: 15px;
  font-weight: 600;
  color: var(--text-main);
  padding-bottom: 12px;
  border-bottom: 1px solid var(--border-subtle);
}

.card-title-icon {
  width: 18px;
  height: 18px;
  color: var(--accent-cyan);
}

.form-group {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.action-buttons-row {
  display: flex;
  gap: 12px;
  margin-top: 8px;
}

.action-buttons-row .btn {
  flex: 1;
}

/* Diff Report Box */
.diff-report-box {
  background: var(--bg-card-footer);
  border: 1px solid var(--border-subtle);
  border-radius: var(--radius-md);
  padding: 14px;
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.diff-stats-row {
  display: flex;
  gap: 10px;
}

.stat-pill {
  padding: 4px 10px;
  border-radius: 12px;
  font-size: 12px;
  font-weight: 600;
}

.stat-source {
  background: rgba(56, 189, 248, 0.15);
  color: #38bdf8;
}

.stat-dest {
  background: rgba(245, 158, 11, 0.15);
  color: #fbbf24;
}

.diff-list-container {
  max-height: 140px;
  overflow-y: auto;
  font-family: 'JetBrains Mono', monospace;
  font-size: 11px;
  background: var(--bg-table-container);
  color: var(--text-main);
  padding: 8px;
  border-radius: 4px;
}

.diff-line {
  line-height: 1.6;
}

.diff-add {
  color: #34d399;
}

.diff-remove {
  color: #fb7185;
}

.diff-mod {
  color: #fbbf24;
}

/* Task Log Box */
.task-log-box {
  display: flex;
  flex-direction: column;
  gap: 8px;
  flex: 1;
}

.log-header {
  font-size: 12px;
  font-weight: 600;
  color: var(--text-secondary);
}

.log-content {
  background: var(--bg-table-container);
  border: 1px solid var(--border-subtle);
  border-radius: var(--radius-sm);
  padding: 12px;
  font-family: 'JetBrains Mono', monospace;
  font-size: 11px;
  color: var(--text-main);
  white-space: pre-wrap;
  word-break: break-all;
  height: 160px;
  overflow-y: auto;
}

/* Scheduler Table */
.task-table-wrapper {
  background: var(--bg-card);
  border: 1px solid var(--border-card);
  border-radius: var(--radius-lg);
  overflow: hidden;
}

.task-table {
  width: 100%;
  border-collapse: collapse;
  font-size: 13px;
  text-align: left;
}

.task-table th,
.task-table td {
  padding: 14px 18px;
  border-bottom: 1px solid var(--border-subtle);
}

.task-table th {
  background: var(--bg-table-header);
  color: var(--text-secondary);
  font-weight: 600;
  font-size: 12px;
}

.status-pill {
  padding: 4px 8px;
  border-radius: 12px;
  font-size: 11px;
  font-weight: 500;
}

.pill-active {
  background: var(--bg-emerald-btn);
  color: var(--text-emerald-btn);
}

.pill-disabled {
  background: var(--bg-status-unmounted);
  color: var(--text-muted);
}

.table-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}

.btn-sm {
  padding: 4px 8px;
  font-size: 11px;
}

/* Web-GUI Iframe */
.webgui-frame-box {
  flex: 1;
  background: var(--bg-card);
  border: 1px solid var(--border-card);
  border-radius: var(--radius-lg);
  overflow: hidden;
  position: relative;
  min-height: 480px;
}

.webgui-iframe {
  width: 100%;
  height: 100%;
  border: none;
}

.webgui-placeholder {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  text-align: center;
  height: 100%;
  padding: 40px;
}

.placeholder-icon {
  width: 64px;
  height: 64px;
  color: var(--accent-cyan);
  margin-bottom: 16px;
  opacity: 0.8;
}

/* Empty State */
.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  text-align: center;
  padding: 80px 20px;
}

.empty-icon-box {
  width: 72px;
  height: 72px;
  border-radius: 50%;
  background: rgba(56, 189, 248, 0.1);
  display: flex;
  align-items: center;
  justify-content: center;
  margin-bottom: 20px;
  border: 1px solid rgba(56, 189, 248, 0.2);
}

.empty-icon {
  width: 36px;
  height: 36px;
  color: var(--accent-cyan);
}

.empty-state h2 {
  font-size: 18px;
  font-weight: 600;
  color: var(--text-main);
  margin-bottom: 10px;
}

.empty-state p {
  font-size: 14px;
  color: var(--text-secondary);
  line-height: 1.6;
  max-width: 480px;
  margin-bottom: 24px;
}

/* Buttons */
.btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  padding: 8px 14px;
  font-size: 13px;
  font-weight: 500;
  border-radius: var(--radius-md);
  border: 1px solid transparent;
}

.btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.btn-icon {
  width: 16px;
  height: 16px;
}

.btn-primary {
  background: linear-gradient(135deg, #0284c7, #2563eb);
  color: #fff;
  box-shadow: 0 2px 8px rgba(2, 132, 199, 0.35);
}

.btn-primary:hover:not(:disabled) {
  background: linear-gradient(135deg, #0369a1, #1d4ed8);
}

.btn-mount-primary {
  width: 100%;
  background: linear-gradient(135deg, #0284c7, #6366f1);
  color: #fff;
  padding: 10px;
  font-weight: 600;
}

.btn-mount-primary:hover:not(:disabled) {
  filter: brightness(1.15);
  box-shadow: 0 4px 15px rgba(2, 132, 199, 0.4);
}

.btn-secondary {
  background: var(--bg-btn-secondary);
  color: var(--text-main);
  border-color: var(--border-subtle);
}

.btn-secondary:hover:not(:disabled) {
  background: var(--bg-btn-secondary-hover);
}

.btn-emerald {
  flex: 1;
  background: var(--bg-emerald-btn);
  color: var(--text-emerald-btn);
  border: 1px solid var(--border-emerald-btn);
}

.btn-emerald:hover:not(:disabled) {
  filter: brightness(1.08);
}

.btn-danger {
  background: var(--bg-danger-btn);
  color: var(--text-danger-btn);
  border: 1px solid var(--border-danger-btn);
}

.btn-danger:hover:not(:disabled) {
  filter: brightness(1.08);
}

.btn-danger-outline {
  background: transparent;
  color: var(--text-danger-btn);
  border: 1px solid var(--border-danger-btn);
}

.btn-danger-outline:hover:not(:disabled) {
  background: var(--bg-danger-btn);
}

.btn-lg {
  padding: 12px 24px;
  font-size: 15px;
}

/* Modal */
.modal-backdrop {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.75);
  backdrop-filter: blur(8px);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 100;
}

.modal-card {
  width: 520px;
  background: var(--bg-modal);
  color: var(--text-main);
  border: 1px solid var(--border-card);
  border-radius: var(--radius-lg);
  box-shadow: var(--shadow-lg);
  overflow: hidden;
  max-height: 90vh;
  display: flex;
  flex-direction: column;
}

.modal-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 18px 24px;
  border-bottom: 1px solid var(--border-subtle);
}

.modal-title {
  display: flex;
  align-items: center;
  gap: 10px;
  font-size: 16px;
  font-weight: 600;
  color: var(--text-main);
}

.modal-title-icon {
  width: 20px;
  height: 20px;
  color: var(--accent-cyan);
}

.close-btn {
  background: transparent;
  color: var(--text-muted);
  padding: 4px;
}

.close-btn:hover {
  color: var(--text-main);
}

.close-icon {
  width: 20px;
  height: 20px;
}

.modal-body {
  padding: 24px;
  display: flex;
  flex-direction: column;
  gap: 18px;
  overflow-y: auto;
}

.setting-group {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.group-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-main);
}

.group-desc {
  font-size: 12px;
  color: var(--text-muted);
  line-height: 1.5;
}

.input-with-button {
  display: flex;
  gap: 8px;
}

.input-with-button .modal-input {
  flex: 1;
}

.switch-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.switch-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-main);
}

.toggle-switch {
  width: 48px;
  height: 26px;
  background: var(--bg-toggle);
  border-radius: 13px;
  position: relative;
  cursor: pointer;
}

.toggle-slider {
  position: absolute;
  top: 3px;
  left: 3px;
  width: 20px;
  height: 20px;
  border-radius: 50%;
  background: #fff;
  transition: transform 0.2s;
}

.toggle-switch.active {
  background: var(--accent-cyan);
}

.toggle-switch.active .toggle-slider {
  transform: translateX(22px);
}

.info-box {
  background: rgba(56, 189, 248, 0.05);
  border: 1px solid rgba(56, 189, 248, 0.15);
  border-radius: var(--radius-md);
  padding: 12px 14px;
}

.info-title {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  font-weight: 600;
  color: var(--accent-cyan);
}

.info-icon {
  width: 15px;
  height: 15px;
}

.link {
  color: var(--accent-cyan);
  text-decoration: underline;
}

.modal-footer {
  padding: 16px 24px;
  background: var(--bg-modal-footer);
  border-top: 1px solid var(--border-subtle);
  display: flex;
  justify-content: flex-end;
  gap: 10px;
}

/* Toast */
.toast-card {
  position: fixed;
  bottom: 24px;
  right: 24px;
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 12px 18px;
  border-radius: var(--radius-md);
  font-size: 13px;
  font-weight: 500;
  box-shadow: var(--shadow-lg);
  z-index: 200;
  backdrop-filter: blur(12px);
}

.toast-icon {
  width: 18px;
  height: 18px;
}

.toast-success {
  background: rgba(6, 78, 59, 0.9);
  color: #6ee7b7;
  border: 1px solid rgba(16, 185, 129, 0.4);
}

.toast-error {
  background: rgba(136, 19, 55, 0.9);
  color: #fda4af;
  border: 1px solid rgba(244, 63, 94, 0.4);
}

.toast-info {
  background: var(--bg-modal);
  color: var(--text-main);
  border: 1px solid var(--border-subtle);
}

.toast-enter-active,
.toast-leave-active {
  transition: all 0.25s cubic-bezier(0.16, 1, 0.3, 1);
}

.toast-enter-from,
.toast-leave-to {
  opacity: 0;
  transform: translateY(12px) scale(0.95);
}

.spin-anim {
  animation: spin 1s linear infinite;
}

@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}

/* --- RcloneView Plus & Path Selection Styles --- */
.field-label-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 6px;
}

.quick-links-group {
  display: flex;
  align-items: center;
  gap: 6px;
}

.quick-link-label {
  font-size: 11px;
  color: var(--text-muted);
}

.btn-tag {
  background: rgba(56, 189, 248, 0.1);
  color: var(--accent-cyan);
  border: 1px solid rgba(56, 189, 248, 0.25);
  border-radius: 4px;
  padding: 1px 6px;
  font-size: 11px;
  cursor: pointer;
  transition: all 0.15s;
}

.btn-tag:hover {
  background: rgba(56, 189, 248, 0.25);
  border-color: var(--accent-cyan);
}

.input-with-actions {
  display: flex;
  gap: 8px;
  align-items: center;
}

.input-with-actions .modal-input {
  flex: 1;
}

.btn-browse {
  background: rgba(56, 189, 248, 0.12);
  color: var(--accent-cyan);
  border: 1px solid rgba(56, 189, 248, 0.3);
  padding: 8px 12px;
  font-size: 12px;
  white-space: nowrap;
  border-radius: var(--radius-sm);
  display: inline-flex;
  align-items: center;
  gap: 6px;
  cursor: pointer;
  transition: all 0.2s;
}

.btn-browse:hover {
  background: rgba(56, 189, 248, 0.25);
  border-color: var(--accent-cyan);
  box-shadow: 0 0 10px rgba(56, 189, 248, 0.2);
}

.btn-browse-cloud {
  background: rgba(168, 85, 247, 0.12);
  color: #c084fc;
  border: 1px solid rgba(168, 85, 247, 0.3);
  padding: 8px 12px;
  font-size: 12px;
  white-space: nowrap;
  border-radius: var(--radius-sm);
  display: inline-flex;
  align-items: center;
  gap: 6px;
  cursor: pointer;
  transition: all 0.2s;
}

.btn-browse-cloud:hover {
  background: rgba(168, 85, 247, 0.25);
  border-color: #c084fc;
  box-shadow: 0 0 10px rgba(168, 85, 247, 0.2);
}

.quick-filters-row {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  margin-top: 6px;
}

.quick-filter-tag {
  background: var(--bg-card-footer);
  border: 1px dashed var(--border-subtle);
  border-radius: 4px;
  padding: 2px 7px;
  font-size: 11px;
  color: var(--text-muted);
  cursor: pointer;
  transition: all 0.15s;
}

.quick-filter-tag:hover {
  border-color: var(--accent-cyan);
  color: var(--accent-cyan);
  background: rgba(56, 189, 248, 0.08);
}

/* Compare Card Header */
.compare-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 12px;
  padding-bottom: 10px;
  border-bottom: 1px solid var(--border-subtle);
}

.compare-header-title {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 15px;
  font-weight: 600;
  color: var(--text-primary);
}

.btn-xs {
  padding: 3px 8px;
  font-size: 11px;
  border-radius: 4px;
}

.btn-icon-xs {
  width: 12px;
  height: 12px;
}

.btn-schedule {
  background: linear-gradient(135deg, #059669 0%, #10b981 100%);
  color: white;
  border: none;
  font-weight: 600;
  box-shadow: 0 4px 14px rgba(16, 185, 129, 0.35);
  transition: all 0.2s;
}

.btn-schedule:hover:not(:disabled) {
  background: linear-gradient(135deg, #047857 0%, #059669 100%);
  transform: translateY(-1px);
  box-shadow: 0 6px 18px rgba(16, 185, 129, 0.45);
}

.btn-schedule-outline {
  background: rgba(16, 185, 129, 0.12);
  color: #34d399;
  border: 1px solid rgba(16, 185, 129, 0.4);
  transition: all 0.15s;
}

.btn-schedule-outline:hover {
  background: rgba(16, 185, 129, 0.25);
  border-color: #34d399;
}

/* --- RcloneView Classical Two-Column View & Compact Setup Styles --- */
.setup-card-compact {
  padding: 16px 20px;
  margin-bottom: 14px;
}

.setup-compact-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 12px;
  padding-bottom: 8px;
  border-bottom: 1px solid var(--border-subtle);
}

.setup-compact-title {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 14px;
  font-weight: 600;
  color: var(--text-primary);
}

.setup-compact-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}

.setup-inputs-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 16px;
  margin-bottom: 12px;
}

.form-group-compact {
  display: flex;
  flex-direction: column;
}

.setup-options-row {
  display: flex;
  gap: 14px;
  align-items: flex-end;
  background: var(--bg-card-footer);
  padding: 10px 14px;
  border-radius: var(--radius-sm);
  border: 1px solid var(--border-subtle);
}

.option-item {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.flex-2 { flex: 2; }
.flex-3 { flex: 3; }

.field-label-compact {
  font-size: 11px;
  font-weight: 600;
  color: var(--text-muted);
}

.modal-input-compact {
  background: var(--bg-input);
  border: 1px solid var(--border-subtle);
  border-radius: var(--radius-sm);
  padding: 6px 10px;
  color: var(--text-primary);
  font-size: 12px;
  outline: none;
}

.modal-input-compact:focus {
  border-color: var(--accent-cyan);
}

.checkbox-center {
  display: flex;
  flex-direction: row;
  align-items: center;
  cursor: pointer;
  padding-top: 18px;
}

/* RcloneView Main Compare View (全寬卡片) */
.rcloneview-main-card {
  padding: 16px;
  display: flex;
  flex-direction: column;
  min-height: 480px;
}

.rcloneview-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 10px;
}

.rcloneview-title-group {
  display: flex;
  align-items: center;
  gap: 12px;
}

.rcloneview-title {
  font-size: 16px;
  font-weight: 700;
  color: var(--text-primary);
}

.rcloneview-count-badge {
  font-size: 11px;
  color: var(--text-muted);
  background: var(--bg-card-footer);
  padding: 2px 8px;
  border-radius: 9999px;
  border: 1px solid var(--border-subtle);
}

.rcloneview-header-right {
  display: flex;
  align-items: center;
  gap: 8px;
}

/* RcloneView Blue Path Header (如 RcloneView 截圖頂部的深藍色左右條) */
.rcloneview-path-header {
  display: grid;
  grid-template-columns: 1fr 1fr;
  background: #0284c7; /* RcloneView signature blue */
  border-radius: var(--radius-sm) var(--radius-sm) 0 0;
  color: #ffffff;
  overflow: hidden;
  box-shadow: 0 2px 6px rgba(0, 0, 0, 0.2);
}

.path-col {
  display: flex;
  align-items: center;
  padding: 6px 12px;
  font-size: 12px;
  font-weight: 600;
  gap: 6px;
  overflow: hidden;
}

.path-src {
  border-right: 1px solid rgba(255, 255, 255, 0.25);
}

.path-star, .path-triangle {
  font-size: 12px;
  flex-shrink: 0;
}

.path-name {
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  font-family: monospace;
  letter-spacing: -0.2px;
  flex: 1;
}

.path-icon-right {
  width: 14px;
  height: 14px;
  color: #fef08a;
  flex-shrink: 0;
}

/* RcloneView Toolbar: Display filters, buttons, search */
.rcloneview-action-toolbar {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 8px;
  background: var(--bg-toolbar);
  border-left: 1px solid var(--border-subtle);
  border-right: 1px solid var(--border-subtle);
  border-bottom: 1px solid var(--border-subtle);
  padding: 6px 10px;
}

.display-group {
  display: flex;
  align-items: center;
  gap: 5px;
}

.toolbar-label {
  font-size: 11px;
  font-weight: 700;
  color: var(--text-muted);
  margin-right: 2px;
}

.rv-toggle-btn {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 3px 8px;
  border-radius: 4px;
  font-size: 11px;
  font-weight: 600;
  cursor: pointer;
  border: 1px solid transparent;
  background: var(--bg-btn-secondary);
  color: var(--text-muted);
  transition: all 0.15s;
}

.rv-icon {
  width: 12px;
  height: 12px;
}

.rv-toggle-btn.active.toggle-src {
  background: rgba(16, 185, 129, 0.2);
  border-color: rgba(16, 185, 129, 0.6);
  color: #34d399;
}

.rv-toggle-btn.active.toggle-dest {
  background: rgba(56, 189, 248, 0.2);
  border-color: rgba(56, 189, 248, 0.6);
  color: #38bdf8;
}

.rv-toggle-btn.active.toggle-eq {
  background: rgba(148, 163, 184, 0.2);
  border-color: rgba(148, 163, 184, 0.6);
  color: #e2e8f0;
}

.rv-toggle-btn.active.toggle-diff {
  background: rgba(168, 85, 247, 0.2);
  border-color: rgba(168, 85, 247, 0.6);
  color: #d8b4fe;
}

.rv-toggle-btn.active.toggle-err {
  background: rgba(244, 63, 94, 0.2);
  border-color: rgba(244, 63, 94, 0.6);
  color: #fb7185;
}

.actions-group {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-left: 8px;
}

.search-group {
  margin-left: auto;
}

.rv-search-box {
  display: flex;
  align-items: center;
  gap: 6px;
  background: var(--bg-input);
  border: 1px solid var(--border-subtle);
  border-radius: var(--radius-sm);
  padding: 3px 8px;
}

.rv-search-icon {
  width: 12px;
  height: 12px;
  color: var(--text-muted);
}

.rv-search-input {
  background: transparent;
  border: none;
  color: var(--text-primary);
  font-size: 11px;
  width: 160px;
  outline: none;
}

/* RcloneView Classical Table Styling */
.rcloneview-table-container {
  flex: 1;
  max-height: 480px;
  overflow-y: auto;
  border-left: 1px solid var(--border-subtle);
  border-right: 1px solid var(--border-subtle);
  background: var(--bg-table-container);
}

.rv-table {
  width: 100%;
  border-collapse: collapse;
  font-size: 11.5px;
}

.rv-thead-row {
  position: sticky;
  top: 0;
  background: var(--bg-table-header);
  z-index: 10;
  border-bottom: 1px solid var(--border-subtle);
}

.rv-thead-row th {
  padding: 6px 8px;
  font-size: 11px;
  font-weight: 600;
  color: var(--text-muted);
  text-align: left;
  border-right: 1px solid rgba(255, 255, 255, 0.05);
}

/* Column Width Distributions */
.col-chk { width: 32px; text-align: center; }
.col-src-name { width: 28%; }
.col-src-size { width: 8%; text-align: right; }
.col-src-date { width: 12%; }

.col-dir {
  width: 44px;
  text-align: center;
  background: var(--bg-table-header);
  border-left: 1px solid var(--border-subtle);
  border-right: 1px solid var(--border-subtle);
}

.col-dst-name { width: 28%; }
.col-dst-size { width: 8%; text-align: right; }
.col-dst-date { width: 12%; }
.col-act { width: 65px; text-align: center; }

/* Rows */
.rv-row {
  border-bottom: 1px solid rgba(255, 255, 255, 0.03);
  cursor: pointer;
  transition: background 0.1s;
}

.rv-row:hover {
  background: var(--bg-card-hover);
}

/* User's red-boxed selection effect (柔和淡藍色選中高亮背景) */
.rv-row.rv-row-selected {
  background: rgba(56, 189, 248, 0.16) !important;
  border-bottom-color: rgba(56, 189, 248, 0.3) !important;
}

.rv-row td {
  padding: 5px 8px;
  vertical-align: middle;
  border-right: 1px solid rgba(255, 255, 255, 0.03);
}

.rv-file-entry {
  display: flex;
  align-items: center;
  gap: 6px;
  overflow: hidden;
  white-space: nowrap;
}

.entry-icon {
  width: 14px;
  height: 14px;
  flex-shrink: 0;
}

.entry-name {
  font-family: monospace;
  color: var(--text-primary);
  overflow: hidden;
  text-overflow: ellipsis;
}

.entry-empty {
  color: var(--text-muted);
  font-style: italic;
}

.entry-placeholder {
  color: #64748b;
  font-size: 11px;
  font-style: italic;
}

.dir-icon-badge {
  display: flex;
  align-items: center;
  justify-content: center;
}

.dir-icon {
  width: 15px;
  height: 15px;
}

.text-amber { color: #f59e0b; }

.rv-empty-cell {
  text-align: center;
  padding: 40px;
}

.placeholder-height {
  padding: 80px 20px;
}

.rv-empty-box {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 8px;
  color: var(--text-muted);
}

.rv-empty-box h4 {
  font-size: 15px;
  color: var(--text-primary);
}

.rv-empty-box p {
  font-size: 12px;
}

/* Classical RcloneView Bottom Statusbar */
.rcloneview-statusbar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  background: var(--bg-statusbar);
  border: 1px solid var(--border-subtle);
  border-top: none;
  border-radius: 0 0 var(--radius-sm) var(--radius-sm);
  padding: 5px 12px;
  font-size: 11.5px;
  color: var(--text-muted);
  font-weight: 500;
}

.selected-count-tag {
  color: var(--accent-cyan);
  font-weight: 700;
}

/* Task Log Details Drawer */
.task-log-details {
  margin-top: 10px;
  background: var(--bg-card);
  border: 1px solid var(--border-subtle);
  border-radius: var(--radius-sm);
  overflow: hidden;
}

.log-summary {
  padding: 8px 12px;
  font-size: 12px;
  font-weight: 600;
  color: var(--text-secondary);
  cursor: pointer;
  user-select: none;
  display: flex;
  align-items: center;
  justify-content: space-between;
  background: var(--bg-card-footer);
}

.log-summary-hint {
  font-size: 11px;
  color: var(--accent-cyan);
}

.log-content-pre {
  padding: 10px 12px;
  font-family: monospace;
  font-size: 11px;
  color: var(--text-secondary);
  max-height: 180px;
  overflow-y: auto;
  white-space: pre-wrap;
  margin: 0;
}

.diff-empty-placeholder {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 40px 20px;
  text-align: center;
}

.diff-empty-placeholder .empty-icon {
  width: 48px;
  height: 48px;
  color: var(--text-muted);
  opacity: 0.6;
  margin-bottom: 8px;
}

.diff-empty-placeholder h4 {
  font-size: 14px;
  color: var(--text-primary);
  margin-bottom: 4px;
}

.diff-empty-placeholder p {
  font-size: 12px;
  color: var(--text-muted);
}

/* Cloud Directory Browser Modal */
.cloud-browser-modal {
  max-width: 580px;
}

.cloud-breadcrumb-bar {
  display: flex;
  align-items: center;
  gap: 8px;
  background: var(--bg-card-footer);
  padding: 8px 12px;
  border-radius: var(--radius-sm);
  margin-bottom: 10px;
  border: 1px solid var(--border-subtle);
}

.cloud-current-path {
  font-family: monospace;
  font-size: 12px;
  color: var(--accent-cyan);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.cloud-dirs-container {
  height: 240px;
  overflow-y: auto;
  background: var(--bg-card);
  border: 1px solid var(--border-subtle);
  border-radius: var(--radius-sm);
  padding: 6px;
  margin-bottom: 10px;
}

.cloud-dirs-loading,
.cloud-dirs-empty {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  height: 100%;
  color: var(--text-muted);
  font-size: 12px;
}

.cloud-dirs-list {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.cloud-dir-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 10px;
  border-radius: 4px;
  cursor: pointer;
  transition: all 0.12s;
  background: var(--bg-card-footer);
}

.cloud-dir-item:hover {
  background: rgba(56, 189, 248, 0.1);
  color: var(--accent-cyan);
}

.cloud-dir-icon {
  width: 15px;
  height: 15px;
  color: #f59e0b;
}

.cloud-dir-name {
  font-size: 12px;
  font-weight: 500;
}

/* Multi-destination styling */
.badge-tag {
  font-size: 10px;
  font-weight: 600;
  padding: 2px 6px;
  border-radius: 4px;
  background: rgba(255, 255, 255, 0.1);
  color: var(--text-muted);
  border: 1px solid rgba(255, 255, 255, 0.15);
}

.badge-cyan {
  background: rgba(56, 189, 248, 0.15);
  color: #38bdf8;
  border-color: rgba(56, 189, 248, 0.3);
}

.dest-label-with-tag {
  display: flex;
  align-items: center;
  gap: 8px;
}

.destinations-col {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.extra-dest-group {
  padding-top: 10px;
  border-top: 1px dashed var(--border-card);
}

.add-destination-row {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-top: 4px;
  margin-bottom: 4px;
  flex-wrap: wrap;
}

.btn-add-dest {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 7px 15px;
  font-size: 12px;
  font-weight: 600;
  border-radius: var(--radius-md);
  background: linear-gradient(135deg, #f59e0b, #ea580c);
  color: #ffffff;
  border: 1px solid rgba(245, 158, 11, 0.5);
  box-shadow: 0 2px 8px rgba(245, 158, 11, 0.3);
  cursor: pointer;
  transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
}

.btn-add-dest:hover {
  filter: brightness(1.12);
  box-shadow: 0 4px 12px rgba(245, 158, 11, 0.45);
  transform: translateY(-1px);
}

.btn-add-dest:active {
  transform: translateY(0);
}

.dest-count-hint {
  font-size: 11px;
  color: var(--text-muted);
}

/* Update Modal Styling */
.update-modal-card {
  max-width: 580px;
}

.update-banner {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 16px;
  border-radius: var(--radius-md);
  background: linear-gradient(135deg, rgba(56, 189, 248, 0.12), rgba(16, 185, 129, 0.08));
  border: 1px solid rgba(56, 189, 248, 0.25);
  margin-bottom: 14px;
}

.update-version-tag {
  display: flex;
  align-items: center;
  gap: 10px;
  font-family: var(--font-mono);
  font-weight: 700;
}

.old-ver {
  color: var(--text-muted);
  font-size: 14px;
}

.ver-arrow {
  width: 16px;
  height: 16px;
  color: #38bdf8;
}

.new-ver {
  color: #10b981;
  font-size: 16px;
  background: rgba(16, 185, 129, 0.2);
  padding: 2px 8px;
  border-radius: 6px;
}

.update-date {
  font-size: 12px;
  color: var(--text-muted);
}

.update-release-title {
  font-size: 15px;
  font-weight: 700;
  color: var(--text-main);
  margin-bottom: 10px;
}

.update-notes-container {
  background: var(--bg-card);
  border: 1px solid var(--border-card);
  border-radius: var(--radius-md);
  padding: 12px;
  margin-bottom: 12px;
}

.update-notes-header {
  font-size: 12px;
  font-weight: 600;
  color: var(--text-muted);
  margin-bottom: 8px;
}

.update-notes-pre {
  max-height: 160px;
  overflow-y: auto;
  font-family: inherit;
  font-size: 12px;
  line-height: 1.6;
  color: var(--text-main);
  white-space: pre-wrap;
  word-break: break-word;
  margin: 0;
}

.update-tip {
  font-size: 12px;
  color: var(--text-muted);
  line-height: 1.5;
  margin: 0;
}

.btn-update-action {
  background: linear-gradient(135deg, #0284c7, #059669);
  color: #fff;
  border: none;
  font-weight: 600;
}

.btn-update-action:hover:not(:disabled) {
  filter: brightness(1.1);
}

@media (max-width: 900px) {
  .btn-text-hide-sm {
    display: none;
  }
}
</style>
