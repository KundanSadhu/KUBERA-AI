//! Typed OfficeCLI adapter.
//!
//! Only high-level KUBERA document operations are represented here. Neither a
//! renderer nor an agent receives a general command string or arbitrary output
//! path. The adapter owns file names, report-root confinement, child-process
//! arguments, timeout, and non-empty-output validation.

use std::{
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::Duration,
};
use wait_timeout::ChildExt;
use zip::{write::FileOptions, ZipArchive, ZipWriter};

use crate::{
    files,
    models::{
        CreateDeckRequest, CreateDocumentRequest, CreateTemplatedDocxRequest,
        CreateWorkbookRequest, OfficeExecution,
    },
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

/// Returns a bounded, text-only OfficeCLI outline for a registered local file.
/// This deliberately does not expose `view html`, a browser process, arbitrary
/// render paths, or generic OfficeCLI commands to the renderer.
pub fn preview_outline(paths: &AppPaths, report: &Path) -> Result<(String, bool), String> {
    const MAX_PREVIEW_BYTES: usize = 64 * 1024;
    let report_path = report.to_string_lossy().to_string();
    let mut child = Command::new(office_binary(paths))
        .args(["view", report_path.as_str(), "outline"])
        .stdout(Stdio::piped())
        // Keep diagnostics off the renderer boundary; the status code is
        // sufficient for this typed operation and stderr may contain paths.
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| format!("Could not start OfficeCLI outline preview: {error}"))?;
    let mut stdout = child
        .stdout
        .take()
        .ok_or_else(|| "Could not capture the OfficeCLI preview stream.".to_string())?;
    let reader = thread::spawn(move || {
        let mut bytes = Vec::new();
        let mut buffer = [0_u8; 8_192];
        loop {
            let count = stdout.read(&mut buffer)?;
            if count == 0 {
                return Ok(bytes);
            }
            // Drain the full stream so OfficeCLI never blocks on a full pipe,
            // while retaining only the bounded prefix for the renderer.
            let remaining = MAX_PREVIEW_BYTES
                .saturating_add(1)
                .saturating_sub(bytes.len());
            bytes.extend_from_slice(&buffer[..count.min(remaining)]);
        }
    });
    let status = child
        .wait_timeout(Duration::from_secs(20))
        .map_err(|error| error.to_string())?;
    let bytes = match status {
        Some(status) if status.success() => reader
            .join()
            .map_err(|_| "OfficeCLI preview reader stopped unexpectedly.".to_string())?
            .map_err(|error| error.to_string())?,
        Some(status) => {
            let _ = reader.join();
            return Err(format!("OfficeCLI outline preview failed with {status}."));
        }
        None => {
            let _ = child.kill();
            let _ = child.wait();
            let _ = reader.join();
            return Err("OfficeCLI outline preview timed out after 20 seconds.".into());
        }
    };
    let truncated = bytes.len() > MAX_PREVIEW_BYTES;
    let bytes = if truncated {
        &bytes[..MAX_PREVIEW_BYTES]
    } else {
        &bytes
    };
    let outline = String::from_utf8_lossy(bytes).trim().to_string();
    if outline.is_empty() {
        return Err("OfficeCLI returned an empty artifact outline.".into());
    }
    Ok((outline, truncated))
}

/// Produces one static OfficeCLI HTML render for a file already confined to
/// KUBERA's report root. The transient output remains in that root, is bounded
/// before it crosses IPC, and is deleted before this function returns.
pub fn preview_rendered_html(
    paths: &AppPaths,
    report: &Path,
    token: &str,
) -> Result<String, String> {
    const MAX_RENDER_BYTES: u64 = 2 * 1024 * 1024;
    if token.is_empty()
        || token.len() > 80
        || !token
            .chars()
            .all(|value| value.is_ascii_hexdigit() || value == '-')
    {
        return Err("Internal visual preview token failed validation.".into());
    }
    let output = paths.reports.join(format!(".kubera-render-{token}.html"));
    let binary = office_binary(paths);
    let args = [
        "view".into(),
        report.to_string_lossy().to_string(),
        "html".into(),
        "-o".into(),
        output.to_string_lossy().to_string(),
    ];
    if let Err(error) = run(&binary, &args) {
        let _ = fs::remove_file(&output);
        return Err(format!("OfficeCLI visual render did not complete: {error}"));
    }
    let result = (|| {
        let metadata = fs::metadata(&output)
            .map_err(|_| "OfficeCLI did not produce a static HTML render.".to_string())?;
        if metadata.len() == 0 || metadata.len() > MAX_RENDER_BYTES {
            return Err("OfficeCLI visual render exceeded KUBERA's 2 MiB preview limit.".into());
        }
        let html = fs::read_to_string(&output)
            .map_err(|_| "OfficeCLI visual render was not valid UTF-8 HTML.".to_string())?;
        let lowercase = html.to_ascii_lowercase();
        if !lowercase.contains("<html") {
            return Err("OfficeCLI visual render did not contain an HTML document.".into());
        }
        // Defense in depth for the already-scriptless iframe boundary: static
        // render assets may only be inline/data resources, never network URLs.
        let policy = "<meta http-equiv=\"Content-Security-Policy\" content=\"default-src 'none'; img-src data:; style-src 'unsafe-inline'; font-src data:;\">";
        if let Some(head) = lowercase.find("<head>") {
            let insert_at = head + "<head>".len();
            let mut rendered = html;
            rendered.insert_str(insert_at, policy);
            Ok(rendered)
        } else {
            Ok(format!(
                "<!doctype html><html><head>{policy}</head><body>{html}</body></html>"
            ))
        }
    })();
    let _ = fs::remove_file(&output);
    result
}

fn create_output(
    paths: &AppPaths,
    file_name: &str,
    file_type: &str,
) -> Result<(String, PathBuf, PathBuf), String> {
    let safe_name =
        files::safe_file_name(file_name, file_type).map_err(|error| error.to_string())?;
    let output = paths
        .report_path(&safe_name)
        .map_err(|error| error.to_string())?;
    let binary = office_binary(paths);
    run(
        &binary,
        &["create".into(), output.to_string_lossy().to_string()],
    )?;
    Ok((safe_name, output, binary))
}

fn close_and_register_output(
    binary: &Path,
    paths: &AppPaths,
    output: &Path,
    safe_name: String,
    file_type: &str,
    summary: String,
) -> Result<OfficeExecution, String> {
    // OfficeCLI keeps an in-process document session. Close explicitly to
    // flush the completed artifact before KUBERA registers it in SQLite.
    run(
        binary,
        &["close".into(), output.to_string_lossy().to_string()],
    )?;
    let bytes =
        files::validate_nonempty_report(output, file_type).map_err(|error| error.to_string())?;
    let canonical =
        files::canonical_inside(&paths.reports, output).map_err(|error| error.to_string())?;
    let relative_path = files::relative_display_path(&paths.reports, &canonical)
        .map_err(|error| error.to_string())?;
    Ok(OfficeExecution {
        deliverable_id: None,
        file_name: safe_name,
        relative_path,
        file_type: file_type.into(),
        bytes,
        summary,
    })
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
    let (safe_name, output, binary) = create_output(paths, &request.file_name, "pptx")?;
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
        // Bullets are normal presentation content; they never become
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
    close_and_register_output(
        &binary,
        paths,
        &output,
        safe_name,
        "pptx",
        format!(
            "Created {} slides through typed OfficeCLI operations.",
            request.slides.len()
        ),
    )
}

/// Creates a plain governed DOCX from a bounded title and paragraph list. The
/// document worker cannot supply XML, a template path, or a raw OfficeCLI verb.
pub fn create_docx_document(
    paths: &AppPaths,
    request: &CreateDocumentRequest,
) -> Result<OfficeExecution, String> {
    if request.title.trim().is_empty() || request.title.len() > 240 {
        return Err("Document title must contain 1–240 characters.".into());
    }
    if request.paragraphs.len() > 40
        || request
            .paragraphs
            .iter()
            .any(|item| item.trim().is_empty() || item.len() > 1_200)
    {
        return Err(
            "Document paragraphs must contain 1–40 non-empty items of at most 1,200 characters."
                .into(),
        );
    }
    let (safe_name, output, binary) = create_output(paths, &request.file_name, "docx")?;
    run(
        &binary,
        &[
            "add".into(),
            output.to_string_lossy().to_string(),
            "/body".into(),
            "--type".into(),
            "paragraph".into(),
            "--prop".into(),
            format!("text={}", request.title.trim()),
            "--prop".into(),
            "style=Heading1".into(),
        ],
    )?;
    for paragraph in &request.paragraphs {
        run(
            &binary,
            &[
                "add".into(),
                output.to_string_lossy().to_string(),
                "/body".into(),
                "--type".into(),
                "paragraph".into(),
                "--prop".into(),
                format!("text={}", paragraph.trim()),
            ],
        )?;
    }
    close_and_register_output(
        &binary,
        paths,
        &output,
        safe_name,
        "docx",
        format!(
            "Created governed DOCX with {} body paragraph(s).",
            request.paragraphs.len()
        ),
    )
}

/// Merges bounded values into an approved DOCX template already copied under
/// KUBERA's template root. The caller can never provide a source path, XML, or
/// raw merge syntax. OfficeCLI validates the completed OOXML package before the
/// artifact is eligible for registration.
pub fn merge_docx_template(
    paths: &AppPaths,
    template: &Path,
    request: &CreateTemplatedDocxRequest,
) -> Result<OfficeExecution, String> {
    if request.values.len() > 40 {
        return Err("Template merge may provide at most 40 placeholder values.".into());
    }
    let mut replacements = std::collections::BTreeMap::new();
    for item in &request.values {
        if !valid_placeholder_key(&item.key) || item.value.len() > 1_200 {
            return Err("Template placeholder names or values exceed KUBERA policy.".into());
        }
        if replacements
            .insert(item.key.clone(), xml_escape(&item.value))
            .is_some()
        {
            return Err("Template placeholder keys must be unique.".into());
        }
    }
    let safe_name =
        files::safe_file_name(&request.file_name, "docx").map_err(|error| error.to_string())?;
    let output = paths
        .report_path(&safe_name)
        .map_err(|error| error.to_string())?;
    copy_merged_docx(template, &output, &replacements)?;
    // Use OfficeCLI only for deterministic package validation here: template
    // copying is an owned byte-to-byte operation, not a general document tool.
    let binary = office_binary(paths);
    run(
        &binary,
        &["validate".into(), output.to_string_lossy().to_string()],
    )?;
    let bytes =
        files::validate_nonempty_report(&output, "docx").map_err(|error| error.to_string())?;
    let canonical =
        files::canonical_inside(&paths.reports, &output).map_err(|error| error.to_string())?;
    let relative_path = files::relative_display_path(&paths.reports, &canonical)
        .map_err(|error| error.to_string())?;
    Ok(OfficeExecution {
        deliverable_id: None,
        file_name: safe_name,
        relative_path,
        file_type: "docx".into(),
        bytes,
        summary: format!(
            "Merged {} approved DOCX placeholder value(s).",
            replacements.len()
        ),
    })
}

fn copy_merged_docx(
    template: &Path,
    output: &Path,
    replacements: &std::collections::BTreeMap<String, String>,
) -> Result<(), String> {
    let input = File::open(template)
        .map_err(|error| format!("Could not open approved template: {error}"))?;
    let mut archive = ZipArchive::new(input)
        .map_err(|_| "Approved template is not a valid DOCX package.".to_string())?;
    let destination =
        File::create(output).map_err(|error| format!("Could not create merged DOCX: {error}"))?;
    let mut writer = ZipWriter::new(destination);
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).map_err(|error| error.to_string())?;
        let name = entry.name().to_string();
        let options = FileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        if entry.is_dir() {
            writer
                .add_directory(name, options)
                .map_err(|error| error.to_string())?;
            continue;
        }
        let mut content = Vec::new();
        entry
            .read_to_end(&mut content)
            .map_err(|error| error.to_string())?;
        if name.starts_with("word/") && name.ends_with(".xml") {
            let xml = String::from_utf8(content)
                .map_err(|_| "DOCX XML content was not UTF-8.".to_string())?;
            let merged = replacements.iter().fold(xml, |value, (key, replacement)| {
                value.replace(&format!("{{{{{key}}}}}"), replacement)
            });
            if merged.contains("{{") && merged.contains("}}") {
                return Err("Template contains unresolved placeholders. Keep each {{placeholder}} token in one Word text run and provide every required value.".into());
            }
            content = merged.into_bytes();
        }
        writer
            .start_file(name, options)
            .map_err(|error| error.to_string())?;
        writer
            .write_all(&content)
            .map_err(|error| error.to_string())?;
    }
    writer.finish().map_err(|error| error.to_string())?;
    Ok(())
}

fn valid_placeholder_key(key: &str) -> bool {
    !key.is_empty()
        && key.len() <= 80
        && key.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.')
        })
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Creates a governed XLSX table in its default Sheet1 worksheet. The request
/// exposes data cells only; it cannot select arbitrary sheets, formulas, files,
/// or OfficeCLI commands.
pub fn create_xlsx_workbook(
    paths: &AppPaths,
    request: &CreateWorkbookRequest,
) -> Result<OfficeExecution, String> {
    if request.sheet_name != "Sheet1" {
        return Err("KUBERA workbook policy permits only the default Sheet1 worksheet.".into());
    }
    if request.columns.is_empty()
        || request.columns.len() > 12
        || request
            .columns
            .iter()
            .any(|item| item.trim().is_empty() || item.len() > 120)
    {
        return Err(
            "Workbook requires 1–12 non-empty column names of at most 120 characters.".into(),
        );
    }
    if request.rows.len() > 500
        || request.rows.iter().any(|row| {
            row.len() != request.columns.len() || row.iter().any(|cell| cell.len() > 300)
        })
    {
        return Err("Workbook rows must match the bounded column count and cell policy.".into());
    }
    let (safe_name, output, binary) = create_output(paths, &request.file_name, "xlsx")?;
    for (column, value) in request.columns.iter().enumerate() {
        set_sheet_cell(&binary, &output, column, 1, value, true)?;
    }
    for (row_index, row) in request.rows.iter().enumerate() {
        for (column, value) in row.iter().enumerate() {
            set_sheet_cell(&binary, &output, column, row_index + 2, value, false)?;
        }
    }
    close_and_register_output(
        &binary,
        paths,
        &output,
        safe_name,
        "xlsx",
        format!(
            "Created governed XLSX table with {} row(s).",
            request.rows.len()
        ),
    )
}

fn set_sheet_cell(
    binary: &Path,
    output: &Path,
    column: usize,
    row: usize,
    value: &str,
    bold: bool,
) -> Result<(), String> {
    let reference = format!("/Sheet1/{}{}", spreadsheet_column_name(column)?, row);
    let mut args = vec![
        "set".into(),
        output.to_string_lossy().to_string(),
        reference,
        "--prop".into(),
        format!("value={value}"),
    ];
    if bold {
        args.push("--prop".into());
        args.push("bold=true".into());
    }
    run(binary, &args).map(|_| ())
}

fn spreadsheet_column_name(index: usize) -> Result<String, String> {
    if index >= 12 {
        return Err("Workbook column exceeds KUBERA policy.".into());
    }
    Ok(((b'A' + index as u8) as char).to_string())
}
