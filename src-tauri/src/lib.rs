mod archive;
mod commands;
mod conflicts;
mod error;
mod fomod;
mod frameworks;
mod game;
mod layout;
mod library;
mod logs;
mod meta;
mod nexus;
mod nightcoop;
mod nxm;
mod presets;
mod profile;
mod report;
mod version;
mod scan;
mod translate;

use commands::AppState;
use tauri::{Emitter, Manager};
use tracing_appender::rolling::{RollingFileAppender, Rotation};

fn init_logging(data: &std::path::Path) -> Option<tracing_appender::non_blocking::WorkerGuard> {
    let appender = RollingFileAppender::builder()
        .rotation(Rotation::DAILY)
        .filename_prefix("launcher")
        .filename_suffix("log")
        .max_log_files(7)
        .build(data.join("logs"))
        .ok()?;
    let (writer, guard) = tracing_appender::non_blocking(appender);
    tracing_subscriber::fmt()
        .with_writer(writer)
        .with_ansi(false)
        .try_init()
        .ok()?;
    Some(guard)
}

fn nxm_in(args: &[String]) -> Option<nxm::NxmLink> {
    args.iter().find(|a| a.len() >= 6 && a[..6].eq_ignore_ascii_case("nxm://")).and_then(|a| nxm::parse(a).ok())
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            if let Some(link) = nxm_in(&args) {
                let _ = app.emit("nxm-link", link);
            }
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.unminimize();
                let _ = w.set_focus();
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            let data = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data)?;
            if let Some(guard) = init_logging(&data) {
                app.manage(guard);
            }
            app.manage(AppState { data });
            let args: Vec<String> = std::env::args().skip(1).collect();
            app.manage(commands::NxmState { pending: std::sync::Mutex::new(nxm_in(&args)) });
            app.manage(commands::NexusState {
                store: Box::new(nexus::CredStore),
            });
            let handle = app.handle().clone();
            std::panic::set_hook(Box::new(move |info| {
                let message = info.to_string();
                tracing::error!("panic: {message}");
                let _ = handle.emit("fatal-error", message);
            }));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::detect_games,
            commands::get_game_dir,
            commands::set_game_dir,
            commands::get_prefs,
            commands::set_prefs,
            commands::game_running,
            commands::list_mods,
            commands::scan_external,
            commands::import_mod,
            commands::nc_fetch,
            commands::nc_install,
            commands::nexus_status,
            commands::nexus_connect,
            commands::nexus_disconnect,
            commands::nexus_link,
            commands::nexus_unlink,
            commands::nexus_check,
            commands::nexus_browse,
            commands::translate_notes,
            commands::nexus_fill_covers,
            commands::framework_status,
            commands::mod_health,
            commands::game_version,
            commands::game_patch_time,
            commands::fomod_image,
            commands::launch_game,
            commands::framework_plan,
            commands::framework_remove,
            commands::fomod_pending,
            commands::fomod_eval,
            commands::fomod_cancel,
            commands::fomod_install,
            commands::nexus_files,
            commands::nexus_install_file,
            commands::shares_list,
            commands::get_install_id,
            commands::reset_install_id,
            commands::presets_list,
            commands::preset_save,
            commands::preset_rename,
            commands::preset_delete,
            commands::preset_apply,
            commands::profile_build,
            commands::profile_open,
            commands::profile_share,
            commands::profile_save,
            commands::profile_read,
            commands::nexus_search,
            commands::nexus_requirements,
            commands::nexus_install,
            commands::nexus_install_nxm,
            commands::nexus_mod_name,
            commands::take_pending_nxm,
            commands::nxm_status,
            commands::nxm_register,
            commands::downloads_recent,
            commands::install_download,
            commands::nexus_probe,
            commands::open_url,
            commands::install_mod,
            commands::set_mod_info,
            commands::mod_image,
            commands::export_mod,
            commands::export_pack,
            commands::app_info,
            commands::enable_mod,
            commands::disable_mod,
            commands::uninstall_mod,
            commands::logs_scan,
            commands::logs_read,
            commands::logs_head,
            commands::logs_poll,
            commands::logs_summary,
            commands::logs_reveal,
            commands::logs_open,
            commands::logs_clear,
            commands::nc_detect,
            commands::build_error_report,
            commands::save_error_report,
            commands::open_folder,
        ])
        .run(tauri::generate_context!())
        .expect("failed to start the launcher");
}
