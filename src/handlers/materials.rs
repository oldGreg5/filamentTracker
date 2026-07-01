use std::collections::HashMap;

use axum::extract::{Path, State};
use axum::response::IntoResponse;
use axum::Form;

use crate::error::AppError;
use crate::reference::{self, ColumnDef, ReferenceConfig};
use crate::state::AppState;

static COLUMNS: &[ColumnDef] = &[
    ColumnDef { key: "name", label: "Name", input_type: "text", required: true },
    ColumnDef { key: "notes", label: "Notes", input_type: "text", required: false },
];

static CONFIG: ReferenceConfig = ReferenceConfig {
    entity: "materials",
    table: "materials",
    singular: "Material",
    title: "Materials",
    columns: COLUMNS,
    spool_fk_column: "material_id",
};

pub async fn page(state: State<AppState>) -> Result<impl IntoResponse, AppError> {
    reference::page(state.0, &CONFIG).await
}

pub async fn create(state: State<AppState>, form: Form<HashMap<String, String>>) -> Result<impl IntoResponse, AppError> {
    reference::create(state.0, &CONFIG, form.0).await
}

pub async fn update(state: State<AppState>, Path(id): Path<i64>, form: Form<HashMap<String, String>>) -> Result<impl IntoResponse, AppError> {
    reference::update(state.0, &CONFIG, id, form.0).await
}

pub async fn delete(state: State<AppState>, Path(id): Path<i64>) -> Result<impl IntoResponse, AppError> {
    reference::delete(state.0, &CONFIG, id).await
}

pub async fn edit(state: State<AppState>, Path(id): Path<i64>) -> Result<impl IntoResponse, AppError> {
    reference::edit_fragment(state.0, &CONFIG, id).await
}

pub async fn view(state: State<AppState>, Path(id): Path<i64>) -> Result<impl IntoResponse, AppError> {
    reference::view_fragment(state.0, &CONFIG, id).await
}
