use axum::extract::State;
use axum::response::{Html, IntoResponse};
use tera::Context;

use crate::error::AppError;
use crate::state::AppState;

pub async fn index(State(state): State<AppState>) -> Result<impl IntoResponse, AppError> {
    let brand_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM brands")
        .fetch_one(&state.pool)
        .await?;
    let material_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM materials")
        .fetch_one(&state.pool)
        .await?;
    let colour_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM colours")
        .fetch_one(&state.pool)
        .await?;
    let spool_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM spools")
        .fetch_one(&state.pool)
        .await?;

    let mut ctx = Context::new();
    ctx.insert("brand_count", &brand_count);
    ctx.insert("material_count", &material_count);
    ctx.insert("colour_count", &colour_count);
    ctx.insert("spool_count", &spool_count);
    ctx.insert("active_nav", "home");

    let body = state.render("index.html", &ctx).await?;
    Ok(Html(body))
}
