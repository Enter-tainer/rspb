use std::str::from_utf8;

use chrono::prelude::*;
use serde::{Deserialize, Serialize};
use sled::transaction;
use sled::Transactional;
use transaction::{ConflictableTransactionError, TransactionalTree};
use uuid::Uuid;

use crate::base32;
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
    pub short_to_uuid_db: sled::Tree,
    pub custom_to_uuid_db: sled::Tree,
}

impl DataTrees {
    pub fn new(database: sled::Db) -> Self {
        DataTrees {
            db: database.open_tree(TreeNames::DataTree).unwrap(),
            short_to_uuid_db: database.open_tree(TreeNames::ShortNameTree).unwrap(),
            custom_to_uuid_db: database.open_tree(TreeNames::CustomNameTree).unwrap(),
        }
    }
}

#[derive(Debug)]
pub enum DataBaseErrorType {
    Existed(DataBaseItem),
    Failed,
    NotFound,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub enum DataType {
    Text(String),
    ShortLink(String),
    Binary(Vec<u8>),
}

impl DataType {
    pub fn get_data(self: &Self) -> &[u8] {
        match self {
            DataType::Text(t) => t.as_bytes(),
            DataType::ShortLink(t) => t.as_bytes(),
            DataType::Binary(t) => t,
        }
    }

    pub fn from_bytes(data: Vec<u8>, is_short_link: Option<bool>) -> Option<DataType> {
        let short_link = is_short_link.unwrap_or(false);
        let d = String::from_utf8(data.clone());
        match d {
            Ok(str) => {
                if short_link {
                    return Some(DataType::ShortLink(String::from(str.trim_end())));
                } else {
                    return Some(DataType::Text(str));
                }
            }
            Err(_) => {
                if short_link {
                    return None;
                } else {
                    return Some(DataType::Binary(data));
                }
            }
        }
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct DataBaseItem {
    pub destroy_time: Option<DateTime<Utc>>,
    pub custom_url: Option<String>,
    pub uuid: Uuid,
    pub hash: String,
    pub short: String,
    pub data: DataType,
}

impl DataBaseItem {
    pub fn new(
        data: DataType,
        custom_url: Option<String>,
        destroy_time: Option<DateTime<Utc>>,
    ) -> DataBaseItem {
        let hash = blake3::hash(data.get_data());
        let short = &base32::encode(hash.as_bytes())[0..4];
        DataBaseItem {
            destroy_time,
            custom_url,
            data: data,
            short: String::from(short),
            hash: String::from(hash.to_hex().as_str()),
            uuid: Uuid::new_v4(),
        }
    }
}

// fn insert_when_not_exist_cas<K: AsRef<[u8]>, V: Into<IVec>>(
//     db: sled::Tree,
//     key: K,
//     value: V,
// ) -> Result<(), DataBaseErrorType> {
//     let res = db.compare_and_swap(key, None as Option<&[u8]>, Some(value));
//     if let Err(_) = res {
//         return Err(DataBaseErrorType::Failed);
//     }
//     return Ok(());
// }

// fn insert_when_not_exist_transaction<K: AsRef<[u8]> + Into<IVec>, V: Into<IVec>>(
//     db: &transaction::TransactionalTree,
//     key: K,
//     value: V,
// ) -> Result<(), ConflictableTransactionError> {
//     if db.get(&key).unwrap().is_none() {
//         if db.insert(key, value).is_ok() {
//             return Ok(());
//         } else {
//             return Err(ConflictableTransactionError::Conflict);
//         };
//     }
//     return Err(ConflictableTransactionError::Conflict);
// }

pub fn add_record(db: DataTrees, data: &DataBaseItem) -> Result<(), DataBaseErrorType> {
    if search_key_in_db(db.clone(), data.uuid.as_bytes()).is_ok() {
        return Err(DataBaseErrorType::Existed(
            get_data_in_db(db.clone(), data.uuid.as_bytes()).unwrap(),
        ));
    } else if search_key_in_db(db.clone(), data.short.as_bytes()).is_ok() {
        return Err(DataBaseErrorType::Existed(
            get_data_in_db(db.clone(), data.short.as_bytes()).unwrap(),
        ));
    } else {
        if let Some(str) = &data.custom_url {
            if search_key_in_db(db.clone(), str.as_bytes()).is_ok() {
                return Err(DataBaseErrorType::Existed(
                    get_data_in_db(db, str.as_bytes()).unwrap(),
                ));
            }
        }
    }
    let res = (&db.db, &db.short_to_uuid_db, &db.custom_to_uuid_db).transaction(
        |(db, short_to_long_db, custom_to_long_db): &(
            TransactionalTree,
            TransactionalTree,
            TransactionalTree,
        )|
         -> Result<(), ConflictableTransactionError> {
            db.insert(data.uuid.as_bytes(), bincode::serialize(&data).unwrap())?;
            short_to_long_db.insert(data.short.as_bytes(), data.uuid.as_bytes())?;
            if let Some(special_url) = &data.custom_url {
                custom_to_long_db.insert(special_url.as_bytes(), data.uuid.as_bytes())?;
            }
            Ok(())
        },
    );
    if res.is_err() {
        return Err(DataBaseErrorType::Failed);
    }
    return Ok(());
}

fn search_key_in_db(db: DataTrees, key: &[u8]) -> Result<TreeNames, DataBaseErrorType> {
    // short -> custom -> uuid
    if db.short_to_uuid_db.contains_key(key).unwrap_or(false) {
        return Ok(TreeNames::ShortNameTree);
    }
    if db.custom_to_uuid_db.contains_key(key).unwrap_or(false) {
        return Ok(TreeNames::CustomNameTree);
    }
    if let Ok(str) = from_utf8(key) {
        if let Ok(id) = uuid::Uuid::parse_str(str) {
            if db.db.contains_key(id.as_bytes()).unwrap_or(false) {
                return Ok(TreeNames::DataTree);
            }
        }
    }
    return Err(DataBaseErrorType::NotFound);
}

fn get_data_in_db(db: DataTrees, key: &[u8]) -> Result<DataBaseItem, DataBaseErrorType> {
    let res = search_key_in_db(db.clone(), key)?;
    let data: DataBaseItem;
    match res {
        TreeNames::DataTree => {
            data = bincode::deserialize::<DataBaseItem>(
                &db.db
                    .get(Uuid::parse_str(from_utf8(key).unwrap()).unwrap().as_bytes())
                    .unwrap()
                    .unwrap(),
            )
            .unwrap();
        }
        TreeNames::ShortNameTree => {
            let key = db.short_to_uuid_db.get(key).unwrap().unwrap();
            data = bincode::deserialize::<DataBaseItem>(&db.db.get(key).unwrap().unwrap()).unwrap();
        }
        TreeNames::CustomNameTree => {
            let key = db.custom_to_uuid_db.get(key).unwrap().unwrap();
            data = bincode::deserialize::<DataBaseItem>(&db.db.get(key).unwrap().unwrap()).unwrap();
        }
    }
    Ok(data)
}

pub fn delete_record(db: DataTrees, key: Uuid) -> Result<(), DataBaseErrorType> {
    let data = db.db.get(key.as_bytes()).unwrap();
    if data.is_none() {
        return Err(DataBaseErrorType::NotFound);
    }
    let data = bincode::deserialize::<DataBaseItem>(&data.unwrap()).unwrap();
    let res = (&db.db, &db.short_to_uuid_db, &db.custom_to_uuid_db).transaction(
        |(db, short_to_long_db, custom_to_long_db): &(
            TransactionalTree,
            TransactionalTree,
            TransactionalTree,
        )|
         -> Result<(), ConflictableTransactionError> {
            db.remove(key.as_bytes())?;
            short_to_long_db.remove(data.short.as_bytes())?;
            if let Some(url) = &data.custom_url {
                custom_to_long_db.remove(url.as_bytes())?;
            }
            Ok(())
        },
    );
    if res.is_err() {
        return Err(DataBaseErrorType::NotFound);
    }
    Ok(())
}

pub fn query_record(db: DataTrees, key: String) -> Result<DataBaseItem, DataBaseErrorType> {
    get_data_in_db(db, key.as_bytes())
}

pub fn update_record(db: DataTrees, key: Uuid, value: DataType) -> Result<(), DataBaseErrorType> {
    let mut data = get_data_in_db(db.clone(), key.to_string().as_bytes())?;
    data.data = value;
    data.hash = String::from(blake3::hash(data.data.get_data()).to_hex().as_str());
    let res = db
        .db
        .transaction(|db| -> Result<(), ConflictableTransactionError> {
            db.insert(key.as_bytes(), bincode::serialize(&data).unwrap())?;
            Ok(())
        });
    if res.is_err() {
        return Err(DataBaseErrorType::Failed);
    }
    Ok(())
}
