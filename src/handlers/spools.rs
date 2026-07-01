use std::collections::HashMap;

use axum::extract::{Form, Multipart, Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{Html, IntoResponse, Redirect, Response};
use chrono::Utc;
use serde::Serialize;
use tera::Context;

use crate::error::AppError;
use crate::models::{SpoolImage, SpoolWithDetails};
use crate::state::AppState;

const SPOOL_LIST_BASE_QUERY: &str = "
SELECT s.id,
       b.name AS brand_name,
       m.name AS material_name,
       c.name AS colour_name,
       c.hex AS colour_hex,
       s.nominal_weight_g,
       s.product_url,
       s.notes,
       s.purchase_date,
       wl.gross_weight_g - COALESCE(s.tare_override_g, b.tare_weight_g) AS remaining_g,
       (wl.gross_weight_g - COALESCE(s.tare_override_g, b.tare_weight_g)) / s.nominal_weight_g AS percent_remaining,
       si.path AS thumbnail_path
FROM spools s
JOIN brands b ON b.id = s.brand_id
JOIN materials m ON m.id = s.material_id
JOIN colours c ON c.id = s.colour_id
LEFT JOIN (
  SELECT spool_id, gross_weight_g, measured_at,
         ROW_NUMBER() OVER (PARTITION BY spool_id ORDER BY measured_at DESC, id DESC) AS rn
  FROM weight_log
) wl ON wl.spool_id = s.id AND wl.rn = 1
LEFT JOIN (
  SELECT spool_id, path,
         ROW_NUMBER() OVER (PARTITION BY spool_id ORDER BY sort_order ASC, id ASC) AS rn
  FROM spool_images
) si ON si.spool_id = s.id AND si.rn = 1
";

/// Whitelisted sort keys -> actual SQL expressions. Never interpolate the raw
/// query param into SQL (see SPEC.md section 7).
fn sort_column(sort: &str) -> &'static str {
    match sort {
        "brand" => "b.name",
        "material" => "m.name",
        "colour" => "c.name",
        "remaining_g" => "remaining_g",
        "percent" => "percent_remaining",
        "purchase_date" => "s.purchase_date",
        _ => "b.name",
    }
}

#[derive(Serialize, sqlx::FromRow)]
struct IdName {
    id: i64,
    name: String,
}

#[derive(Serialize, sqlx::FromRow)]
struct ColourOption {
    id: i64,
    name: String,
    hex: Option<String>,
}

pub async fn list(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, AppError> {
    let filter_brand: Option<i64> = params.get("brand").and_then(|v| v.parse().ok());
    let filter_material: Option<i64> = params.get("material").and_then(|v| v.parse().ok());
    let filter_colour: Option<i64> = params.get("colour").and_then(|v| v.parse().ok());
    let sort = params.get("sort").map(String::as_str).unwrap_or("brand");
    let dir = if params.get("dir").map(String::as_str) == Some("desc") { "DESC" } else { "ASC" };

    let mut sql = SPOOL_LIST_BASE_QUERY.to_string();
    let mut conditions = Vec::new();
    if filter_brand.is_some() {
        conditions.push("s.brand_id = ?");
    }
    if filter_material.is_some() {
        conditions.push("s.material_id = ?");
    }
    if filter_colour.is_some() {
        conditions.push("s.colour_id = ?");
    }
    if !conditions.is_empty() {
        sql.push_str(" WHERE ");
        sql.push_str(&conditions.join(" AND "));
    }
    sql.push_str(&format!(" ORDER BY {} {}", sort_column(sort), dir));

    let mut query = sqlx::query_as::<_, SpoolWithDetails>(&sql);
    if let Some(id) = filter_brand {
        query = query.bind(id);
    }
    if let Some(id) = filter_material {
        query = query.bind(id);
    }
    if let Some(id) = filter_colour {
        query = query.bind(id);
    }
    let spools = query.fetch_all(&state.pool).await?;

    let is_htmx = headers.get("HX-Request").map(|v| v == "true").unwrap_or(false);

    if is_htmx {
        let mut ctx = Context::new();
        ctx.insert("spools", &spools);
        let body = state.render("partials/spool_rows.html", &ctx).await?;
        return Ok(Html(body));
    }

    let brands: Vec<IdName> = sqlx::query_as("SELECT id, name FROM brands ORDER BY name")
        .fetch_all(&state.pool)
        .await?;
    let materials: Vec<IdName> = sqlx::query_as("SELECT id, name FROM materials ORDER BY name")
        .fetch_all(&state.pool)
        .await?;
    let colours: Vec<ColourOption> = sqlx::query_as("SELECT id, name, hex FROM colours ORDER BY name")
        .fetch_all(&state.pool)
        .await?;

    let dir_lower = if dir == "DESC" { "desc" } else { "asc" };
    let mut ctx = Context::new();
    ctx.insert("spools", &spools);
    ctx.insert("brands", &brands);
    ctx.insert("materials", &materials);
    ctx.insert("colours", &colours);
    ctx.insert("sort", sort);
    ctx.insert("dir", dir_lower);
    ctx.insert("filter_brand", &filter_brand);
    ctx.insert("filter_material", &filter_material);
    ctx.insert("filter_colour", &filter_colour);
    ctx.insert("filter_brand_str", &filter_brand.map(|v| v.to_string()).unwrap_or_default());
    ctx.insert("filter_material_str", &filter_material.map(|v| v.to_string()).unwrap_or_default());
    ctx.insert("filter_colour_str", &filter_colour.map(|v| v.to_string()).unwrap_or_default());

    let body = state.render("spool_list.html", &ctx).await?;
    Ok(Html(body))
}

pub async fn new_form(State(state): State<AppState>) -> Result<impl IntoResponse, AppError> {
    let brands: Vec<IdName> = sqlx::query_as("SELECT id, name FROM brands ORDER BY name")
        .fetch_all(&state.pool)
        .await?;
    let materials: Vec<IdName> = sqlx::query_as("SELECT id, name FROM materials ORDER BY name")
        .fetch_all(&state.pool)
        .await?;
    let colours: Vec<ColourOption> = sqlx::query_as("SELECT id, name, hex FROM colours ORDER BY name")
        .fetch_all(&state.pool)
        .await?;

    let mut ctx = Context::new();
    ctx.insert("brands", &brands);
    ctx.insert("materials", &materials);
    ctx.insert("colours", &colours);
    let body = state.render("spool_new.html", &ctx).await?;
    Ok(Html(body))
}

#[derive(Default)]
struct NewSpoolForm {
    brand_id: Option<i64>,
    material_id: Option<i64>,
    colour_id: Option<i64>,
    nominal_weight_g: Option<f64>,
    weight_mode: String,
    gross_weight_g: Option<f64>,
    product_url: Option<String>,
    notes: Option<String>,
    purchase_date: Option<String>,
    images: Vec<(String, Vec<u8>)>,
}

async fn parse_multipart(mut multipart: Multipart) -> Result<NewSpoolForm, AppError> {
    let mut form = NewSpoolForm::default();

    while let Some(field) = multipart.next_field().await? {
        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
            "images" => {
                let file_name = field.file_name().unwrap_or("").to_string();
                let bytes = field.bytes().await?;
                if !file_name.is_empty() && !bytes.is_empty() {
                    form.images.push((file_name, bytes.to_vec()));
                }
            }
            _ => {
                let text = field.text().await?;
                let text = text.trim().to_string();
                match name.as_str() {
                    "brand_id" => form.brand_id = text.parse().ok(),
                    "material_id" => form.material_id = text.parse().ok(),
                    "colour_id" => form.colour_id = text.parse().ok(),
                    "nominal_weight_g" => form.nominal_weight_g = text.parse().ok(),
                    "weight_mode" => form.weight_mode = text,
                    "gross_weight_g" => form.gross_weight_g = text.parse().ok(),
                    "product_url" if !text.is_empty() => form.product_url = Some(text),
                    "notes" if !text.is_empty() => form.notes = Some(text),
                    "purchase_date" if !text.is_empty() => form.purchase_date = Some(text),
                    _ => {}
                }
            }
        }
    }

    Ok(form)
}

pub async fn create(State(state): State<AppState>, multipart: Multipart) -> Result<impl IntoResponse, AppError> {
    let form = parse_multipart(multipart).await?;

    let brand_id = form.brand_id.ok_or_else(|| anyhow::anyhow!("brand is required"))?;
    let material_id = form.material_id.ok_or_else(|| anyhow::anyhow!("material is required"))?;
    let colour_id = form.colour_id.ok_or_else(|| anyhow::anyhow!("colour is required"))?;
    let nominal_weight_g = form.nominal_weight_g.unwrap_or(1000.0);

    let brand_tare: f64 = sqlx::query_scalar("SELECT tare_weight_g FROM brands WHERE id = ?")
        .bind(brand_id)
        .fetch_one(&state.pool)
        .await?;

    let gross_weight_g = if form.weight_mode == "opened" {
        form.gross_weight_g.ok_or_else(|| anyhow::anyhow!("gross weight is required when already opened"))?
    } else {
        brand_tare + nominal_weight_g
    };

    let mut tx = state.pool.begin().await?;

    let spool_id: i64 = sqlx::query_scalar(
        "INSERT INTO spools (brand_id, material_id, colour_id, nominal_weight_g, product_url, notes, purchase_date)
         VALUES (?, ?, ?, ?, ?, ?, ?) RETURNING id",
    )
    .bind(brand_id)
    .bind(material_id)
    .bind(colour_id)
    .bind(nominal_weight_g)
    .bind(&form.product_url)
    .bind(&form.notes)
    .bind(&form.purchase_date)
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query("INSERT INTO weight_log (spool_id, gross_weight_g, measured_at) VALUES (?, ?, ?)")
        .bind(spool_id)
        .bind(gross_weight_g)
        .bind(Utc::now().to_rfc3339())
        .execute(&mut *tx)
        .await?;

    for (sort_order, (file_name, bytes)) in form.images.iter().enumerate() {
        let ext = std::path::Path::new(file_name)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("bin");
        let stored_name = format!("{}.{}", uuid::Uuid::new_v4(), ext);
        let dest = state.image_dir.join(&stored_name);
        tokio::fs::write(&dest, bytes).await?;

        sqlx::query("INSERT INTO spool_images (spool_id, path, sort_order) VALUES (?, ?, ?)")
            .bind(spool_id)
            .bind(&stored_name)
            .bind(sort_order as i64)
            .execute(&mut *tx)
            .await?;
    }

    tx.commit().await?;

    Ok(Redirect::to("/spools"))
}

#[derive(Serialize, sqlx::FromRow)]
struct WeightHistoryEntry {
    id: i64,
    gross_weight_g: f64,
    measured_at: String,
    remaining_g: f64,
}

async fn fetch_spool(pool: &sqlx::SqlitePool, id: i64) -> Result<Option<SpoolWithDetails>, AppError> {
    let sql = format!("{SPOOL_LIST_BASE_QUERY} WHERE s.id = ?");
    let spool = sqlx::query_as(&sql).bind(id).fetch_optional(pool).await?;
    Ok(spool)
}

async fn fetch_detail_extras(pool: &sqlx::SqlitePool, id: i64) -> Result<(Vec<SpoolImage>, Vec<WeightHistoryEntry>), AppError> {
    let images: Vec<SpoolImage> = sqlx::query_as("SELECT * FROM spool_images WHERE spool_id = ? ORDER BY sort_order")
        .bind(id)
        .fetch_all(pool)
        .await?;

    let history: Vec<WeightHistoryEntry> = sqlx::query_as(
        "SELECT wl.id, wl.gross_weight_g, wl.measured_at,
                wl.gross_weight_g - COALESCE(s.tare_override_g, b.tare_weight_g) AS remaining_g
         FROM weight_log wl
         JOIN spools s ON s.id = wl.spool_id
         JOIN brands b ON b.id = s.brand_id
         WHERE wl.spool_id = ?
         ORDER BY wl.measured_at DESC, wl.id DESC",
    )
    .bind(id)
    .fetch_all(pool)
    .await?;

    Ok((images, history))
}

async fn render_detail_main(state: &AppState, id: i64) -> Result<String, AppError> {
    let spool = fetch_spool(&state.pool, id).await?;
    let (images, history) = fetch_detail_extras(&state.pool, id).await?;

    let mut ctx = Context::new();
    ctx.insert("spool", &spool);
    ctx.insert("images", &images);
    ctx.insert("history", &history);
    Ok(state.render("partials/spool_detail_main.html", &ctx).await?)
}

pub async fn detail(State(state): State<AppState>, Path(id): Path<i64>) -> Result<Response, AppError> {
    let Some(spool) = fetch_spool(&state.pool, id).await? else {
        return Ok((StatusCode::NOT_FOUND, "Spool not found").into_response());
    };
    let (images, history) = fetch_detail_extras(&state.pool, id).await?;

    let mut ctx = Context::new();
    ctx.insert("spool", &spool);
    ctx.insert("images", &images);
    ctx.insert("history", &history);
    let body = state.render("spool_detail.html", &ctx).await?;
    Ok(Html(body).into_response())
}

pub async fn weigh(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Form(form): Form<HashMap<String, String>>,
) -> Result<impl IntoResponse, AppError> {
    let gross_weight_g: f64 = form
        .get("gross_weight_g")
        .and_then(|v| v.trim().parse().ok())
        .ok_or_else(|| anyhow::anyhow!("gross weight is required"))?;

    sqlx::query("INSERT INTO weight_log (spool_id, gross_weight_g, measured_at) VALUES (?, ?, ?)")
        .bind(id)
        .bind(gross_weight_g)
        .bind(Utc::now().to_rfc3339())
        .execute(&state.pool)
        .await?;

    let body = render_detail_main(&state, id).await?;
    Ok(Html(body))
}
