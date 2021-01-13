use blake3::Hash;
use chrono::prelude::*;
use serde::{Deserialize, Serialize};
use sled::IVec;
use std::ops::Deref;
#[derive(Debug, Hash, PartialEq, Eq)]
pub enum TreeNames {
    DataTree,
    ShortNameTree,
    CustomNameTree,
}

impl AsRef<[u8]> for TreeNames {
    fn as_ref(&self) -> &[u8] {
        match self {
            TreeNames::DataTree => &[0],
            TreeNames::ShortNameTree => &[1],
            TreeNames::CustomNameTree => &[2],
        }
    }
}
#[derive(Debug, Clone)]
pub struct DataTrees {
    pub db: sled::Tree,
    pub short_to_long_db: sled::Tree,
    pub custom_to_long_db: sled::Tree,
}

impl DataTrees {
    pub fn new(database: sled::Db) -> Self {
        DataTrees {
            db: database.open_tree(TreeNames::DataTree).unwrap(),
            short_to_long_db: database.open_tree(TreeNames::ShortNameTree).unwrap(),
            custom_to_long_db: database.open_tree(TreeNames::CustomNameTree).unwrap(),
        }
    }
}

enum DataBaseErrorType {
    Existed,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub enum TextItem {
    Code(String),
    ShortLink(String),
}

impl TextItem {
    pub fn get_data(self: &Self) -> String {
        match self {
            TextItem::Code(t) => t.clone(),
            TextItem::ShortLink(t) => t.clone(),
        }
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct DateBaseItem {
    destroy_time: Option<DateTime<Utc>>,
    custom_url: Option<String>,
    hash: String,
    short: String,
    text: TextItem,
}

impl DateBaseItem {
    pub fn new(
        text: TextItem,
        custom_url: Option<String>,
        destroy_time: Option<DateTime<Utc>>,
    ) -> DateBaseItem {
        let hash = blake3::hash(text.get_data().as_bytes());
        let short = String::from(&hash.to_hex()[0..4]);
        DateBaseItem {
            destroy_time,
            custom_url,
            text,
            short,
            hash: String::from(hash.to_hex().as_str()),
        }
    }
}

fn insert_when_not_exist<K: AsRef<[u8]>, V: Into<IVec>>(
    db: sled::Tree,
    key: K,
    value: V,
) -> Result<(), DataBaseErrorType> {
    let res = db.compare_and_swap(key, None as Option<&[u8]>, Some(value));
    if let Err(_) = res {
        return Err(DataBaseErrorType::Existed);
    }
    return Ok(());
}

pub fn add_record(db: DataTrees, data: DateBaseItem) -> Result<(), DataBaseErrorType> {
    insert_when_not_exist(db.db, data.hash, bincode::serialize(&data).unwrap())?;
    insert_when_not_exist(db.short_to_long_db, data.short, data.hash.as_bytes())?;

    if let Some(special_url) = data.custom_url {
        insert_when_not_exist(
            db.custom_to_long_db,
            &special_url.as_str(),
            data.hash.as_bytes(),
        )?;
    }
    return Ok(());
}

pub fn delete_record(db: sled::Db, key: String) -> Result<(), DataBaseErrorType> {}
