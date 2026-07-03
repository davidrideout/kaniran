//! The minijinja environment, wrapped in an auto-reloader: template edits
//! are picked up on the next request, no rebuild. Reload cost is a re-parse
//! of the changed files — negligible next to segmentation.

use std::path::PathBuf;

use minijinja::path_loader;
use minijinja_autoreload::AutoReloader;
use serde::Serialize;

use crate::error::DemoError;

/// Template engine handle stored in [`crate::AppState`].
pub struct Templates {
    reloader: AutoReloader,
}

impl Templates {
    /// Watch `dir` and serve templates from it.
    pub fn new(dir: String) -> Self {
        let dir = PathBuf::from(dir);
        let reloader = AutoReloader::new(move |notifier| {
            let mut env = minijinja::Environment::new();
            notifier.watch_path(&dir, true);
            env.set_loader(path_loader(&dir));
            Ok(env)
        });
        Self { reloader }
    }

    /// Render `name` with `context` (any `Serialize` value).
    pub fn render(&self, name: &str, context: impl Serialize) -> Result<String, DemoError> {
        let env = self.reloader.acquire_env()?;
        Ok(env.get_template(name)?.render(context)?)
    }
}
