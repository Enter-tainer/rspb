use chrono::prelude::*;
use serde::{Deserialize, Serialize};
use sled::transaction;
use sled::IVec;
use sled::Transactional;
use transaction::{ConflictableTransactionError, TransactionalTree};
use uuid::Uuid;
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

pub enum DataBaseErrorType {
    Existed,
    NotFound,
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
pub struct DataBaseItem {
    pub destroy_time: Option<DateTime<Utc>>,
    pub custom_url: Option<String>,
    pub uuid: Uuid,
    pub hash: String,
    pub short: String,
    pub text: TextItem,
}

impl DataBaseItem {
    pub fn new(
        text: TextItem,
        custom_url: Option<String>,
        destroy_time: Option<DateTime<Utc>>,
    ) -> DataBaseItem {
        let hash = blake3::hash(text.get_data().as_bytes());
        let short = String::from(&hash.to_hex()[0..4]);
        DataBaseItem {
            destroy_time,
            custom_url,
            text,
            short,
            hash: String::from(hash.to_hex().as_str()),
            uuid: Uuid::new_v4(),
        }
    }
}

fn insert_when_not_exist_cas<K: AsRef<[u8]>, V: Into<IVec>>(
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

fn insert_when_not_exist_transaction<K: AsRef<[u8]> + Into<IVec>, V: Into<IVec>>(
    db: &transaction::TransactionalTree,
    key: K,
    value: V,
) -> Result<(), ConflictableTransactionError> {
    if db.get(&key).unwrap().is_none() {
        if db.insert(key, value).is_ok() {
            return Ok(());
        } else {
            return Err(ConflictableTransactionError::Conflict);
        };
    }
    return Err(ConflictableTransactionError::Conflict);
}

pub fn add_record(db: DataTrees, data: &DataBaseItem) -> Result<(), DataBaseErrorType> {
    let res = (&db.db, &db.short_to_uuid_db, &db.custom_to_uuid_db).transaction(
        |(db, short_to_long_db, custom_to_long_db): &(
            TransactionalTree,
            TransactionalTree,
            TransactionalTree,
        )|
         -> Result<(), ConflictableTransactionError> {
            insert_when_not_exist_transaction(
                db,
                data.uuid.as_bytes(),
                bincode::serialize(&data).unwrap(),
            )?;
            insert_when_not_exist_transaction(
                short_to_long_db,
                data.short.as_bytes(),
                data.uuid.as_bytes(),
            )?;
            if let Some(special_url) = &data.custom_url {
                insert_when_not_exist_transaction(
                    custom_to_long_db,
                    special_url.as_bytes(),
                    data.uuid.as_bytes(),
                )?;
            }
            Ok(())
        },
    );
    if res.is_err() {
        return Err(DataBaseErrorType::Existed);
    }
    return Ok(());
}

fn search_key_in_db(db: DataTrees, key: &[u8]) -> Result<TreeNames, DataBaseErrorType> {
    // uuid -> short -> custom
    if db.db.contains_key(key).unwrap_or(false) {
        return Ok(TreeNames::DataTree);
    }
    if db.short_to_uuid_db.contains_key(key).unwrap_or(false) {
        return Ok(TreeNames::ShortNameTree);
    }
    if db.custom_to_uuid_db.contains_key(key).unwrap_or(false) {
        return Ok(TreeNames::CustomNameTree);
    }
    return Err(DataBaseErrorType::NotFound);
}

fn get_data_in_db(db: DataTrees, key: &[u8]) -> Result<DataBaseItem, DataBaseErrorType> {
    let res = search_key_in_db(db.clone(), key)?;
    let data: DataBaseItem;
    match res {
        TreeNames::DataTree => {
            data = bincode::deserialize::<DataBaseItem>(&db.db.get(key).unwrap().unwrap()).unwrap();
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

pub fn delete_record(db: DataTrees, key: String) -> Result<(), DataBaseErrorType> {
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

// fn update_record(db: DataTrees, key: String, value: DataBaseItem) -> Result<(), DataBaseErrorType> {
//     let data = get_data_in_db(db.clone(), key.as_bytes())?;
//     let res = (&db.db, &db.short_to_uuid_db, &db.custom_to_uuid_db).transaction(
//         |(db, short_to_long_db, custom_to_long_db): &(
//             TransactionalTree,
//             TransactionalTree,
//             TransactionalTree,
//         )|
//          -> Result<(), ConflictableTransactionError> {
//             db.insert(key.as_bytes(), bincode::serialize(&value).unwrap())?;
//             short_to_long_db.insert(data.short.as_bytes(), value.short.as_bytes())?;
//             if let Some(url) = &data.custom_url {
//                 custom_to_long_db.insert(url.as_bytes(), va)?;
//             }
//             Ok(())
//         },
//     );
//     if res.is_err() {
//         return Err(DataBaseErrorType::NotFound);
//     }
//     Ok(())
// }
