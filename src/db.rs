use serde::Deserialize;
use sled::{Db, Error, IVec, Result};
use std::str;

pub struct DancespieleDB {
    db: Db,
}

#[derive(Deserialize, Clone)]
pub struct Percentage {
    pub pair: String,
    pub new_stop_loss: String,
    pub next_stop_loss: String,
}

impl DancespieleDB {
    pub fn new(url: &str) -> Self {
        Self {
            db: sled::open(url).unwrap(),
        }
    }

    pub fn fetch_coins_percentages_stop_loss(&mut self) -> Result<Vec<Percentage>> {
        let percentages = self
            .db
            .get("percentages")?
            .ok_or_else(|| Error::CollectionNotFound(IVec::from("percentages")))?;

        let percentages_string = str::from_utf8(&percentages).unwrap();

        let response: Vec<Percentage> = serde_json::from_str(percentages_string).unwrap();

        Ok(response)
    }
}
