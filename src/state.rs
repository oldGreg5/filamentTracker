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
        let rendered = self.tera.read().await.render(name, ctx).map_err(anyhow::Error::from)?;
        Ok(rendered)
    }
}
