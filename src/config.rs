use std::env;

#[derive(Clone, Debug)]
pub struct Config {
    pub database_url: String,
    pub image_dir: String,
    pub port: u16,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            database_url: env::var("DATABASE_URL").expect("DATABASE_URL must be set"),
            image_dir: env::var("IMAGE_DIR").expect("IMAGE_DIR must be set"),
            port: env::var("PORT")
                .unwrap_or_else(|_| "8080".to_string())
                .parse()
                .expect("PORT must be a valid u16"),
        }
    }
}
