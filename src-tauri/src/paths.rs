use std::{
    fs, io,
    path::{Path, PathBuf},
};
use tauri::{AppHandle, Manager};

#[derive(Clone)]
pub struct AppPaths {
    pub root: PathBuf,
    pub resources: Option<PathBuf>,
    pub database: PathBuf,
    pub reports: PathBuf,
    pub templates: PathBuf,
    pub logs: PathBuf,
}

impl AppPaths {
    pub fn from_app(app: &AppHandle) -> io::Result<Self> {
        let root = app
            .path()
            .app_data_dir()
            .map_err(|error| io::Error::other(error.to_string()))?;
        let resources = app.path().resource_dir().ok();
        Ok(Self {
            resources,
            database: root.join("kubera.db"),
            reports: root.join("reports"),
            templates: root.join("templates"),
            logs: root.join("logs"),
            root,
        })
    }

    pub fn prepare(&self) -> io::Result<()> {
        fs::create_dir_all(&self.reports)?;
        fs::create_dir_all(&self.templates)?;
        fs::create_dir_all(&self.logs)?;
        Ok(())
    }

    /// Converts an internally generated relative output path to a canonical
    /// report-root location. User/model supplied absolute or traversal paths
    /// are deliberately rejected.
    pub fn report_path(&self, relative_name: &str) -> io::Result<PathBuf> {
        if relative_name.is_empty() || Path::new(relative_name).is_absolute() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "output must be a relative report name",
            ));
        }
        let candidate = self.reports.join(relative_name);
        let parent = candidate
            .parent()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "missing report parent"))?;
        fs::create_dir_all(parent)?;
        let canonical_root = self.reports.canonicalize()?;
        let canonical_parent = parent.canonicalize()?;
        if !canonical_parent.starts_with(&canonical_root) {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "report path escapes output root",
            ));
        }
        Ok(candidate)
    }
}
