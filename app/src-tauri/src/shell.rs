//! The Tauri commands. Each one turns the arguments into a path and a value,
//! calls a function of [`crate::commands`] or [`crate::signoff`] on a worker
//! thread (a command that is not `async` would run on the main thread and freeze
//! the window while Word exports), and gives the result back to the page.

use std::path::{Path, PathBuf};

use adapdf::fix::{Environment, FixOptions, RealFs, Timestamp};
use adapdf::fonts::SystemFonts;
use tauri_plugin_opener::OpenerExt;

use crate::commands::{
    self, ApplyResult, CheckResult, Choice, CommandError, CommandResult, ExportResult, OfferView,
    ReportView, WorkingInfo,
};
use crate::signoff::{self, Signoff};

/// Runs work that touches files or starts Word on a worker thread.
async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> CommandResult<T> + Send + 'static,
) -> CommandResult<T> {
    tauri::async_runtime::spawn_blocking(work)
        .await
        .map_err(|e| CommandError::new(format!("The work stopped unexpectedly ({e}).")))?
}

fn with_environment<T>(work: impl FnOnce(&Environment) -> T) -> T {
    let fonts = SystemFonts::from_environment();
    let env = Environment {
        fs: &RealFs,
        now: Timestamp::now(),
        fonts: &fonts,
    };
    work(&env)
}

/// Which files belong to a guide, and whether its accessible copy exists.
#[tauri::command]
async fn working_info(path: String) -> CommandResult<WorkingInfo> {
    blocking(move || {
        Ok(with_environment(|env| {
            commands::working_info(Path::new(&path), env)
        }))
    })
    .await
}

/// Makes the accessible copy with no fix, so that the person can open it in Word.
#[tauri::command]
async fn create_copy(path: String, start_over: bool) -> CommandResult<WorkingInfo> {
    blocking(move || {
        with_environment(|env| commands::create_copy(Path::new(&path), start_over, env))
    })
    .await
}

/// Opens the accessible copy in the person's own Word, with the program that
/// Windows has for .docx files. The app does not control that Word afterwards.
/// Only the accessible copy of a guide is opened (see [`commands::check_open_in_word`]).
#[tauri::command]
async fn open_in_word(app: tauri::AppHandle, path: String) -> CommandResult<()> {
    let file: PathBuf = commands::check_open_in_word(Path::new(&path))?;
    app.opener()
        .open_path(file.to_string_lossy(), None::<&str>)
        .map_err(|e| {
            CommandError::new(format!(
                "The document could not be opened in Word ({e}). Open {} yourself.",
                file.display()
            ))
        })
}

/// Opens the folder that holds a .docx file, in the file explorer.
#[tauri::command]
async fn open_folder(app: tauri::AppHandle, path: String) -> CommandResult<()> {
    let folder: PathBuf = commands::check_open_folder(Path::new(&path))?;
    app.opener()
        .open_path(folder.to_string_lossy(), None::<&str>)
        .map_err(|e| {
            CommandError::new(format!(
                "The folder could not be opened ({e}). Open {} yourself.",
                folder.display()
            ))
        })
}

#[tauri::command]
async fn check(path: String) -> CommandResult<CheckResult> {
    blocking(move || commands::check(Path::new(&path), &SystemFonts::from_environment())).await
}

#[tauri::command]
async fn fix_offers(path: String) -> CommandResult<Vec<OfferView>> {
    blocking(move || {
        with_environment(|env| commands::fix_offers(Path::new(&path), env, &FixOptions::default()))
    })
    .await
}

#[tauri::command]
async fn apply_fixes(
    path: String,
    choices: Vec<Choice>,
    start_over: bool,
) -> CommandResult<ApplyResult> {
    blocking(move || {
        with_environment(|env| {
            commands::apply_fixes(
                Path::new(&path),
                &choices,
                start_over,
                env,
                &FixOptions::default(),
            )
        })
    })
    .await
}

/// Runs the export. It can take minutes (Word has 5 minutes), so it has its own
/// worker thread, and the page shows that it is working.
#[tauri::command]
async fn export(path: String) -> CommandResult<ExportResult> {
    blocking(move || commands::export_with_word(Path::new(&path), &SystemFonts::from_environment()))
        .await
}

/// Opens the HTML report in the default browser. Only the HTML file of a report on
/// a local drive is opened (see [`commands::check_report_html`]).
#[tauri::command]
async fn open_report(app: tauri::AppHandle, path: String) -> CommandResult<()> {
    let file: PathBuf = commands::check_report_html(Path::new(&path))?;
    app.opener()
        .open_path(file.to_string_lossy(), None::<&str>)
        .map_err(|e| {
            CommandError::new(format!(
                "The report could not be opened in the browser ({e}). Open {} yourself.",
                file.display()
            ))
        })
}

#[tauri::command]
async fn read_report(path: String) -> CommandResult<ReportView> {
    blocking(move || commands::read_report(Path::new(&path))).await
}

#[tauri::command]
async fn save_signoff(report_json: String, signoff: Signoff) -> CommandResult<ReportView> {
    blocking(move || signoff::save_signoff(Path::new(&report_json), &signoff)).await
}

/// Starts the window.
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            check,
            working_info,
            create_copy,
            open_in_word,
            open_folder,
            fix_offers,
            apply_fixes,
            export,
            open_report,
            read_report,
            save_signoff
        ])
        .run(tauri::generate_context!())
        .expect("the window could not be started");
}
