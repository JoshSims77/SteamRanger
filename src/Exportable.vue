<template>
  <div class="exportable">

    <!-- LEFT: GROUP LIST -->
    <div class="group-list">
      <h2 class="heading">Groups</h2>
      <p class="subheading">Select a group to configure and export.</p>

      <!-- Regular groups -->
      <div
        v-for="group in exportGroups"
        :key="group.id"
        class="group-row"
        :class="{ selected: selectedGroupId === group.id }"
        @click="selectGroup(group.id)"
      >
        <div class="group-row-inner">
          <span class="group-name">{{ group.name }}</span>
          <span class="group-count">{{ group.achievements.length }}</span>
        </div>
        <div class="icon-strip">
          <img
            v-for="ach in group.achievements.slice(0, 8)"
            :key="ach.id"
            :src="ach.icon"
            :title="ach.name"
            class="strip-icon"
          />
          <span v-if="group.achievements.length > 8" class="strip-overflow">
            +{{ group.achievements.length - 8 }}
          </span>
        </div>
      </div>

      <!-- Footer group — always last -->
      <div
        class="group-row footer-row"
        :class="{ selected: selectedGroupId === 'footer' }"
        @click="selectGroup('footer')"
      >
        <div class="group-row-inner">
          <span class="group-name">Acknowledgements</span>
          <span class="group-count">Footer</span>
        </div>
      </div>

      <div class="back-row">
        <button class="btn btn-back" @click="goBack">← Back to Groups</button>
        <button class="btn btn-export" @click="handleExport" :disabled="exporting">
          {{ exporting ? "Exporting..." : "Export Guide" }}
        </button>
      </div>

      <!-- CONFIRM OVERWRITE DIALOG -->
      <div v-if="showConfirm" class="dialog-overlay">
        <div class="dialog">
          <p class="dialog-message">
            There is already a <strong>{{ gameName }}</strong> folder in
            <code>/output_for_users</code>. Should we replace it?
          </p>
          <div class="dialog-actions">
            <button class="btn btn-danger" @click="confirmOverwrite">Replace</button>
            <button class="btn btn-back" @click="showConfirm = false">Cancel</button>
          </div>
        </div>
      </div>

      <!-- EXPORT STATUS -->
      <div v-if="exportStatus" class="export-status" :class="exportStatus.type">
        {{ exportStatus.message }}
      </div>
    </div>

    <!-- RIGHT: REGULAR GROUP OPTIONS -->
    <div class="options-panel" v-if="selectedGroup && selectedGroupId !== 'footer'">

      <div class="options-header">
        <h3 class="options-title">{{ selectedGroup.name }}</h3>
        <span class="options-count">{{ selectedGroup.achievements.length }} achievements</span>
      </div>

      <div class="format-picker">
        <label class="format-option" :class="{ active: formatType === 'connected' }">
          <input type="radio" v-model="formatType" value="connected" />
          <div class="format-label">
            <span class="format-name">Connected List</span>
            <span class="format-desc">Sequential achievements in a single flowing section. Common for unmissables.</span>
          </div>
        </label>

        <label class="format-option" :class="{ active: formatType === 'separated' }">
          <input type="radio" v-model="formatType" value="separated" />
          <div class="format-label">
            <span class="format-name">Separated List</span>
            <span class="format-desc">Achievements in the same category, each visually distinct.</span>
          </div>
        </label>
      </div>

      <div class="sub-options">
        <label class="checkbox-option">
          <input type="checkbox" v-model="addUserDescriptions" />
          <span>Add User Descriptions</span>
        </label>
        <p class="checkbox-hint">
          Inserts <code>WRITE_ACHIEVEMENT_NOTES_HERE</code> after each achievement description for your own notes.
        </p>
      </div>

      <div class="output-section">
        <div class="output-header">
          <span class="output-label">BBCode Output</span>
          <button class="btn btn-copy" @click="copyToClipboard">
            {{ copied ? "Copied!" : "Copy" }}
          </button>
        </div>
        <textarea class="bbcode-box" readonly :value="bbcode" />
      </div>

    </div>

    <!-- RIGHT: FOOTER / ACKNOWLEDGEMENTS -->
    <div class="options-panel" v-else-if="selectedGroupId === 'footer'">

      <div class="options-header">
        <h3 class="options-title">Acknowledgements</h3>
        <span class="options-count">Footer section</span>
      </div>

      <p class="footer-hint">
        This section generates an acknowledgements footer for your guide.
        Add as many contributors as you like — each entry is a placeholder
        for you to fill in with a real Steam profile image ID and username.
      </p>

      <!-- CONTRIBUTOR COUNT -->
      <div class="contributor-controls">
        <span class="contributor-label">Contributors</span>
        <div class="contributor-stepper">
          <button class="stepper-btn" @click="removeContributor" :disabled="contributors.length === 0">−</button>
          <span class="stepper-count">{{ contributors.length }}</span>
          <button class="stepper-btn" @click="addContributor">+</button>
        </div>
      </div>

      <!-- CONTRIBUTOR PREVIEW -->
      <div v-if="contributors.length > 0" class="contributor-list">
        <div v-for="(c, i) in contributors" :key="i" class="contributor-row">
          <span class="contributor-index">{{ i + 1 }}</span>
          <span class="contributor-placeholder">STEAM_PROFILE_IMG — USERNAME - Achievement Name</span>
        </div>
      </div>

      <div class="output-section">
        <div class="output-header">
          <span class="output-label">BBCode Output</span>
          <button class="btn btn-copy" @click="copyFooterToClipboard">
            {{ copiedFooter ? "Copied!" : "Copy" }}
          </button>
        </div>
        <textarea class="bbcode-box" readonly :value="footerBbcode" />
      </div>

    </div>

    <!-- RIGHT: EMPTY STATE -->
    <div class="options-panel empty-state" v-else>
      <span class="empty-text">← Select a group to begin</span>
    </div>

  </div>
</template>

<script setup lang="ts">
import { ref, computed } from "vue";
import { exportGroups, showExport, gameName } from "./lib/GuideState";
import type { AchievementGroup } from "./lib/achievement";

import {
  mkdir,
  exists,
  remove,
  writeTextFile,
  copyFile,
  readDir
} from "@tauri-apps/plugin-fs";
import { openPath } from "@tauri-apps/plugin-opener";
import { steamApi } from "./lib/steamApi";
import { join, homeDir } from "@tauri-apps/api/path";

type FormatType = "connected" | "separated";

interface GroupOptions {
  formatType: FormatType;
  addUserDescriptions: boolean;
}

// ─── Group selection ──────────────────────────────────────────────────────────

const selectedGroupId = ref<string | null>(null);
const groupOptions = ref<Record<string, GroupOptions>>({});

function getOptions(id: string): GroupOptions {
  if (!groupOptions.value[id]) {
    groupOptions.value[id] = { formatType: "connected", addUserDescriptions: false };
  }
  return groupOptions.value[id];
}

const formatType = computed<FormatType>({
  get: () => selectedGroupId.value ? getOptions(selectedGroupId.value).formatType : "connected",
  set: (val) => { if (selectedGroupId.value) getOptions(selectedGroupId.value).formatType = val; }
});

const addUserDescriptions = computed<boolean>({
  get: () => selectedGroupId.value ? getOptions(selectedGroupId.value).addUserDescriptions : false,
  set: (val) => { if (selectedGroupId.value) getOptions(selectedGroupId.value).addUserDescriptions = val; }
});

const copied = ref(false);
const copiedFooter = ref(false);
const exporting = ref(false);
const showConfirm = ref(false);
const exportStatus = ref<{ type: "success" | "error"; message: string } | null>(null);

const selectedGroup = computed<AchievementGroup | null>(() =>
  exportGroups.value.find(g => g.id === selectedGroupId.value) ?? null
);

function selectGroup(id: string) {
  selectedGroupId.value = id;
  if (id !== "footer") getOptions(id);
}

// ─── Contributors ─────────────────────────────────────────────────────────────

const contributors = ref<null[]>([]);

function addContributor() {
  contributors.value.push(null);
}

function removeContributor() {
  if (contributors.value.length > 0) contributors.value.pop();
}

// ─── Footer BBCode ────────────────────────────────────────────────────────────

const DIVIDER = "-------------------------------------------------------------------------------------------------------";
const NL = "[h1][/h1]";
const STEAMRANGER_CREDIT = "This guide was made for free using SteamRanger, available here: PLACEHOLDER_LINK";

const footerBbcode = computed(() => {
  const lines: string[] = [];

  lines.push("I would like to thank the following people for their help with this guide:");
  lines.push(NL);
  lines.push(DIVIDER);

  if (contributors.value.length > 0) {
    contributors.value.forEach(() => {
      lines.push(NL);
      lines.push("__IMG__[b]USERNAME[/b]");
      lines.push("[i]Achievement Name[/i]");
      lines.push(NL);
      lines.push(DIVIDER);
    });
  }

  lines.push(NL);
  lines.push(NL);
  lines.push(DIVIDER);
  lines.push(NL);
  lines.push(STEAMRANGER_CREDIT);

  return lines.join("\n");
});

// ─── BBCode Generators ────────────────────────────────────────────────────────

function buildConnectedList(group: AchievementGroup, withNotes: boolean): string {
  const nl = "[h1][/h1]";
  const intro = "WRITE_SECTION_INTRO_HERE";

  const entries = group.achievements.map((a) => {
    const lines = [
      nl,
      `__IMG__[b]${a.name}[/b]`,
      `[i]${a.description}[/i]`,
    ];
    if (withNotes) lines.push("WRITE_ACHIEVEMENT_NOTES_HERE");
    return lines.join("\n");
  });

  return [intro, nl, nl, ...entries].join("\n");
}

function buildSeparatedList(group: AchievementGroup, withNotes: boolean): string {
  const nl = "[h1][/h1]";
  const divider = "-------------------------------------------------------------------------------------------------------";
  const intro = "WRITE_SECTION_INTRO_HERE";

  const entries = group.achievements.map((a) => {
    const lines = [
      nl,
      `__IMG__[b]${a.name}[/b]`,
      `[i]${a.description}[/i]`,
    ];
    if (withNotes) lines.push("WRITE_ACHIEVEMENT_NOTES_HERE");
    return lines.join("\n");
  });

  const body = entries.reduce((acc, entry, i) => {
    acc.push(entry);
    if (i < entries.length - 1) acc.push(divider);
    return acc;
  }, [] as string[]);

  return [intro, nl, nl, ...body].join("\n");
}

const bbcode = computed(() => {
  if (!selectedGroup.value) return "";
  if (formatType.value === "connected") {
    return buildConnectedList(selectedGroup.value, addUserDescriptions.value);
  }
  return buildSeparatedList(selectedGroup.value, addUserDescriptions.value);
});

// ─── Export Logic ─────────────────────────────────────────────────────────────

function toFolderName(name: string): string {
  return name.replace(/[<>:"/\\|?*]/g, "").trim();
}

async function downloadIcon(url: string, destPath: string): Promise<void> {
  await steamApi.downloadIcon(url, destPath);
}

async function buildExport(): Promise<void> {
  const home = await homeDir();
  const outputRoot = await join(home, "SteamRanger", "output_for_users");
  const gameFolder = await join(outputRoot, toFolderName(gameName.value));

  await mkdir(outputRoot, { recursive: true });

  const imagesFolder = await join(gameFolder, "images");
  await mkdir(imagesFolder, { recursive: true });

  for (const group of exportGroups.value) {
    const bbcodeFolder = await join(gameFolder, `${toFolderName(group.name)}_bbcodes`);
    await mkdir(bbcodeFolder, { recursive: true });

    // Download all achievement icons into the flat images folder
    for (const ach of group.achievements) {
      const ext = ach.icon.split(".").pop()?.split("?")[0] ?? "jpg";
      const filename = `${toFolderName(ach.name)}.${ext}`;
      const iconDest = await join(imagesFolder, filename);
      try {
        await downloadIcon(ach.icon, iconDest);
      } catch {
        console.warn(`Could not download icon for ${ach.name}`);
      }
    }

    const opts = getOptions(group.id);
    const code = opts.formatType === "connected"
      ? buildConnectedList(group, opts.addUserDescriptions)
      : buildSeparatedList(group, opts.addUserDescriptions);

    const bbcodePath = await join(bbcodeFolder, `${toFolderName(group.name)}.txt`);
    await writeTextFile(bbcodePath, code);
  }

  // Write footer bbcode
  const footerFolder = await join(gameFolder, "acknowledgements_bbcodes");
  await mkdir(footerFolder, { recursive: true });
  await writeTextFile(await join(footerFolder, "acknowledgements.txt"), footerBbcode.value);

  const additionalImages = await join(gameFolder, "additional_images");
  await mkdir(additionalImages, { recursive: true });

  const lettersSource = await join(home, "SteamRanger", "src", "assets", "letters");
  try {
    const entries = await readDir(lettersSource);
    for (const entry of entries) {
      if (entry.name) {
        const src = await join(lettersSource, entry.name);
        const dest = await join(imagesFolder, entry.name);
        await copyFile(src, dest);
      }
    }
  } catch (e: any) {
    console.warn("Letters path attempted:", lettersSource);
    console.warn("Letters error (raw):", e);
    console.warn("Letters error (json):", JSON.stringify(e));
  }
}

async function handleExport(): Promise<void> {
  exportStatus.value = null;

  const home = await homeDir();
  const outputRoot = await join(home, "SteamRanger", "output_for_users");
  const gameFolder = await join(outputRoot, toFolderName(gameName.value));

  const alreadyExists = await exists(gameFolder);
  if (alreadyExists) {
    showConfirm.value = true;
    return;
  }

  await runExport(gameFolder);
}

async function confirmOverwrite(): Promise<void> {
  showConfirm.value = false;

  const home = await homeDir();
  const outputRoot = await join(home, "SteamRanger", "output_for_users");
  const gameFolder = await join(outputRoot, toFolderName(gameName.value));

  try {
    await remove(gameFolder, { recursive: true });
  } catch {
    exportStatus.value = { type: "error", message: "Failed to remove existing folder." };
    return;
  }

  await runExport(gameFolder);
}

async function runExport(gameFolder: string): Promise<void> {
  exporting.value = true;
  exportStatus.value = null;

  try {
    await buildExport();
    exportStatus.value = {
      type: "success",
      message: `Exported to /SteamRanger/output_for_users/${toFolderName(gameName.value)}`
    };
  } catch (e: any) {
    exportStatus.value = {
      type: "error",
      message: `Export failed: ${e?.message ?? "Unknown error"}`
    };
  } finally {
    exporting.value = false;
  }

  // Open folder separately — don't let this failure affect the export status
  try {
    await openPath(gameFolder);
  } catch (e: any) {
    console.warn("Could not open folder in explorer:", e);
  }
}

// ─── Copy ─────────────────────────────────────────────────────────────────────

async function copyToClipboard() {
  try {
    await navigator.clipboard.writeText(bbcode.value);
    copied.value = true;
    setTimeout(() => (copied.value = false), 2000);
  } catch {
    const el = document.querySelector(".bbcode-box") as HTMLTextAreaElement;
    el?.select();
  }
}

async function copyFooterToClipboard() {
  try {
    await navigator.clipboard.writeText(footerBbcode.value);
    copiedFooter.value = true;
    setTimeout(() => (copiedFooter.value = false), 2000);
  } catch {
    const el = document.querySelector(".bbcode-box") as HTMLTextAreaElement;
    el?.select();
  }
}

function goBack() {
  showExport.value = false;
}
</script>

<style scoped>
.exportable {
  display: flex;
  height: 100%;
  gap: 0;
}

/* ── LEFT COLUMN ──────────────────────────────────────────────────── */
.group-list {
  width: 220px;
  min-width: 220px;
  border-right: 1px solid #e5e7eb;
  display: flex;
  flex-direction: column;
  gap: 6px;
  padding: 12px;
  overflow-y: auto;
}

.heading {
  font-size: 14px;
  font-weight: 600;
  margin: 0;
}

.subheading {
  font-size: 11px;
  color: #6b7280;
  margin: 0 0 4px;
}

.group-row {
  border: 1px solid #d1d5db;
  border-radius: 6px;
  padding: 8px;
  background: #fff;
  cursor: pointer;
  transition: border-color 0.15s, background 0.15s;
}

.group-row:hover {
  border-color: #9ca3af;
  background: #f9fafb;
}

.group-row.selected {
  border-color: #111827;
  background: #f3f4f6;
}

.footer-row {
  border-style: dashed;
  border-color: #9ca3af;
  margin-top: 4px;
}

.footer-row.selected {
  border-style: dashed;
  border-color: #111827;
}

.group-row-inner {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 4px;
}

.group-name {
  font-size: 12px;
  font-weight: 600;
  color: #111827;
}

.group-count {
  font-size: 11px;
  color: #6b7280;
  background: #e5e7eb;
  border-radius: 10px;
  padding: 1px 6px;
}

.icon-strip {
  display: flex;
  flex-wrap: wrap;
  gap: 3px;
  align-items: center;
}

.strip-icon {
  width: 20px;
  height: 20px;
  border-radius: 3px;
}

.strip-overflow {
  font-size: 10px;
  color: #6b7280;
}

.back-row {
  margin-top: auto;
  padding-top: 12px;
  display: flex;
  flex-direction: column;
  gap: 6px;
}

/* ── RIGHT PANEL ──────────────────────────────────────────────────── */
.options-panel {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 16px;
  padding: 16px;
  overflow-y: auto;
}

.empty-state {
  align-items: center;
  justify-content: center;
}

.empty-text {
  font-size: 13px;
  color: #9ca3af;
}

.options-header {
  display: flex;
  align-items: baseline;
  gap: 10px;
}

.options-title {
  font-size: 14px;
  font-weight: 600;
  margin: 0;
  color: #111827;
}

.options-count {
  font-size: 12px;
  color: #6b7280;
}

/* FORMAT PICKER */
.format-picker {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.format-option {
  display: flex;
  align-items: flex-start;
  gap: 10px;
  border: 1px solid #d1d5db;
  border-radius: 6px;
  padding: 10px 12px;
  cursor: pointer;
  background: #fff;
  transition: border-color 0.15s, background 0.15s;
}

.format-option input[type="radio"] {
  margin-top: 2px;
  flex-shrink: 0;
  cursor: pointer;
}

.format-option.active {
  border-color: #111827;
  background: #f3f4f6;
}

.format-label {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.format-name {
  font-size: 13px;
  font-weight: 600;
  color: #111827;
}

.format-desc {
  font-size: 11px;
  color: #6b7280;
}

/* SUB OPTIONS */
.sub-options {
  display: flex;
  flex-direction: column;
  gap: 4px;
  padding: 10px 12px;
  background: #f9fafb;
  border: 1px solid #e5e7eb;
  border-radius: 6px;
}

.checkbox-option {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 13px;
  font-weight: 500;
  color: #111827;
  cursor: pointer;
}

.checkbox-option input[type="checkbox"] {
  cursor: pointer;
}

.checkbox-hint {
  font-size: 11px;
  color: #6b7280;
  margin: 0;
}

.checkbox-hint code {
  font-family: monospace;
  background: #e5e7eb;
  padding: 1px 4px;
  border-radius: 3px;
}

/* FOOTER PANEL */
.footer-hint {
  font-size: 12px;
  color: #6b7280;
  margin: 0;
  line-height: 1.5;
}

.contributor-controls {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 12px;
  border: 1px solid #e5e7eb;
  border-radius: 6px;
  background: #f9fafb;
}

.contributor-label {
  font-size: 13px;
  font-weight: 600;
  color: #111827;
}

.contributor-stepper {
  display: flex;
  align-items: center;
  gap: 10px;
}

.stepper-btn {
  width: 28px;
  height: 28px;
  border: 1px solid #d1d5db;
  border-radius: 6px;
  background: #fff;
  font-size: 16px;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  color: #111827;
}

.stepper-btn:hover {
  background: #f3f4f6;
}

.stepper-btn:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.stepper-count {
  font-size: 14px;
  font-weight: 600;
  color: #111827;
  min-width: 20px;
  text-align: center;
}

.contributor-list {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.contributor-row {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 6px 10px;
  background: #f9fafb;
  border: 1px solid #e5e7eb;
  border-radius: 6px;
  font-size: 11px;
}

.contributor-index {
  font-weight: 700;
  color: #6b7280;
  min-width: 16px;
}

.contributor-placeholder {
  color: #9ca3af;
  font-style: italic;
}

/* BBCODE OUTPUT */
.output-section {
  display: flex;
  flex-direction: column;
  gap: 6px;
  flex: 1;
}

.output-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.output-label {
  font-size: 13px;
  font-weight: 600;
  color: #111827;
}

.bbcode-box {
  flex: 1;
  min-height: 200px;
  padding: 10px 12px;
  border: 1px solid #d1d5db;
  border-radius: 6px;
  font-size: 12px;
  font-family: monospace;
  color: #374151;
  background: #f9fafb;
  resize: vertical;
  box-sizing: border-box;
  width: 100%;
}

/* BUTTONS */
.btn {
  padding: 10px 12px;
  border: none;
  border-radius: 6px;
  background: #111827;
  color: white;
  font-size: 14px;
  cursor: pointer;
  white-space: nowrap;
}

.btn:hover {
  background: #1f2937;
}

.btn-copy {
  padding: 6px 12px;
  font-size: 12px;
}

.btn-back {
  width: 100%;
  background: #6b7280;
  font-size: 12px;
  padding: 8px 12px;
}

.btn-back:hover {
  background: #4b5563;
}

.btn-export {
  width: 100%;
  font-size: 12px;
  padding: 8px 12px;
}

.btn-export:disabled {
  background: #9ca3af;
  cursor: not-allowed;
}

.btn-danger {
  background: #dc2626;
}

.btn-danger:hover {
  background: #b91c1c;
}

/* DIALOG */
.dialog-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.4);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 100;
}

.dialog {
  background: #fff;
  border: 1px solid #d1d5db;
  border-radius: 8px;
  padding: 20px;
  max-width: 360px;
  width: 90%;
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.dialog-message {
  font-size: 13px;
  color: #111827;
  margin: 0;
  line-height: 1.5;
}

.dialog-message code {
  font-family: monospace;
  background: #f3f4f6;
  padding: 1px 4px;
  border-radius: 3px;
}

.dialog-actions {
  display: flex;
  gap: 8px;
  justify-content: flex-end;
}

/* EXPORT STATUS */
.export-status {
  font-size: 12px;
  padding: 8px 10px;
  border-radius: 6px;
  margin-top: 4px;
}

.export-status.success {
  background: #dcfce7;
  color: #15803d;
  border: 1px solid #bbf7d0;
}

.export-status.error {
  background: #fee2e2;
  color: #dc2626;
  border: 1px solid #fecaca;
}
</style>