use std::collections::HashMap;

use axum::response::{Html, IntoResponse};
use serde::Serialize;
use serde_json::{json, Map, Value};
use sqlx::{Row, SqlitePool};
use tera::Context;

use crate::error::AppError;
use crate::state::AppState;

#[derive(Clone, Copy, Serialize)]
pub struct ColumnDef {
    pub key: &'static str,
    pub label: &'static str,
    pub input_type: &'static str, // "text" | "number" | "color"
    pub required: bool,
}

/// Describes one reference entity (brands/materials/colours) so the generic
/// CRUD handlers and the single Tera partial (templates/partials/reference_macros.html)
/// can be reused across all three (see SPEC.md section 5).
pub struct ReferenceConfig {
    pub entity: &'static str,   // url segment, e.g. "brands"
    pub table: &'static str,    // db table name
    pub singular: &'static str, // e.g. "Brand"
    pub title: &'static str,    // page heading, e.g. "Brands"
    pub columns: &'static [ColumnDef],
    /// Column on `spools` referencing this table's id, used to block deletes that
    /// would orphan a spool, e.g. "brand_id".
    pub spool_fk_column: &'static str,
}

async fn fetch_rows(pool: &SqlitePool, config: &ReferenceConfig) -> Vec<Map<String, Value>> {
    let sql = format!("SELECT * FROM {} ORDER BY name", config.table);
    let rows = sqlx::query(&sql).fetch_all(pool).await.expect("query reference rows");
    rows.into_iter().map(|row| row_to_map(&row, config)).collect()
}

fn row_to_map(row: &sqlx::sqlite::SqliteRow, config: &ReferenceConfig) -> Map<String, Value> {
    let mut map = Map::new();
    let id: i64 = row.try_get("id").expect("id column");
    map.insert("id".into(), json!(id));
    for col in config.columns {
        if col.input_type == "number" {
            let v: Option<f64> = row.try_get(col.key).unwrap_or(None);
            map.insert(col.key.into(), json!(v));
        } else {
            let v: Option<String> = row.try_get(col.key).unwrap_or(None);
            map.insert(col.key.into(), json!(v));
        }
    }
    map
}

async fn fetch_row(pool: &SqlitePool, config: &ReferenceConfig, id: i64) -> Option<Map<String, Value>> {
    let sql = format!("SELECT * FROM {} WHERE id = ?", config.table);
    let row = sqlx::query(&sql).bind(id).fetch_optional(pool).await.expect("query reference row");
    row.map(|r| row_to_map(&r, config))
}

fn base_context(config: &ReferenceConfig) -> Context {
    let mut ctx = Context::new();
    ctx.insert("entity", config.entity);
    ctx.insert("singular", config.singular);
    ctx.insert("columns", config.columns);
    ctx
}

pub async fn page(state: AppState, config: &ReferenceConfig) -> Result<impl IntoResponse, AppError> {
    let items = fetch_rows(&state.pool, config).await;
    let mut ctx = base_context(config);
    ctx.insert("title", config.title);
    ctx.insert("items", &items);
    ctx.insert("active_nav", config.entity);
    let body = state.render("reference_page.html", &ctx).await?;
    Ok(Html(body))
}

async fn render_rows(state: &AppState, config: &ReferenceConfig, error: Option<&str>) -> Result<String, AppError> {
    let items = fetch_rows(&state.pool, config).await;
    let mut ctx = base_context(config);
    ctx.insert("items", &items);
    if let Some(e) = error {
        ctx.insert("error", e);
    }
    Ok(state.render("reference_rows.html", &ctx).await?)
}

fn bind_form_values<'q>(
    mut query: sqlx::query::Query<'q, sqlx::Sqlite, sqlx::sqlite::SqliteArguments<'q>>,
    config: &ReferenceConfig,
    form: &'q HashMap<String, String>,
) -> sqlx::query::Query<'q, sqlx::Sqlite, sqlx::sqlite::SqliteArguments<'q>> {
    for col in config.columns {
        let raw = form.get(col.key).map(|s| s.trim().to_string()).unwrap_or_default();
        if col.input_type == "number" {
            let v: f64 = raw.parse().unwrap_or(0.0);
            query = query.bind(v);
        } else if col.required {
            query = query.bind(raw);
        } else {
            let v = if raw.is_empty() { None } else { Some(raw) };
            query = query.bind(v);
        }
    }
    query
}

pub async fn create(
    state: AppState,
    config: &ReferenceConfig,
    form: HashMap<String, String>,
) -> Result<impl IntoResponse, AppError> {
    let cols: Vec<&str> = config.columns.iter().map(|c| c.key).collect();
    let placeholders = vec!["?"; cols.len()].join(", ");
    let sql = format!(
        "INSERT INTO {} ({}) VALUES ({})",
        config.table,
        cols.join(", "),
        placeholders
    );
    let query = sqlx::query(&sql);
    let query = bind_form_values(query, config, &form);
    query.execute(&state.pool).await?;

    let body = render_rows(&state, config, None).await?;
    Ok(Html(body))
}

pub async fn update(
    state: AppState,
    config: &ReferenceConfig,
    id: i64,
    form: HashMap<String, String>,
) -> Result<impl IntoResponse, AppError> {
    let assignments: Vec<String> = config.columns.iter().map(|c| format!("{} = ?", c.key)).collect();
    let sql = format!(
        "UPDATE {} SET {} WHERE id = ?",
        config.table,
        assignments.join(", ")
    );
    let query = sqlx::query(&sql);
    let query = bind_form_values(query, config, &form);
    query.bind(id).execute(&state.pool).await?;

    let body = render_rows(&state, config, None).await?;
    Ok(Html(body))
}

pub async fn delete(state: AppState, config: &ReferenceConfig, id: i64) -> Result<impl IntoResponse, AppError> {
    let count_sql = format!("SELECT COUNT(*) FROM spools WHERE {} = ?", config.spool_fk_column);
    let in_use: i64 = sqlx::query_scalar(&count_sql).bind(id).fetch_one(&state.pool).await?;

    let body = if in_use > 0 {
        render_rows(
            &state,
            config,
            Some(&format!(
                "Cannot delete: {} spool(s) still reference this {}.",
                in_use, config.singular
            )),
        )
        .await?
    } else {
        let sql = format!("DELETE FROM {} WHERE id = ?", config.table);
        sqlx::query(&sql).bind(id).execute(&state.pool).await?;
        render_rows(&state, config, None).await?
    };
    Ok(Html(body))
}

pub async fn edit_fragment(state: AppState, config: &ReferenceConfig, id: i64) -> Result<impl IntoResponse, AppError> {
    let item = fetch_row(&state.pool, config, id).await;
    let mut ctx = base_context(config);
    ctx.insert("item", &item);
    let body = state.render("reference_row_edit.html", &ctx).await?;
    Ok(Html(body))
}

pub async fn view_fragment(state: AppState, config: &ReferenceConfig, id: i64) -> Result<impl IntoResponse, AppError> {
    let item = fetch_row(&state.pool, config, id).await;
    let mut ctx = base_context(config);
    ctx.insert("item", &item);
    let body = state.render("reference_row_view.html", &ctx).await?;
    Ok(Html(body))
}

