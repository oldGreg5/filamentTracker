use std::path::PathBuf;
use std::sync::Arc;

use sqlx::SqlitePool;
use tera::Tera;
use tokio::sync::RwLock;

use crate::error::AppError;

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub tera: Arc<RwLock<Tera>>,
    pub image_dir: PathBuf,
    /// Changes every process start, so static asset URLs (?v=...) become new
    /// URLs on every deploy — otherwise Cloudflare/browser caches keep serving
    /// the old /static/app.js and /static/style.css indefinitely.
    pub asset_version: String,
}

impl AppState {
    /// Renders a template. In debug builds, reloads templates from disk first so
    /// edits show up on refresh without a rebuild (see SPEC.md section 2).
    pub async fn render(&self, name: &str, ctx: &tera::Context) -> Result<String, AppError> {
        if cfg!(debug_assertions) {
            self.tera
                .write()
                .await
                .full_reload()
                .map_err(anyhow::Error::from)?;
        }
        let mut ctx = ctx.clone();
        ctx.insert("asset_v", &self.asset_version);
        let rendered = self.tera.read().await.render(name, &ctx).map_err(anyhow::Error::from)?;
        Ok(rendered)
    }
}
