use std::sync::Arc;

use chrono::prelude::*;
use serde::{Deserialize, Serialize};
use sled::IVec;
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub enum TextItem {
    Code(String),
    ShortLink(String),
}

impl TextItem {
    pub fn get_data(self: &Self) -> String {
        match self {
            TextItem::Code(t) => {t.clone()}
            TextItem::ShortLink(t) => {t.clone()}
        }
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct DateBaseItem {
    sunset: Option<DateTime<Local>>,
    vanity_url: Option<String>,
    text: TextItem,
}

fn insert_when_not_exist<K: AsRef<[u8]>, V: Into<IVec>>(db: Arc<sled::Db>, key: K, value: V) -> Result<(), String> {
    let existed = db.get(&key).unwrap().is_some();
    if !existed {
        db.insert(&key, value).unwrap();
        return Ok(());
    }
    return Err(String::from("existed"));
}

pub fn add_record(db: Arc<sled::Db>, data: DateBaseItem) -> Result<(), String> {
    let hash = blake3::hash(data.text.get_data().as_bytes());
    let short = &hash.to_hex()[0..4];
    if let Some(special_url) = data.vanity_url {
        return insert_when_not_exist(db, &special_url.as_str(), short);
    }
    insert_when_not_exist(db, short, bincode::serialize(&data).unwrap())
}

pub fn delete_record(db: Arc<sled::Db>, key: String) -> Result<(), String> {

}
