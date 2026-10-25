//! Typed OfficeCLI adapter.
//!
//! Only high-level KUBERA document operations are represented here. Neither a
//! renderer nor an agent receives a general command string or arbitrary output
//! path. The adapter owns file names, report-root confinement, child-process
//! arguments, timeout, and non-empty-output validation.

use std::{
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};
use wait_timeout::ChildExt;

use crate::{
    files,
    models::{CreateDeckRequest, OfficeExecution},
    paths::AppPaths,
};

fn office_binary(paths: &AppPaths) -> PathBuf {
    if let Ok(value) = std::env::var("KUBERA_OFFICECLI_BIN") {
        return PathBuf::from(value);
    }
    #[cfg(target_os = "windows")]
    let name = "officecli-x86_64-pc-windows-msvc.exe";
    #[cfg(target_os = "macos")]
    let name = "officecli-aarch64-apple-darwin";
    #[cfg(target_os = "linux")]
    let name = "officecli-x86_64-unknown-linux-gnu";
    if let Some(resources) = paths.resources.as_ref() {
        let bundled = resources.join(name);
        if bundled.exists() {
            return bundled;
        }
    }
    PathBuf::from("binaries").join(name)
}

fn run(binary: &Path, args: &[String]) -> Result<String, String> {
    let mut child = Command::new(binary)
        .args(args)
        .spawn()
        .map_err(|error| format!("Could not start OfficeCLI '{}': {error}", binary.display()))?;
    let status = child
        .wait_timeout(Duration::from_secs(45))
        .map_err(|error| error.to_string())?;
    match status {
        Some(status) if status.success() => Ok(format!("OfficeCLI completed with {status}")),
        Some(status) => Err(format!("OfficeCLI failed with {status}")),
        None => {
            let _ = child.kill();
            let _ = child.wait();
            Err("OfficeCLI timed out after 45 seconds.".into())
        }
    }
}

pub fn create_pptx_deck(
    paths: &AppPaths,
    request: &CreateDeckRequest,
) -> Result<OfficeExecution, String> {
    if request.title.trim().is_empty() || request.title.len() > 240 {
        return Err("Deck title must contain 1–240 characters.".into());
    }
    if request.slides.is_empty() || request.slides.len() > 20 {
        return Err("Deck requires 1–20 slides.".into());
    }
    let safe_name =
        files::safe_file_name(&request.file_name, "pptx").map_err(|error| error.to_string())?;
    let output = paths
        .report_path(&safe_name)
        .map_err(|error| error.to_string())?;
    let binary = office_binary(paths);

    // Create the file first. Subsequent calls receive no model-controlled path:
    // they use the report-root output resolved above.
    run(
        &binary,
        &["create".into(), output.to_string_lossy().to_string()],
    )?;
    for slide in &request.slides {
        if slide.title.trim().is_empty() || slide.title.len() > 240 {
            return Err("Every slide needs a 1–240 character title.".into());
        }
        let mut slide_args = vec![
            "add".into(),
            output.to_string_lossy().to_string(),
            "/".into(),
            "--type".into(),
            "slide".into(),
            "--prop".into(),
            format!("title={}", slide.title.trim()),
        ];
        // Bullets are combined as normal presentation content; they never become
        // executable arguments or paths.
        if !slide.bullets.is_empty() {
            if slide.bullets.len() > 12 || slide.bullets.iter().any(|item| item.len() > 300) {
                return Err("Slide bullets exceed document policy limits.".into());
            }
            slide_args.push("--prop".into());
            slide_args.push(format!("body={}", slide.bullets.join("\n• ")));
        }
        run(&binary, &slide_args)?;
    }
    // OfficeCLI keeps an in-process document session. Close explicitly to
    // flush the completed deck to the KUBERA-owned report path before we
    // register it in SQLite.
    run(
        &binary,
        &["close".into(), output.to_string_lossy().to_string()],
    )?;

    let bytes =
        files::validate_nonempty_report(&output, "pptx").map_err(|error| error.to_string())?;
    let canonical =
        files::canonical_inside(&paths.reports, &output).map_err(|error| error.to_string())?;
    let relative_path = files::relative_display_path(&paths.reports, &canonical)
        .map_err(|error| error.to_string())?;
    Ok(OfficeExecution {
        file_name: safe_name,
        relative_path,
        file_type: "pptx".into(),
        bytes,
        summary: format!(
            "Created {} slides through typed OfficeCLI operations.",
            request.slides.len()
        ),
    })
}
