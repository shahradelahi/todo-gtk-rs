use crate::config::APP_ID;
use std::path::PathBuf;

pub fn data_path() -> PathBuf {
    let mut path = glib::user_data_dir();
    path.push(APP_ID);
    std::fs::create_dir_all(&path).expect("Failed to create data directory");
    path.push("data.json");
    path
}
