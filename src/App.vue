<script setup lang="ts">
import { onMounted, onUnmounted, ref, computed } from "vue";
import { useI18n } from "./i18n";
import {
  startDownload,
  cancelDownload,
  listJobs,
  clearFinished,
  fetchInfo,
  ytdlpVersion,
  updateYtdlpNow,
  revealPath,
  defaultDownloadDir,
  onProgress,
  onStatus,
} from "./api";
import type { DownloadOptions, JobView, VideoInfo } from "./types";

const { locale, tr, trStatus, setLocale } = useI18n();

type Tab = "download" | "jobs" | "settings";
const tab = ref<Tab>("download");

// ---------------------------------------------------------------------------
// Download form state
// ---------------------------------------------------------------------------

const url = ref("");
const analyzing = ref(false);
const info = ref<VideoInfo | null>(null);
const infoError = ref<string | null>(null);

type Quality = "best" | "1080" | "720" | "480" | "audio";
const quality = ref<Quality>("best");
const outputDir = ref("");
const showAdvanced = ref(false);
const rawArgs = ref("");
const cookies = ref("");

const FORMAT_MAP: Record<Quality, string> = {
  best: "bv*+ba/b",
  "1080": "bv*[height<=1080]+ba/b[height<=1080]",
  "720": "bv*[height<=720]+ba/b[height<=720]",
  "480": "bv*[height<=480]+ba/b[height<=480]",
  audio: "bestaudio/b",
};

const startedThisSubmit = ref(false);

async function analyze() {
  const u = url.value.trim();
  if (!u) return;
  analyzing.value = true;
  infoError.value = null;
  info.value = null;
  try {
    info.value = await fetchInfo(u);
  } catch (e) {
    infoError.value = String(e);
  } finally {
    analyzing.value = false;
  }
}

async function submit() {
  const u = url.value.trim();
  if (!u) return;
  const opts: DownloadOptions = {
    format: FORMAT_MAP[quality.value],
  };
  if (outputDir.value.trim()) opts.outputDir = outputDir.value.trim();
  if (quality.value === "audio") {
    opts.rawArgs = [opts.rawArgs, "-x", "--audio-format mp3"]
      .filter(Boolean)
      .join(" ");
  }
  if (rawArgs.value.trim()) {
    opts.rawArgs = [opts.rawArgs, rawArgs.value.trim()]
      .filter(Boolean)
      .join(" ");
  }
  if (cookies.value.trim()) opts.cookies = cookies.value.trim();

  startedThisSubmit.value = true;
  try {
    const id = await startDownload(u, opts);
    // Insert immediately so progress/status events for this id land on a
    // visible job instead of being dropped by the `if (!j) return` guards.
    upsertJob({
      id,
      url: u,
      title: null,
      status: "queued",
      filepath: null,
      error: null,
      downloaded: 0,
      total: null,
      speed: null,
      eta: null,
    });
    url.value = "";
    info.value = null;
    infoError.value = null;
    tab.value = "jobs";
  } finally {
    startedThisSubmit.value = false;
  }
}

async function browse() {
  const { open } = await import("@tauri-apps/plugin-dialog");
  const selected = await open({ directory: true, multiple: false });
  if (typeof selected === "string") outputDir.value = selected;
}

// ---------------------------------------------------------------------------
// Job list state
// ---------------------------------------------------------------------------

const jobs = ref<Map<number, JobView>>(new Map());
const jobOrder = ref<number[]>([]);

function upsertJob(j: JobView) {
  if (!jobs.value.has(j.id)) jobOrder.value.push(j.id);
  jobs.value.set(j.id, j);
  // trigger reactivity on Map mutation
  jobs.value = new Map(jobs.value);
  jobOrder.value = [...jobOrder.value];
}

const jobList = computed<JobView[]>(() =>
  jobOrder.value
    .slice()
    .reverse()
    .map((id) => jobs.value.get(id)!)
    .filter(Boolean),
);

const activeCount = computed(
  () =>
    jobList.value.filter((j) =>
      ["queued", "resolving", "downloading"].includes(j.status),
    ).length,
);

let unlistenProgress: (() => void) | null = null;
let unlistenStatus: (() => void) | null = null;

onMounted(async () => {
  unlistenProgress = await onProgress((p) => {
    const j = jobs.value.get(p.id);
    if (!j) return;
    j.downloaded = p.downloaded;
    j.total = p.total;
    j.speed = p.speed;
    j.eta = p.eta;
    if (p.status === "downloading") j.status = "downloading";
    upsertJob(j);
  });
  unlistenStatus = await onStatus((p) => {
    const j = jobs.value.get(p.id);
    if (j) {
      j.status = p.status;
      if (p.message) j.error = p.message;
      if (p.title) j.title = p.title;
      if (p.filepath) j.filepath = p.filepath;
      upsertJob(j);
    }
  });
  const existing = await listJobs();
  existing.forEach((j) => upsertJob({ ...j, downloaded: 0, total: null, speed: null, eta: null }));
  // Pre-fill the save location with the user's Downloads folder once.
  if (!outputDir.value.trim()) {
    try {
      outputDir.value = await defaultDownloadDir();
    } catch {
      /* leave empty; backend still falls back to Downloads */
    }
  }
});

onUnmounted(() => {
  unlistenProgress?.();
  unlistenStatus?.();
});

async function cancel(id: number) {
  await cancelDownload(id);
}

async function clearDone() {
  await clearFinished();
  jobOrder.value = jobOrder.value.filter((id) => {
    const j = jobs.value.get(id);
    return j && !["finished", "error", "cancelled"].includes(j.status);
  });
  jobs.value = new Map(
    jobOrder.value.map((id) => [id, jobs.value.get(id)!]),
  );
}

// ---------------------------------------------------------------------------
// Settings state
// ---------------------------------------------------------------------------

const versionInfo = ref<Awaited<ReturnType<typeof ytdlpVersion>> | null>(null);
const checkingUpdate = ref(false);
const updatingKernel = ref(false);
const updateMsg = ref<string | null>(null);

async function checkUpdate() {
  checkingUpdate.value = true;
  updateMsg.value = null;
  try {
    versionInfo.value = await ytdlpVersion();
  } finally {
    checkingUpdate.value = false;
  }
}

async function doUpdate() {
  updatingKernel.value = true;
  updateMsg.value = null;
  try {
    const v = await updateYtdlpNow();
    updateMsg.value = v;
    versionInfo.value = await ytdlpVersion();
  } catch (e) {
    updateMsg.value = `ERROR:${String(e)}`;
  } finally {
    updatingKernel.value = false;
  }
}

// ---------------------------------------------------------------------------
// Formatting helpers
// ---------------------------------------------------------------------------

function fmtBytes(n: number | null | undefined): string {
  if (n == null || !isFinite(n)) return "--";
  const units = ["B", "KB", "MB", "GB"];
  let v = n;
  let i = 0;
  while (v >= 1024 && i < units.length - 1) {
    v /= 1024;
    i++;
  }
  return `${v.toFixed(v >= 100 || i === 0 ? 0 : 1)} ${units[i]}`;
}

function fmtSpeed(n: number | null | undefined): string {
  if (n == null || !isFinite(n)) return "--";
  return `${fmtBytes(n)}/s`;
}

function fmtEta(s: number | null | undefined): string {
  if (s == null || !isFinite(s)) return "--";
  const m = Math.floor(s / 60);
  const sec = Math.round(s % 60);
  return m > 0 ? `${m}m ${sec}s` : `${sec}s`;
}

function fmtDuration(s: number | null | undefined): string {
  if (s == null || !isFinite(s)) return "";
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const sec = Math.round(s % 60);
  return h > 0
    ? `${h}:${String(m).padStart(2, "0")}:${String(sec).padStart(2, "0")}`
    : `${m}:${String(sec).padStart(2, "0")}`;
}

const activeJobCount = computed(() => activeCount.value);
</script>

<template>
  <div class="app">
    <!-- ================= Header ================= -->
    <header class="titlebar">
      <div class="brand">
        <svg class="logo" viewBox="0 0 24 24" fill="none" aria-hidden="true">
          <path
            d="M12 3v12m0 0 4.5-4.5M12 15l-4.5-4.5"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
          />
          <path
            d="M4 17v2a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2v-2"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
          />
        </svg>
        <span class="brand-name">yt-dlp GUI</span>
      </div>
      <nav class="tabs">
        <button
          class="tab"
          :class="{ active: tab === 'download' }"
          @click="tab = 'download'"
        >
          {{ tr("tabDownload") }}
          <span v-if="activeJobCount > 0" class="dot" />
        </button>
        <button
          class="tab"
          :class="{ active: tab === 'jobs' }"
          @click="tab = 'jobs'"
        >
          {{ tr("tabHistory") }}
          <span v-if="activeJobCount > 0" class="badge-count">{{
            activeJobCount
          }}</span>
        </button>
        <button
          class="tab"
          :class="{ active: tab === 'settings' }"
          @click="tab = 'settings'"
        >
          {{ tr("tabSettings") }}
        </button>
      </nav>
      <div class="lang-switch">
        <button
          class="lang-btn"
          :class="{ active: locale === 'zh' }"
          @click="setLocale('zh')"
        >
          中
        </button>
        <button
          class="lang-btn"
          :class="{ active: locale === 'en' }"
          @click="setLocale('en')"
        >
          EN
        </button>
      </div>
    </header>

    <!-- ================= Download view ================= -->
    <main class="content" v-show="tab === 'download'">
      <section class="card input-card">
        <div class="url-row">
          <input
            class="url-input"
            v-model="url"
            :placeholder="tr('urlPlaceholder')"
            spellcheck="false"
            @keydown.enter="analyze"
          />
          <button class="btn ghost" :disabled="analyzing || !url.trim()" @click="analyze">
            {{ analyzing ? tr("analyzing") : tr("addUrl") }}
          </button>
        </div>

        <div v-if="infoError" class="info-error">
          <b>{{ tr("fetchInfoFailed") }}:</b> {{ infoError }}
        </div>

        <div v-if="info" class="preview">
          <img
            v-if="info.thumbnail"
            :src="info.thumbnail"
            class="thumb"
            referrerpolicy="no-referrer"
          />
          <div class="preview-meta">
            <div class="preview-title">{{ info.title }}</div>
            <div class="preview-sub">
              <span v-if="info.uploader">{{ info.uploader }}</span>
              <span v-if="info.duration">· {{ fmtDuration(info.duration) }}</span>
              <span v-if="info.formats">· {{ info.formats.length }} formats</span>
            </div>
          </div>
        </div>

        <div class="option-grid">
          <div class="field">
            <label class="field-label">{{ tr("quality") }}</label>
            <div class="seg">
              <button
                v-for="(label, q) in {
                  best: tr('qualityBest'),
                  '1080': tr('quality1080'),
                  '720': tr('quality720'),
                  '480': tr('quality480'),
                  audio: tr('qualityAudio'),
                }"
                :key="q"
                class="seg-btn"
                :class="{ active: quality === q }"
                @click="quality = q as Quality"
              >
                {{ label }}
              </button>
            </div>
          </div>

          <div class="field">
            <label class="field-label">{{ tr("outputDir") }}</label>
            <div class="dir-row">
              <input class="dir-input" v-model="outputDir" spellcheck="false" />
              <button class="btn ghost" @click="browse">{{ tr("chooseDir") }}</button>
            </div>
          </div>
        </div>

        <button class="adv-toggle" @click="showAdvanced = !showAdvanced">
          <span class="chev" :class="{ open: showAdvanced }">▸</span>
          {{ tr("advanced") }}
        </button>

        <div v-show="showAdvanced" class="advanced">
          <div class="field">
            <label class="field-label">{{ tr("rawArgs") }}</label>
            <input
              class="text-input"
              v-model="rawArgs"
              :placeholder="tr('rawArgsPlaceholder')"
              spellcheck="false"
            />
          </div>
          <div class="field">
            <label class="field-label">{{ tr("cookies") }}</label>
            <textarea
              class="textarea"
              v-model="cookies"
              :placeholder="tr('cookiesPlaceholder')"
              rows="4"
              spellcheck="false"
            />
          </div>
        </div>

        <div class="submit-row">
          <button
            class="btn primary"
            :disabled="!url.trim() || startedThisSubmit"
            @click="submit"
          >
            {{ tr("download") }}
          </button>
        </div>
      </section>
    </main>

    <!-- ================= Jobs view ================= -->
    <main class="content" v-show="tab === 'jobs'">
      <section class="card">
        <div class="jobs-head">
          <h2>{{ tr("queue") }}</h2>
          <button class="btn ghost small" @click="clearDone">
            {{ tr("clearFinished") }}
          </button>
        </div>
        <div v-if="jobList.length === 0" class="empty">{{ tr("noJobs") }}</div>
        <transition-group name="job" tag="div" class="job-list">
          <div v-for="j in jobList" :key="j.id" class="job" :data-status="j.status">
            <div class="job-main">
              <div class="job-title">
                {{ j.title || j.url }}
              </div>
              <div v-if="j.filepath" class="job-path" :title="j.filepath">
                {{ j.filepath }}
              </div>
              <div v-if="j.error" class="job-error" :title="j.error">
                {{ j.error }}
              </div>
              <div
                v-if="j.status === 'downloading' && j.total"
                class="job-progress-line"
              >
                <div class="bar">
                  <div
                    class="bar-fill"
                    :style="{ width: ((j.downloaded ?? 0) / j.total) * 100 + '%' }"
                  />
                </div>
                <span class="pct">
                  {{ Math.round(((j.downloaded ?? 0) / j.total) * 100) }}%
                </span>
              </div>
              <div v-if="j.status === 'downloading'" class="job-stats">
                <span>{{ fmtBytes(j.downloaded) }} / {{ fmtBytes(j.total) }}</span>
                <span>{{ fmtSpeed(j.speed) }}</span>
                <span>{{ tr("eta") }} {{ fmtEta(j.eta) }}</span>
              </div>
            </div>
            <div class="job-side">
              <span class="status-chip" :data-status="j.status">
                {{ trStatus(j.status) }}
              </span>
              <button
                v-if="['queued', 'resolving', 'downloading'].includes(j.status)"
                class="btn ghost small"
                @click="cancel(j.id)"
              >
                {{ tr("cancel") }}
              </button>
              <button
                v-if="j.filepath"
                class="btn ghost small"
                @click="revealPath(j.filepath!)"
              >
                {{ tr("openLocation") }}
              </button>
            </div>
          </div>
        </transition-group>
      </section>
    </main>

    <!-- ================= Settings view ================= -->
    <main class="content" v-show="tab === 'settings'">
      <section class="card">
        <h2>{{ tr("settings") }}</h2>
        <div class="setting-row">
          <div>
            <div class="field-label">{{ tr("language") }}</div>
            <div class="seg">
              <button
                class="seg-btn"
                :class="{ active: locale === 'zh' }"
                @click="setLocale('zh')"
              >
                中文
              </button>
              <button
                class="seg-btn"
                :class="{ active: locale === 'en' }"
                @click="setLocale('en')"
              >
                English
              </button>
            </div>
          </div>
        </div>

        <div class="setting-row">
          <div>
            <div class="field-label">{{ tr("ytdlpVersion") }}</div>
            <div class="version-line">
              <code class="version-code">{{
                versionInfo?.current || "…"
              }}</code>
              <button
                class="btn ghost small"
                :disabled="checkingUpdate"
                @click="checkUpdate"
              >
                {{ checkingUpdate ? tr("checking") : tr("checkUpdate") }}
              </button>
            </div>
            <div v-if="versionInfo" class="update-status">
              <template v-if="!versionInfo.updateAvailable">
                ✓ {{ tr("upToDate") }}
              </template>
              <template v-else>
                <span class="update-avail">
                  ↑ {{ tr("updateAvailable") }}: {{ versionInfo.latest }}
                </span>
                <button
                  class="btn primary small"
                  :disabled="updatingKernel"
                  @click="doUpdate"
                >
                  {{ updatingKernel ? tr("updating") : tr("updateNow") }}
                </button>
              </template>
            </div>
            <div v-if="updateMsg" class="update-msg" :class="{ err: updateMsg.startsWith('ERROR:') }">
              {{
                updateMsg.startsWith("ERROR:")
                  ? tr("updateFailed") + ": " + updateMsg.slice(6)
                  : tr("updateDone") + " " + updateMsg
              }}
            </div>
          </div>
        </div>
      </section>
    </main>
  </div>
</template>

<style>
:root {
  --bg: #f6f7f9;
  --card: #ffffff;
  --ink: #1a1d21;
  --muted: #6b7280;
  --line: #e5e7eb;
  --ac: #4f46e5;
  --ac-soft: #eef2ff;
  --ok: #059669;
  --err: #dc2626;
  --radius: 12px;
  font-family: "Segoe UI", "Microsoft YaHei", system-ui, sans-serif;
}
* {
  box-sizing: border-box;
  margin: 0;
  padding: 0;
}
body {
  background: var(--bg);
  color: var(--ink);
  font-size: 14px;
  line-height: 1.6;
  user-select: none;
}
input,
textarea {
  user-select: text;
}
.app {
  display: flex;
  flex-direction: column;
  height: 100vh;
}

/* ---------- header ---------- */
.titlebar {
  display: flex;
  align-items: center;
  gap: 24px;
  padding: 14px 24px;
  background: var(--card);
  border-bottom: 1px solid var(--line);
  flex: none;
}
.brand {
  display: flex;
  align-items: center;
  gap: 10px;
}
.logo {
  width: 26px;
  height: 26px;
  color: var(--ac);
}
.brand-name {
  font-weight: 700;
  font-size: 15px;
  letter-spacing: -0.01em;
}
.tabs {
  display: flex;
  gap: 4px;
  flex: 1;
}
.tab {
  position: relative;
  border: none;
  background: transparent;
  font: inherit;
  font-weight: 600;
  color: var(--muted);
  padding: 8px 16px;
  border-radius: 8px;
  cursor: pointer;
  display: inline-flex;
  align-items: center;
  gap: 7px;
  transition: background 0.15s, color 0.15s;
}
.tab:hover {
  background: var(--ac-soft);
  color: var(--ac);
}
.tab.active {
  background: var(--ac-soft);
  color: var(--ac);
}
.badge-count {
  font-size: 11px;
  font-weight: 700;
  background: var(--ac);
  color: #fff;
  border-radius: 999px;
  min-width: 18px;
  height: 18px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  padding: 0 5px;
}
.dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: var(--ac);
  animation: pulse 1.2s ease-in-out infinite;
}
@keyframes pulse {
  50% {
    opacity: 0.3;
  }
}
.lang-switch {
  display: flex;
  gap: 2px;
  background: var(--bg);
  border: 1px solid var(--line);
  border-radius: 8px;
  padding: 2px;
}
.lang-btn {
  border: none;
  background: transparent;
  font: inherit;
  font-size: 12px;
  font-weight: 600;
  color: var(--muted);
  padding: 4px 10px;
  border-radius: 6px;
  cursor: pointer;
}
.lang-btn.active {
  background: var(--card);
  color: var(--ac);
  box-shadow: 0 1px 2px rgba(0, 0, 0, 0.08);
}

/* ---------- content / card ---------- */
.content {
  flex: 1;
  overflow-y: auto;
  padding: 24px;
}
.card {
  background: var(--card);
  border: 1px solid var(--line);
  border-radius: var(--radius);
  padding: 24px;
  max-width: 820px;
  margin: 0 auto;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.04);
}
.card h2 {
  font-size: 16px;
  font-weight: 700;
  margin-bottom: 16px;
}

/* ---------- inputs ---------- */
.url-row {
  display: flex;
  gap: 10px;
}
.url-input,
.text-input,
.dir-input,
.textarea {
  flex: 1;
  width: 100%;
  border: 1.5px solid var(--line);
  border-radius: 10px;
  padding: 11px 14px;
  font: inherit;
  background: var(--bg);
  color: var(--ink);
  outline: none;
  transition: border-color 0.15s, box-shadow 0.15s;
}
.url-input:focus,
.text-input:focus,
.dir-input:focus,
.textarea:focus {
  border-color: var(--ac);
  box-shadow: 0 0 0 3px var(--ac-soft);
  background: var(--card);
}
.textarea {
  resize: vertical;
  font-family: Consolas, monospace;
  font-size: 12.5px;
}
.btn {
  border: none;
  font: inherit;
  font-weight: 600;
  border-radius: 10px;
  padding: 11px 20px;
  cursor: pointer;
  transition: filter 0.15s, transform 0.1s, background 0.15s;
  flex: none;
}
.btn:active {
  transform: scale(0.98);
}
.btn.primary {
  background: var(--ac);
  color: #fff;
}
.btn.primary:hover {
  filter: brightness(1.1);
}
.btn.ghost {
  background: var(--bg);
  color: var(--ink);
  border: 1.5px solid var(--line);
}
.btn.ghost:hover {
  border-color: var(--ac);
  color: var(--ac);
}
.btn.small {
  padding: 6px 12px;
  font-size: 12.5px;
  border-radius: 8px;
}
.btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.preview {
  display: flex;
  gap: 14px;
  margin-top: 16px;
  padding: 12px;
  background: var(--bg);
  border-radius: 10px;
}
.thumb {
  width: 160px;
  height: 90px;
  object-fit: cover;
  border-radius: 8px;
  flex: none;
  background: var(--line);
}
.preview-title {
  font-weight: 700;
  font-size: 14.5px;
  line-height: 1.4;
}
.preview-sub {
  color: var(--muted);
  font-size: 12.5px;
  margin-top: 4px;
  display: flex;
  gap: 6px;
  flex-wrap: wrap;
}
.info-error {
  margin-top: 14px;
  padding: 10px 14px;
  background: #fef2f2;
  border: 1px solid #fecaca;
  border-radius: 10px;
  color: var(--err);
  font-size: 13px;
}

.option-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 18px;
  margin-top: 20px;
}
.field-label {
  display: block;
  font-size: 12.5px;
  font-weight: 600;
  color: var(--muted);
  margin-bottom: 7px;
}
.seg {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
  background: var(--bg);
  border: 1px solid var(--line);
  border-radius: 10px;
  padding: 3px;
}
.seg-btn {
  border: none;
  background: transparent;
  font: inherit;
  font-size: 12.5px;
  font-weight: 600;
  color: var(--muted);
  padding: 7px 12px;
  border-radius: 7px;
  cursor: pointer;
  transition: all 0.15s;
}
.seg-btn:hover {
  color: var(--ink);
}
.seg-btn.active {
  background: var(--card);
  color: var(--ac);
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
}
.dir-row {
  display: flex;
  gap: 8px;
}
.dir-input {
  flex: 1;
}

.adv-toggle {
  margin-top: 18px;
  border: none;
  background: transparent;
  font: inherit;
  font-size: 13px;
  font-weight: 600;
  color: var(--muted);
  cursor: pointer;
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 4px 0;
}
.adv-toggle:hover {
  color: var(--ac);
}
.chev {
  transition: transform 0.2s;
  display: inline-block;
}
.chev.open {
  transform: rotate(90deg);
}
.advanced {
  margin-top: 12px;
  display: flex;
  flex-direction: column;
  gap: 14px;
  padding: 16px;
  background: var(--bg);
  border-radius: 10px;
  border: 1px dashed var(--line);
}
.submit-row {
  margin-top: 22px;
  display: flex;
  justify-content: flex-end;
}

/* ---------- jobs ---------- */
.jobs-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 14px;
}
.jobs-head h2 {
  margin-bottom: 0;
}
.empty {
  text-align: center;
  color: var(--muted);
  padding: 48px 0;
}
.job-list {
  display: flex;
  flex-direction: column;
  gap: 10px;
}
.job {
  display: flex;
  gap: 16px;
  padding: 14px 16px;
  border: 1px solid var(--line);
  border-radius: 10px;
  background: var(--bg);
}
.job-main {
  flex: 1;
  min-width: 0;
}
.job-title {
  font-weight: 600;
  font-size: 13.5px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.job-path {
  font-size: 12px;
  color: var(--muted);
  font-family: Consolas, monospace;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  margin-top: 2px;
}
.job-error {
  font-size: 12px;
  color: var(--err);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  margin-top: 2px;
}
.job-progress-line {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-top: 8px;
}
.bar {
  flex: 1;
  height: 6px;
  background: var(--line);
  border-radius: 999px;
  overflow: hidden;
}
.bar-fill {
  height: 100%;
  background: var(--ac);
  border-radius: 999px;
  transition: width 0.3s ease;
}
.pct {
  font-size: 12px;
  font-weight: 700;
  color: var(--ac);
  min-width: 38px;
  text-align: right;
}
.job-stats {
  display: flex;
  gap: 16px;
  margin-top: 6px;
  font-size: 11.5px;
  color: var(--muted);
  font-family: Consolas, monospace;
}
.job-side {
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  gap: 8px;
  flex: none;
}
.status-chip {
  font-size: 11px;
  font-weight: 700;
  padding: 3px 10px;
  border-radius: 999px;
  background: var(--line);
  color: var(--muted);
}
.status-chip[data-status="downloading"] {
  background: var(--ac-soft);
  color: var(--ac);
}
.status-chip[data-status="finished"] {
  background: #ecfdf5;
  color: var(--ok);
}
.status-chip[data-status="error"] {
  background: #fef2f2;
  color: var(--err);
}
.status-chip[data-status="resolving"] {
  animation: pulse 1.2s ease-in-out infinite;
}
.job-enter-active,
.job-leave-active {
  transition: all 0.25s ease;
}
.job-enter-from,
.job-leave-to {
  opacity: 0;
  transform: translateY(-6px);
}

/* ---------- settings ---------- */
.setting-row {
  padding: 14px 0;
  border-bottom: 1px solid var(--line);
}
.setting-row:last-child {
  border-bottom: none;
}
.version-line {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-top: 6px;
}
.version-code {
  font-family: Consolas, monospace;
  font-size: 13px;
  background: var(--bg);
  border: 1px solid var(--line);
  padding: 4px 10px;
  border-radius: 6px;
}
.update-status {
  margin-top: 10px;
  display: flex;
  align-items: center;
  gap: 12px;
  font-size: 13px;
}
.update-avail {
  color: #d97706;
  font-weight: 600;
}
.update-msg {
  margin-top: 8px;
  font-size: 12.5px;
  color: var(--ok);
}
.update-msg.err {
  color: var(--err);
}
</style>
