use super::dtos::{OpenOrders, Trades};
use coinnect::error::{Error, ErrorKind, Result};
use coinnect::kraken::{KrakenApi, KrakenCreds};
use serde_json::{self, Map, Value};
use std::collections::HashMap;

pub struct KrakenOpr {
    kraken_api: KrakenApi,
}

impl KrakenOpr {
    pub fn new(cred: KrakenCreds) -> Self {
        let kraken_api = KrakenApi::new(cred).unwrap();
        Self { kraken_api }
    }

    pub fn get_trades(&mut self) -> Result<Trades> {
        let trades_history = self.kraken_api.get_trades_history("", "", "", "", "0")?;
        let result_opt = trades_history.get("result");

        if let Some(result) = result_opt {
            let trades_string = result.to_string();
            let trades: Trades = serde_json::from_str(&trades_string)?;

            Ok(trades)
        } else {
            Err(Error::from_kind(ErrorKind::MissingField(
                "result".to_string(),
            )))
        }
    }

    pub fn get_current_balance(&mut self) -> Result<HashMap<String, String>> {
        let account_balance = self.kraken_api.get_account_balance()?;
        let result_opt = account_balance.get("result");

        if let Some(result) = result_opt {
            let account_string = result.to_string();

            let accout: HashMap<String, String> = serde_json::from_str(&account_string)?;

            Ok(accout)
        } else {
            Err(Error::from_kind(ErrorKind::MissingField(
                "result".to_string(),
            )))
        }
    }

    pub fn get_active_orders(&mut self) -> Result<OpenOrders> {
        let open_orders = self.kraken_api.get_open_orders("", "")?;
        let result_opt = open_orders.get("result");

        if let Some(result) = result_opt {
            let orders_string = result.to_string();
            let orders: OpenOrders = serde_json::from_str(&orders_string)?;

            Ok(orders)
        } else {
            Err(Error::from_kind(ErrorKind::MissingField(
                "result".to_string(),
            )))
        }
    }

    pub fn calc_profit(&mut self) -> Result<String> {
        let trades_active = self.get_trades()?.trades;
        let current_balance = self.get_current_balance();

        Ok("ok".to_string())
    }
}
