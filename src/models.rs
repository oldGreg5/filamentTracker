use serde::Serialize;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct SpoolImage {
    pub id: i64,
    pub spool_id: i64,
    pub path: String,
    pub sort_order: i64,
}

/// Spool joined with brand/material/colour names and computed remaining weight,
/// used for the list and detail views.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct SpoolWithDetails {
    pub id: i64,
    pub brand_name: String,
    pub material_name: String,
    pub colour_name: String,
    pub colour_hex: Option<String>,
    pub nominal_weight_g: f64,
    pub product_url: Option<String>,
    pub notes: Option<String>,
    pub purchase_date: Option<String>,
    pub remaining_g: Option<f64>,
    pub percent_remaining: Option<f64>,
    pub thumbnail_path: Option<String>,
}
