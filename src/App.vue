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
  DownloadCloud
} from "lucide-vue-next";

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
const customRclonePath = ref(localStorage.getItem("rclone_custom_path") || "C:\\rclone\\rclone.exe");

// Configuration map for remotes
const remoteConfigs = reactive({});
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
  dest: "",
  action: "sync",
  isChecking: false,
  isRunning: false,
  diffResult: null,
  taskLog: ""
});

// Scheduler State
const scheduledTasks = ref(JSON.parse(localStorage.getItem("rclone_scheduled_tasks") || "[]"));
const showAddTaskModal = ref(false);
const newTask = reactive({
  name: "",
  source: "",
  dest: "",
  action: "copy",
  intervalMinutes: 60,
  enabled: true
});

let schedulerTimer = null;

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
  if (t.includes("drive")) return { name: "Google Drive", color: "#4285F4", bg: "rgba(66, 133, 244, 0.15)" };
  if (t.includes("photo")) return { name: "Google Photos", color: "#EA4335", bg: "rgba(234, 67, 53, 0.15)" };
  if (t.includes("onedrive")) return { name: "OneDrive", color: "#0078D4", bg: "rgba(0, 120, 212, 0.15)" };
  if (t.includes("dropbox")) return { name: "Dropbox", color: "#0061FF", bg: "rgba(0, 97, 255, 0.15)" };
  if (t.includes("s3")) return { name: "Amazon S3", color: "#FF9900", bg: "rgba(255, 153, 0, 0.15)" };
  if (t.includes("webdav")) return { name: "WebDAV", color: "#10B981", bg: "rgba(16, 185, 129, 0.15)" };
  if (t.includes("ftp")) return { name: "FTP / SFTP", color: "#8B5CF6", bg: "rgba(139, 92, 246, 0.15)" };
  return { name: type || "雲端儲存", color: "#38BDF8", bg: "rgba(56, 189, 248, 0.15)" };
}

// Autostart toggle
async function checkAutostart() {
  try {
    autostartActive.value = await isEnabled();
  } catch (err) {
    console.error("檢查開機自啟失敗:", err);
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

    let driveIndex = 0;
    remoteList.forEach((r) => {
      if (!remoteConfigs[r.name]) {
        // Assign distinct available drive letters to each card
        const assigned = availableDrives.value[driveIndex] || "Z:";
        driveIndex++;

        remoteConfigs[r.name] = {
          driveLetter: r.mounted_drive || assigned,
          volname: r.name,
          cacheMode: "full",
          readOnly: false,
          autoMount: autoMountList.value.includes(r.name)
        };
      } else if (r.is_mounted && r.mounted_drive) {
        remoteConfigs[r.name].driveLetter = r.mounted_drive;
      }
    });

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
    await invoke("mount_remote", {
      rclonePath: customRclonePath.value.trim() || null,
      remote: remoteName,
      driveLetter: conf.driveLetter,
      volname: conf.volname || remoteName,
      cacheMode: conf.cacheMode || "full",
      readOnly: conf.readOnly || false
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
    const msg = await invoke("update_remote_gui", {
      rclonePath: customRclonePath.value.trim() || null,
      name: editRemoteForm.name,
      params
    });
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

// RcloneView Plus Compare / Check Diff
async function handleCheckDiff() {
  if (!syncForm.source || !syncForm.dest) {
    showToast("請先填寫來源與目標路徑！", "error");
    return;
  }
  syncForm.isChecking = true;
  syncForm.diffResult = null;
  try {
    const res = await invoke("check_folder_diff", {
      rclonePath: customRclonePath.value.trim() || null,
      source: syncForm.source.trim(),
      dest: syncForm.dest.trim()
    });
    syncForm.diffResult = res;
    showToast(res.message, "success");
  } catch (err) {
    showToast(`比對失敗: ${err}`, "error");
  } finally {
    syncForm.isChecking = false;
  }
}

// RcloneView Plus Run Sync Job
async function handleRunSync() {
  if (!syncForm.source || !syncForm.dest) {
    showToast("請先填寫來源與目標路徑！", "error");
    return;
  }
  syncForm.isRunning = true;
  syncForm.taskLog = "正在執行任務中，請稍候...";
  try {
    const log = await invoke("run_sync_task", {
      rclonePath: customRclonePath.value.trim() || null,
      action: syncForm.action,
      source: syncForm.source.trim(),
      dest: syncForm.dest.trim()
    });
    syncForm.taskLog = log;
    showToast("同步/備份任務執行成功！", "success");
  } catch (err) {
    syncForm.taskLog = `錯誤: ${err}`;
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
  showToast("已成功建立排程任務！", "success");
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
        await invoke("run_sync_task", {
          rclonePath: customRclonePath.value.trim() || null,
          action: task.action,
          source: task.source,
          dest: task.dest
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
}

function saveCustomPath() {
  localStorage.setItem("rclone_custom_path", customRclonePath.value);
  showToast("已儲存 Rclone 路徑設定！", "success");
  refreshAll();
}

const mountedCount = computed(() => remotes.value.filter((r) => r.is_mounted).length);
const totalCount = computed(() => remotes.value.length);
const isPrerequisiteMissing = computed(() => !envStatus.value.rclone_found || !envStatus.value.winfsp_found);

onMounted(async () => {
  await checkAutostart();
  await refreshAll(true);
  schedulerTimer = setInterval(runSchedulerCycle, 60000);
});

onUnmounted(() => {
  if (schedulerTimer) clearInterval(schedulerTimer);
});
</script>

<template>
  <div class="app-layout">
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
        </div>

        <div class="prereq-cards-row">
          <!-- Rclone Missing Card -->
          <div v-if="!envStatus.rclone_found" class="prereq-item-card">
            <div class="prereq-card-text">
              <span class="prereq-badge">必要元件 1</span>
              <h4>Rclone 核心執行檔</h4>
              <p>預設檢查路徑 <code>C:\rclone\rclone.exe</code> 尚未找到執行檔。</p>
            </div>
            <button class="btn btn-download" @click="openUrl('https://rclone.org/downloads/')">
              <DownloadCloud class="btn-icon" />
              <span>前往 Rclone 官網下載</span>
            </button>
          </div>

          <!-- WinFsp Missing Card -->
          <div v-if="!envStatus.winfsp_found" class="prereq-item-card">
            <div class="prereq-card-text">
              <span class="prereq-badge">必要元件 2</span>
              <h4>WinFsp 檔案系統核心</h4>
              <p>Windows 虛擬檔案系統驅動，未安裝將無法建立本機磁碟代號。</p>
            </div>
            <div class="prereq-btn-group">
              <button class="btn btn-download" @click="openUrl('https://winfsp.dev/')">
                <DownloadCloud class="btn-icon" />
                <span>WinFsp 官方網站</span>
              </button>
              <button class="btn btn-download-alt" @click="openUrl('https://github.com/winfsp/winfsp/releases/latest')">
                <ExternalLink class="btn-icon" />
                <span>GitHub 下載安裝檔 (.msi)</span>
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
        <span v-else class="clickable-link" @click="openUrl('https://rclone.org/downloads/')">
          Rclone: 尚未安裝 (點此下載)
        </span>
      </div>

      <div class="env-item" :class="{ 'env-ok': envStatus.winfsp_found, 'env-warn': !envStatus.winfsp_found }">
        <component :is="envStatus.winfsp_found ? ShieldCheck : AlertTriangle" class="env-icon" />
        <span v-if="envStatus.winfsp_found">WinFsp: 已就緒 (正常運作)</span>
        <span v-else class="clickable-link" @click="openUrl('https://winfsp.dev/')">
          WinFsp: 尚未安裝 (點此下載安裝)
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
                  class="field-input"
                  placeholder="檔案總管中顯示的名稱"
                />
              </div>

              <!-- Cache Mode -->
              <div class="setting-row" v-if="!remote.is_mounted">
                <label class="field-label">快取模式</label>
                <select v-model="remoteConfigs[remote.name].cacheMode" class="field-select">
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
            <p>可比對兩個路徑（本機或雲端）的檔案差異，並執行單向鏡像同步或增量備份</p>
          </div>
        </div>

        <div class="sync-panel-grid">
          <!-- Setup Card -->
          <div class="glass-card">
            <h3 class="card-title">
              <FolderSync class="card-title-icon" />
              <span>任務路徑設定</span>
            </h3>

            <div class="form-group">
              <label class="field-label">來源路徑 (Source)</label>
              <input
                type="text"
                v-model="syncForm.source"
                class="modal-input"
                placeholder="例如：GDrive_alanytp100:Documents 或 D:\MyData"
              />
              <span class="field-hint">可輸入已建立的雲端（如 GDrive:）或本機資料夾路徑</span>
            </div>

            <div class="form-group">
              <label class="field-label">目的路徑 (Destination)</label>
              <input
                type="text"
                v-model="syncForm.dest"
                class="modal-input"
                placeholder="例如：D:\Backup\GDrive 或 GPhoto_alanytp100:Backup"
              />
            </div>

            <div class="form-group">
              <label class="field-label">任務動作</label>
              <select v-model="syncForm.action" class="modal-input">
                <option value="sync">Sync (單向鏡像同步: 目的端檔案會與來源端完全一致)</option>
                <option value="copy">Copy (增量複製備份: 僅複製新檔/更新檔，不刪除目的端舊檔)</option>
              </select>
            </div>

            <div class="action-buttons-row">
              <button
                class="btn btn-secondary"
                @click="handleCheckDiff"
                :disabled="syncForm.isChecking || syncForm.isRunning || isPrerequisiteMissing"
              >
                <FileCheck class="btn-icon" :class="{ 'spin-anim': syncForm.isChecking }" />
                <span>{{ syncForm.isChecking ? '比對中...' : '比對兩端差異 (Check Diff)' }}</span>
              </button>

              <button
                class="btn btn-primary"
                @click="handleRunSync"
                :disabled="syncForm.isRunning || syncForm.isChecking || isPrerequisiteMissing"
              >
                <Play class="btn-icon" :class="{ 'spin-anim': syncForm.isRunning }" />
                <span>{{ syncForm.isRunning ? '執行任務中...' : '立即開始同步/備份' }}</span>
              </button>
            </div>
          </div>

          <!-- Diff & Log Card -->
          <div class="glass-card">
            <h3 class="card-title">
              <FileCheck class="card-title-icon" />
              <span>差異比對報告與執行記錄</span>
            </h3>

            <!-- Diff Summary Box -->
            <div v-if="syncForm.diffResult" class="diff-report-box">
              <div class="diff-stats-row">
                <div class="stat-pill stat-source">來源獨有: {{ syncForm.diffResult.total_source_files }} 檔</div>
                <div class="stat-pill stat-dest">目的端多出: {{ syncForm.diffResult.total_dest_files }} 檔</div>
              </div>
              <div class="diff-list-container">
                <div
                  v-for="(diffLine, idx) in syncForm.diffResult.differences"
                  :key="idx"
                  class="diff-line"
                  :class="{
                    'diff-add': diffLine.startsWith('+'),
                    'diff-remove': diffLine.startsWith('-'),
                    'diff-mod': diffLine.startsWith('*') || diffLine.startsWith('!')
                  }"
                >
                  {{ diffLine }}
                </div>
              </div>
            </div>

            <!-- Task Log -->
            <div class="task-log-box">
              <div class="log-header">即時執行日誌</div>
              <pre class="log-content">{{ syncForm.taskLog || '尚未執行任務，點選「立即開始同步/備份」即可檢視進度。' }}</pre>
            </div>
          </div>
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
                    <button class="btn btn-secondary btn-sm" @click="toggleTaskEnabled(task)">
                      {{ task.enabled ? '停用' : '啟用' }}
                    </button>
                    <button class="btn btn-danger btn-sm" @click="removeTask(task.id)">
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
            <span>修改雲端設定: {{ editRemoteForm.name }}</span>
          </div>
          <button class="close-btn" @click="showEditRemoteModal = false">
            <X class="close-icon" />
          </button>
        </div>

        <div class="modal-body">
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
            <input type="text" v-model="editRemoteForm.clientId" class="modal-input" placeholder="自訂 Google API Client ID" />
            <label class="group-title mt-2">自訂 Client Secret (選填)</label>
            <input type="password" v-model="editRemoteForm.clientSecret" class="modal-input" placeholder="自訂 Google API Client Secret" />
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
                <div class="switch-title">開機自動啟動 (常駐於 System Tray)</div>
                <div class="group-desc">登入 Windows 時自動在右下角系統匣常駐啟動</div>
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
  background: radial-gradient(circle at 10% 20%, #0d1322 0%, #080b12 90%);
}

/* Header */
.app-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 14px 28px;
  background: rgba(16, 22, 36, 0.85);
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
  background: linear-gradient(90deg, #f8fafc, #94a3b8);
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
  background: rgba(10, 15, 26, 0.6);
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
  color: #fde68a;
}

.prereq-cards-row {
  display: flex;
  gap: 16px;
  flex-wrap: wrap;
}

.prereq-item-card {
  flex: 1;
  min-width: 280px;
  background: rgba(17, 24, 39, 0.85);
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
  color: #f59e0b;
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.prereq-item-card h4 {
  font-size: 14px;
  font-weight: 600;
  color: #f8fafc;
}

.prereq-item-card p {
  font-size: 12px;
  color: #94a3b8;
  line-height: 1.4;
}

.prereq-item-card code {
  font-family: 'JetBrains Mono', monospace;
  background: rgba(0, 0, 0, 0.4);
  padding: 2px 6px;
  border-radius: 4px;
  color: #38bdf8;
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
  background: rgba(255, 255, 255, 0.1);
  color: #f8fafc;
  font-weight: 500;
  padding: 8px 14px;
  font-size: 12px;
  border-radius: var(--radius-sm);
  border: 1px solid rgba(255, 255, 255, 0.15);
  display: inline-flex;
  align-items: center;
  gap: 6px;
}

.btn-download-alt:hover {
  background: rgba(255, 255, 255, 0.15);
}

/* Environment Banner */
.env-banner {
  display: flex;
  align-items: center;
  gap: 20px;
  padding: 8px 28px;
  background: rgba(10, 15, 26, 0.6);
  border-bottom: 1px solid var(--border-subtle);
  font-size: 12px;
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
  color: #fbbf24;
}

.env-neutral {
  color: var(--text-secondary);
}

.clickable-link {
  color: #fbbf24;
  text-decoration: underline;
  cursor: pointer;
}

.clickable-link:hover {
  color: #f59e0b;
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
  border-color: rgba(255, 255, 255, 0.15);
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
  background: rgba(16, 185, 129, 0.15);
  color: #34d399;
  border: 1px solid rgba(16, 185, 129, 0.3);
}

.status-mounted .status-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: #10b981;
  box-shadow: 0 0 6px #10b981;
}

.status-unmounted {
  background: rgba(100, 116, 139, 0.15);
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
  background: rgba(15, 23, 42, 0.8);
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
  box-shadow: 0 0 0 2px rgba(56, 189, 248, 0.2);
}

.drive-locked {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 12px;
  background: rgba(16, 185, 129, 0.1);
  border: 1px solid rgba(16, 185, 129, 0.25);
  border-radius: var(--radius-sm);
  font-size: 13px;
  font-weight: 600;
  color: #34d399;
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
  background: rgba(15, 23, 42, 0.8);
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
  color: #0f172a;
  stroke-width: 3;
}

.checkbox-label {
  font-size: 12px;
  color: var(--text-secondary);
}

.card-footer {
  padding: 14px 18px;
  background: rgba(15, 23, 42, 0.4);
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
  background: rgba(15, 23, 42, 0.6);
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
  background: rgba(0, 0, 0, 0.3);
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
  background: #090d16;
  border: 1px solid var(--border-subtle);
  border-radius: var(--radius-sm);
  padding: 12px;
  font-family: 'JetBrains Mono', monospace;
  font-size: 11px;
  color: #cbd5e1;
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
  background: rgba(15, 23, 42, 0.5);
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
  background: rgba(16, 185, 129, 0.15);
  color: #34d399;
}

.pill-disabled {
  background: rgba(100, 116, 139, 0.15);
  color: #94a3b8;
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
  background: rgba(255, 255, 255, 0.05);
  color: var(--text-main);
  border-color: var(--border-subtle);
}

.btn-secondary:hover:not(:disabled) {
  background: rgba(255, 255, 255, 0.1);
}

.btn-emerald {
  flex: 1;
  background: rgba(16, 185, 129, 0.15);
  color: #34d399;
  border-color: rgba(16, 185, 129, 0.3);
}

.btn-emerald:hover:not(:disabled) {
  background: rgba(16, 185, 129, 0.25);
}

.btn-danger {
  background: rgba(244, 63, 94, 0.15);
  color: #fb7185;
  border-color: rgba(244, 63, 94, 0.3);
}

.btn-danger:hover:not(:disabled) {
  background: rgba(244, 63, 94, 0.25);
}

.btn-danger-outline {
  background: transparent;
  color: #fb7185;
  border-color: rgba(244, 63, 94, 0.4);
}

.btn-danger-outline:hover:not(:disabled) {
  background: rgba(244, 63, 94, 0.15);
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
  background: #111827;
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
}

.toggle-switch {
  width: 48px;
  height: 26px;
  background: #334155;
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
  background: rgba(15, 23, 42, 0.6);
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
  background: rgba(30, 41, 59, 0.9);
  color: #e2e8f0;
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
</style>
