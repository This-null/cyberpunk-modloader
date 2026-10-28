const art = (a: string, b: string, label: string) =>
  `data:image/svg+xml;utf8,${encodeURIComponent(
    `<svg xmlns="http://www.w3.org/2000/svg" width="640" height="360"><defs><linearGradient id="g" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="${a}"/><stop offset="1" stop-color="${b}"/></linearGradient></defs><rect width="640" height="360" fill="url(#g)"/><polygon points="0,360 150,160 260,360" fill="#101016"/><polygon points="200,360 380,90 500,360" fill="#16161e"/><polygon points="420,360 560,190 640,360" fill="#0c0c12"/><text x="24" y="44" font-family="sans-serif" font-size="26" fill="#ffffffcc">${label}</text></svg>`,
  )}`;

const categories = [
  { category_id: 1, name: "Visuals and Graphics", parent: null },
  { category_id: 2, name: "Lighting", parent: 1 },
  { category_id: 3, name: "Gameplay", parent: null },
  { category_id: 4, name: "Clothing and Accessories", parent: null },
  { category_id: 5, name: "Weapons", parent: null },
  { category_id: 6, name: "User Interface", parent: null },
  { category_id: 7, name: "Audio", parent: null },
];

const names: [string, string, number, boolean, string, string][] = [
  ["Neon Streets Reloaded", "Deeper neon colours and wet reflections across Night City.", 2, false, "#ff4696", "#3a1a6a"],
  ["Chrome HUD", "A cleaner HUD with a smaller minimap and brushed metal frames.", 6, false, "#f0be3c", "#1e2846"],
  ["Street Samurai Jacket Pack", "Twelve new jackets with full colour variants for V.", 4, false, "#3cb4f0", "#14283c"],
  ["Night City Weather Overhaul", "Heavier rain, fog banks and clearer sunsets.", 1, false, "#5ad2a0", "#10303a"],
  ["Smart Gun Rebalance", "Rebalances smart weapons so tech builds stay fun at every level.", 5, false, "#e0643c", "#2a1410"],
  ["Spicy Outfits Collection", "An adult outfit collection for the bold.", 4, true, "#d23c78", "#2a0f20"],
  ["Better Loot Tables", "Fewer junk drops and more meaningful rewards from fights.", 3, false, "#8c78f0", "#1c1640"],
  ["Radio Expanded", "Adds thirty new tracks to the in-game radio.", 7, false, "#f0a03c", "#2a1c0c"],
  ["Cyberware Sound Fix", "Fixes missing sounds for several cyberware abilities.", 7, false, "#64d2f0", "#102838"],
  ["Skyline Lights", "Brighter, warmer lights on distant towers.", 2, false, "#ffbe5a", "#2a1a36"],
  ["Compact Quest Log", "Shrinks and tidies the quest log so it fits on one screen.", 6, false, "#78e08c", "#10281c"],
  ["Mature Companion Skins", "Adult skins for companions.", 4, true, "#ff6482", "#2c0c18"],
  ["Faster Elevators", "Cuts elevator waits in half.", 3, false, "#a0a0f0", "#181830"],
  ["Hologram Billboards 4K", "Sharper textures for animated billboards.", 1, false, "#f064d2", "#280c28"],
];

const mods = names.map(([name, summary, category_id, adult, a, b], i) => ({
  mod_id: 100 + i,
  name,
  summary,
  version: `${1 + (i % 3)}.${i % 7}.0`,
  picture_url: i % 5 === 4 ? null : art(a, b, name),
  updated_timestamp: 1_761_000_000 - i * 86_400,
  status: "published",
  author: ["Ayse", "NightOwl", "Mireille", "kuro", "Tomek"][i % 5],
  category_id,
  contains_adult_content: adult,
  endorsement_count: 20 + i * 137,
  mod_downloads: 800 + i * 4321,
}));

const library = [
  {
    name: "Neon Streets Reloaded",
    enabled: true,
    files: ["archive/pc/mod/neon.archive"],
    installed_files: ["archive/pc/mod/neon.archive"],
    created_dirs: [],
    description: "Deeper neon colours and wet reflections across Night City.",
    version: "1.0.0",
    image: null,
    source: null,
    nexus_id: 100,
  },
];

const handlers: Record<string, (args: Record<string, unknown>) => unknown> = {
  detect_games: () => ["D:\\SteamLibrary\\steamapps\\common\\Cyberpunk 2077"],
  get_prefs: () => ({ locale: null, theme: null, motion: null, translate: null, updates: null, translate_lang: null }),
  set_prefs: () => null,
  get_game_dir: () => "D:\\SteamLibrary\\steamapps\\common\\Cyberpunk 2077",
  set_game_dir: () => null,
  game_running: () => false,
  list_mods: () => library,
  scan_external: () => [],
  mod_image: () => null,
  app_info: () => ({ version: "0.0.3-beta", data_dir: "C:\\Users\\you\\AppData\\Roaming\\com.skydev.cp2077modlauncher" }),
  nc_fetch: () => ({ updated: "", latest: "0.1.1-beta.2", offline: false, releases: [] }),
  nc_detect: () => null,
  nexus_status: () => ({ connected: true, user: { name: "Ayse", is_premium: true, is_supporter: false } }),
  nexus_check: () => [
    { name: "Neon Streets Reloaded", mod_id: 100, installed: "1.0.0", latest: "1.2.0", update: true, checked_at: 1, page_url: "https://www.nexusmods.com/cyberpunk2077/mods/100", error: null },
  ],
  nexus_browse: (args) => {
    const kind = String(args.kind);
    const order = kind === "latest_added" ? [...mods].reverse() : kind === "latest_updated" ? [...mods].sort((x, y) => y.mod_id % 5 - x.mod_id % 5) : mods;
    return { items: order, categories, fetched_at: 1, installed: [100] };
  },
  nexus_search: (args) => {
    const p = args.params as { offset: number; count: number; query: string };
    const all = mods.filter((m) => !p.query || (m.name ?? "").toLowerCase().includes(p.query.toLowerCase())).map((m) => ({ ...m, tags: ["Visuals", "Cars"] }));
    return { items: all.slice(p.offset, p.offset + p.count), total: all.length };
  },
  nxm_status: () => "none",
  nxm_register: (a) => (a.enable ? "ours" : "none"),
  take_pending_nxm: () => null,
  downloads_recent: () => [],
  nexus_mod_name: () => "Neon Streets Reloaded",
  translate_notes: (a) => (a.lines as string[]).map((l) => `[${a.lang}] ${l}`),
  nexus_requirements: () => [{ mod_id: 107, name: "Cyber Engine Tweaks", url: "", external: false, notes: "" }, { mod_id: 1511, name: "redscript", url: "", external: false, notes: "0.5.32" }, { mod_id: 0, name: "Some Tool", url: "", external: true, notes: "" }],
  profile_build: () => ({ code: "CP2077:abc", json: "{}", count: 3, skipped: ["Hand made"] }),
  profile_open: () => ({ name: "My setup", encoded: "CP2077:abc", items: [{ source: "nexus", mod_id: 107, version: "1.38.1", name: "Cyber Engine Tweaks", installed: true }, { source: "nexus", mod_id: 1511, version: "0.5.32", name: "redscript", installed: false }] }),
  profile_share: () => ({ code: "ABCD2345", expires: Math.floor(Date.now() / 1000) + 43200 }),
  nexus_install: () => ["Installed"],
  logs_scan: () => [],
  logs_summary: () => null,
};

export function installMock() {
  let nextCallback = 1;
  const w = window as unknown as Record<string, unknown>;
  w.__TAURI_INTERNALS__ = {
    metadata: {
      currentWindow: { label: "main" },
      currentWebview: { windowLabel: "main", label: "main" },
    },
    transformCallback: () => nextCallback++,
    unregisterCallback: () => {},
    convertFileSrc: (p: string) => p,
    invoke: async (cmd: string, args: Record<string, unknown> = {}) => {
      if (cmd.startsWith("plugin:")) return 1;
      const h = handlers[cmd];
      if (!h) throw { code: "io", message: `mock: ${cmd} is not mocked`, details: null };
      return h(args);
    },
  };
  w.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener: () => {} };
}
