use serde::{Deserialize, Serialize};
use sled::{Db, Error, IVec, Result};
use std::str;

pub struct DancespieleDB {
    db: Db,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
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

#[cfg(test)]
mod tests {
    use super::{DancespieleDB, Percentage};

    #[test]
    fn should_fetch_coins_percentages_stop_loss() {
        let percentages = vec![
            Percentage {
                new_stop_loss: String::from("15.0"),
                next_stop_loss: String::from("5.0"),
                pair: String::from("KAVAEUR"),
            },
            Percentage {
                new_stop_loss: String::from("30.0"),
                next_stop_loss: String::from("5.0"),
                pair: String::from("OXTEUR"),
            },
        ];

        let percentages_string = serde_json::to_string(&percentages).unwrap();

        let mut dancespiele_db = DancespieleDB::new("test_sled");

        dancespiele_db
            .db
            .insert("percentages", percentages_string.as_bytes())
            .unwrap();

        let percentages_saved = dancespiele_db.fetch_coins_percentages_stop_loss().unwrap();

        dancespiele_db.db.remove("percentages").unwrap();

        assert_eq!(
            serde_json::to_string(&percentages_saved).unwrap(),
            serde_json::to_string(&percentages).unwrap()
        );
    }
}
