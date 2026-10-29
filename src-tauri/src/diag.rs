//! Diagnostics: everything that is logged also goes to daily files under
//! %LOCALAPPDATA%\VoidTrace\logs (7 kept), panics and UI errors included, and a
//! bug report bundles those files with a description into one zip to send.

use anyhow::{Context, Result};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use tracing_appender::non_blocking::WorkerGuard;

static GUARD: OnceLock<WorkerGuard> = OnceLock::new();

pub fn log_dir() -> PathBuf {
    wfc_core::data_dir().join("logs")
}

/// Console plus a daily log file. Call once, before anything logs.
pub fn init_logging() {
    // tracing_appender prunes old files by listing the directory; it must exist first.
    let _ = std::fs::create_dir_all(log_dir());
    use tracing_subscriber::{fmt, prelude::*, EnvFilter};
    let filter = || EnvFilter::new("info,voidtrace=debug,wfc_core=debug,ui=debug");
    let registry = tracing_subscriber::registry().with(fmt::layer().with_filter(filter()));
    let file = tracing_appender::rolling::Builder::new()
        .rotation(tracing_appender::rolling::Rotation::DAILY)
        .filename_prefix("voidtrace")
        .filename_suffix("log")
        .max_log_files(7)
        .build(log_dir());
    match file {
        Ok(f) => {
            let (writer, guard) = tracing_appender::non_blocking(f);
            let _ = GUARD.set(guard);
            registry.with(fmt::layer().with_ansi(false).with_writer(writer).with_filter(filter())).init();
        }
        Err(e) => {
            registry.init();
            tracing::warn!("no log file: {e}");
        }
    }
    std::panic::set_hook(Box::new(|info| {
        tracing::error!("panic: {info}\n{}", std::backtrace::Backtrace::force_capture());
    }));
}

/// The last `max` bytes of a file (whole lines where possible).
fn tail(path: &Path, max: u64) -> Result<Vec<u8>> {
    let mut f = std::fs::File::open(path)?;
    let len = f.metadata()?.len();
    f.seek(SeekFrom::Start(len.saturating_sub(max)))?;
    let mut buf = Vec::new();
    f.read_to_end(&mut buf)?;
    if len > max {
        if let Some(nl) = buf.iter().position(|&b| b == b'\n') {
            buf.drain(..=nl);
        }
    }
    Ok(buf)
}

pub struct Report<'a> {
    pub version: &'a str,
    pub description: &'a str,
    pub include_game_log: bool,
    pub game_running: bool,
    pub settings: Vec<(String, String)>,
}

/// Write the bug report zip to the desktop (or the data folder) and return its path.
pub fn write_report(r: &Report) -> Result<PathBuf> {
    let now = chrono::Local::now();
    let dir = dirs::desktop_dir().filter(|d| d.is_dir()).unwrap_or_else(wfc_core::data_dir);
    let path = dir.join(format!("VoidTrace-bugreport-{}.zip", now.format("%Y%m%d-%H%M%S")));
    let mut zip = zip::ZipWriter::new(std::fs::File::create(&path).with_context(|| format!("create {}", path.display()))?);
    let opts = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    let ee = wfc_core::ee_log_path();
    let mut txt = String::new();
    txt += "VoidTrace bug report\n\n";
    txt += &format!("Version:      {}\n", r.version);
    txt += &format!("Created:      {}\n", now.format("%Y-%m-%d %H:%M:%S %z"));
    txt += &format!("OS:           {} {}\n", std::env::consts::OS, std::env::consts::ARCH);
    txt += &format!("Game running: {}\n", if r.game_running { "yes" } else { "no" });
    txt += &format!("EE.log:       {} ({})\n", ee.display(), if ee.exists() { "found" } else { "not found" });
    txt += "\nWhat happened:\n";
    txt += if r.description.trim().is_empty() { "(no description)" } else { r.description.trim() };
    txt += "\n\nSettings (tokens and passwords removed):\n";
    for (k, v) in &r.settings {
        let shown = if k.contains("token") || k.contains("password") {
            "<removed>".to_string()
        } else if v.len() > 400 {
            format!("<{} bytes>", v.len())
        } else {
            v.clone()
        };
        txt += &format!("  {k} = {shown}\n");
    }
    zip.start_file("report.txt", opts)?;
    zip.write_all(txt.as_bytes())?;

    // App logs, newest first, at most 2 MB each.
    let mut logs: Vec<_> = std::fs::read_dir(log_dir())
        .map(|it| it.flatten().map(|e| e.path()).filter(|p| p.is_file()).collect())
        .unwrap_or_default();
    logs.sort();
    logs.reverse();
    for p in logs.iter().take(7) {
        if let (Some(name), Ok(data)) = (p.file_name().and_then(|n| n.to_str()), tail(p, 2 << 20)) {
            zip.start_file(format!("logs/{name}"), opts)?;
            zip.write_all(&data)?;
        }
    }

    if r.include_game_log && ee.exists() {
        if let Ok(data) = tail(&ee, 1 << 20) {
            zip.start_file("ee-log-tail.txt", opts)?;
            zip.write_all(&data)?;
        }
    }
    zip.finish()?;
    tracing::info!("bug report written to {}", path.display());
    Ok(path)
}
