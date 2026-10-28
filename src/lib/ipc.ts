import { invoke } from "@tauri-apps/api/core";

export interface ModRecord {
  name: string;
  enabled: boolean;
  files: string[];
  installed_files: string[];
  created_dirs: string[];
  description: string;
  version: string;
  image: string | null;
  source: string | null;
  nexus_id: number | null;
}

export interface ExternalMod {
  name: string;
  kind: "archive" | "cet" | "redscript" | "red4ext" | "tweaks";
  files: string[];
}

export interface NcRelease {
  version: string;
  channel: string;
  date: string;
  game: string[];
  protocol: number;
  build: string;
  title: string;
  notes: string[];
  size: number;
  sha256: string;
  file_name: string;
  yanked: string | null;
}

export interface NcData {
  updated: string;
  latest: string;
  releases: NcRelease[];
  offline: boolean;
}

export interface LogFile {
  id: string;
  source: string;
  group: string;
  name: string;
  size: number;
  modified: number;
  crash: boolean;
}

export interface LogChunk {
  text: string;
  start: number;
  end: number;
  size: number;
}

export interface CrashSummary {
  id: string;
  name: string;
  source: string;
  modified: number;
  excerpt: string;
}

export interface Detected {
  version: string | null;
  build: string | null;
  source: string;
}

export interface NexusMod {
  mod_id: number;
  name: string | null;
  summary: string | null;
  version: string;
  picture_url: string | null;
  updated_timestamp: number;
  status: string;
  author: string;
  category_id: number;
  contains_adult_content: boolean;
  endorsement_count: number;
  mod_downloads: number;
  tags: string[];
}

export interface NexusCategory {
  category_id: number;
  name: string;
  parent: number | null;
}

export interface NexusBrowse {
  items: NexusMod[];
  categories: NexusCategory[];
  fetched_at: number;
  installed: number[];
}

export interface NexusSearchParams {
  query: string;
  categories: string[];
  hide_adult: boolean;
  sort: string;
  tag: string | null;
  offset: number;
  count: number;
}

export interface NexusSearchPage {
  items: NexusMod[];
  total: number;
}

export interface NxmLink {
  mod_id: number;
  file_id: number;
  key: string;
  expires: number;
}

export type NxmHandler = "ours" | "other" | "none";

export interface FoundDownload {
  path: string;
  name: string;
  size: number;
  modified: number;
}

export interface FomodPlugin {
  name: string;
  description: string;
}

export interface FomodGroup {
  name: string;
  kind: string;
  order: string;
  plugins: FomodPlugin[];
}

export interface FomodStep {
  name: string;
  order: string;
  groups: FomodGroup[];
}

export interface FomodView {
  name: string;
  order: string;
  steps: FomodStep[];
}

export interface FomodPending {
  name: string;
  view: FomodView;
}

export interface FomodEvaluation {
  visible: boolean[];
  types: string[][][];
}

export type FomodSelection = number[][][];

export interface FrameworkState {
  id: string;
  present: boolean;
  version: string | null;
}

export interface FrameworkReport {
  frameworks: FrameworkState[];
  old_loader: boolean;
}

export interface NexusFileInfo {
  file_id: number;
  name: string;
  version: string;
  category_name: string;
  is_primary: boolean;
  size_kb: number;
  file_name: string;
  uploaded_timestamp: number;
}

export interface NexusRequirement {
  mod_id: number;
  name: string;
  url: string;
  external: boolean;
  notes: string;
}

export interface ProfilePlanItem {
  source: string;
  mod_id: number;
  version: string;
  name: string;
  installed: boolean;
}

export interface ProfileBuilt {
  code: string;
  json: string;
  count: number;
  skipped: string[];
}

export interface ProfileView {
  name: string;
  items: ProfilePlanItem[];
  encoded: string;
}

export interface Preset {
  name: string;
  mods: string[];
}

export interface ApplyReport {
  enabled: string[];
  disabled: string[];
  failed: [string, string][];
  missing: string[];
}

export interface SharedEntry {
  code: string;
  expires: number;
  name: string;
  count: number;
}

export interface NexusUser {
  name: string;
  is_premium: boolean;
  is_supporter: boolean;
}

export interface NexusStatus {
  connected: boolean;
  user: NexusUser | null;
}

export interface NexusItem {
  name: string;
  mod_id: number;
  installed: string;
  latest: string | null;
  update: boolean;
  checked_at: number | null;
  page_url: string;
  error: string | null;
}

export interface AppInfo {
  version: string;
  data_dir: string;
}

export interface Conflict {
  file: string;
  owner: string | null;
}

export interface AppError {
  code: string;
  message: string;
  details: Conflict[] | null;
}

export function toAppError(e: unknown): AppError {
  if (e && typeof e === "object" && "code" in e && "message" in e) {
    const o = e as { code: string; message: string; details?: Conflict[] | null };
    return { code: o.code, message: o.message, details: o.details ?? null };
  }
  return { code: "unknown", message: String(e), details: null };
}

export const ipc = {
  detectGames: () => invoke<string[]>("detect_games"),
  getGameDir: () => invoke<string | null>("get_game_dir"),
  setGameDir: (path: string) => invoke<void>("set_game_dir", { path }),
  gameRunning: () => invoke<boolean>("game_running"),
  listMods: () => invoke<ModRecord[]>("list_mods"),
  scanExternal: () => invoke<ExternalMod[]>("scan_external"),
  importMod: (name: string) => invoke<ModRecord>("import_mod", { name }),
  installMod: (zipPath: string, name: string) =>
    invoke<{ added: ModRecord[]; skipped: string[]; ignored?: string[] }>("install_mod", { zipPath, name }),
  setModInfo: (name: string, description: string, version: string, imagePath: string | null) =>
    invoke<ModRecord>("set_mod_info", { name, description, version, imagePath }),
  modImage: (name: string) => invoke<string | null>("mod_image", { name }),
  exportMod: (name: string, dest: string) => invoke<void>("export_mod", { name, dest }),
  exportPack: (names: string[], dest: string) => invoke<void>("export_pack", { names, dest }),
  appInfo: () => invoke<AppInfo>("app_info"),
  nexusStatus: () => invoke<NexusStatus>("nexus_status"),
  nexusConnect: (key: string) => invoke<NexusStatus>("nexus_connect", { key }),
  nexusDisconnect: () => invoke<void>("nexus_disconnect"),
  nexusCheck: () => invoke<NexusItem[]>("nexus_check"),
  nexusLink: (name: string, target: string) => invoke<ModRecord>("nexus_link", { name, target }),
  nexusUnlink: (name: string) => invoke<ModRecord>("nexus_unlink", { name }),
  openUrl: (url: string) => invoke<void>("open_url", { url }),
  nexusBrowse: (kind: string) => invoke<NexusBrowse>("nexus_browse", { kind }),
  nexusSearch: (params: NexusSearchParams) => invoke<NexusSearchPage>("nexus_search", { params }),
  nexusInstallNxm: (link: NxmLink) => invoke<string[]>("nexus_install_nxm", { link }),
  nexusModName: (modId: number) => invoke<string>("nexus_mod_name", { modId }),
  takePendingNxm: () => invoke<NxmLink | null>("take_pending_nxm"),
  nxmStatus: () => invoke<NxmHandler>("nxm_status"),
  nxmRegister: (enable: boolean) => invoke<NxmHandler>("nxm_register", { enable }),
  downloadsRecent: (since: number) => invoke<FoundDownload[]>("downloads_recent", { since }),
  installDownload: (path: string) => invoke<ModRecord[]>("install_download", { path }),
  translateNotes: (lines: string[], lang: string) => invoke<string[]>("translate_notes", { lines, lang }),
  nexusRequirements: (modId: number) => invoke<NexusRequirement[]>("nexus_requirements", { modId }),
  profileBuild: (names: string[], title: string) => invoke<ProfileBuilt>("profile_build", { names, title }),
  profileOpen: (text: string) => invoke<ProfileView>("profile_open", { text }),
  profileShare: (encoded: string) => invoke<{ code: string; expires: number }>("profile_share", { encoded }),
  profileSave: (path: string, text: string) => invoke<void>("profile_save", { path, text }),
  profileRead: (path: string) => invoke<string>("profile_read", { path }),
  presetsList: () => invoke<Preset[]>("presets_list"),
  presetSave: (name: string, mods: string[]) => invoke<Preset[]>("preset_save", { name, mods }),
  presetRename: (old: string, name: string) => invoke<Preset[]>("preset_rename", { old, new: name }),
  presetDelete: (name: string) => invoke<Preset[]>("preset_delete", { name }),
  presetApply: (name: string) => invoke<ApplyReport>("preset_apply", { name }),
  fomodPending: () => invoke<FomodPending>("fomod_pending"),
  fomodEval: (selection: FomodSelection) => invoke<FomodEvaluation>("fomod_eval", { selection }),
  fomodCancel: () => invoke<void>("fomod_cancel"),
  fomodInstall: (selection: FomodSelection) =>
    invoke<{ added: ModRecord[]; skipped: string[]; ignored?: string[] }>("fomod_install", { selection }),
  frameworkPlan: (id: string) => invoke<string[]>("framework_plan", { id }),
  frameworkRemove: (id: string) => invoke<number>("framework_remove", { id }),
  frameworkStatus: () => invoke<FrameworkReport>("framework_status"),
  nexusFiles: (modId: number) => invoke<NexusFileInfo[]>("nexus_files", { modId }),
  nexusInstallFile: (modId: number, fileId: number) => invoke<string[]>("nexus_install_file", { modId, fileId }),
  nexusFillCovers: () => invoke<number>("nexus_fill_covers"),
  sharesList: () => invoke<SharedEntry[]>("shares_list"),
  getInstallId: () => invoke<string>("get_install_id"),
  resetInstallId: () => invoke<string>("reset_install_id"),
  nexusInstall: (modId: number) => invoke<string[]>("nexus_install", { modId }),
  nexusProbe: () => invoke<number>("nexus_probe"),
  ncFetch: (lang: string) => invoke<NcData>("nc_fetch", { lang }),
  ncInstall: (version: string, lang: string, force: boolean) =>
    invoke<void>("nc_install", { version, lang, force }),
  enableMod: (name: string, force: boolean) => invoke<void>("enable_mod", { name, force }),
  disableMod: (name: string) => invoke<void>("disable_mod", { name }),
  uninstallMod: (name: string) => invoke<void>("uninstall_mod", { name }),
  buildErrorReport: (error: string, note: string, language: string, logId: string | null = null) =>
    invoke<string>("build_error_report", { error, note, language, logId }),
  logsScan: () => invoke<LogFile[]>("logs_scan"),
  logsRead: (id: string, end?: number, max?: number) =>
    invoke<LogChunk>("logs_read", { id, end: end ?? null, max: max ?? null }),
  logsHead: (id: string) => invoke<LogChunk>("logs_head", { id }),
  logsPoll: (id: string, from: number) => invoke<LogChunk>("logs_poll", { id, from }),
  logsSummary: () => invoke<CrashSummary | null>("logs_summary"),
  getPrefs: () =>
    invoke<{
      locale: string | null;
      theme: string | null;
      motion: string | null;
      translate: string | null;
      updates: string | null;
      translate_lang: string | null;
      mod_updates: string | null;
    }>("get_prefs"),
  setPrefs: (locale: string, theme: string, motion: string, translate: string, updates: string, translateLang: string, modUpdates: string) =>
    invoke<void>("set_prefs", { locale, theme, motion, translate, updates, translateLang, modUpdates }),
  logsOpen: (id: string) => invoke<void>("logs_open", { id }),
  logsReveal: (id: string) => invoke<void>("logs_reveal", { id }),
  logsClear: () => invoke<{ removed: number; freed: number }>("logs_clear"),
  ncDetect: (builds: { version: string; build: string }[]) => invoke<Detected | null>("nc_detect", { builds }),
  saveErrorReport: (text: string) => invoke<string>("save_error_report", { text }),
  openFolder: (path: string) => invoke<void>("open_folder", { path }),
};
