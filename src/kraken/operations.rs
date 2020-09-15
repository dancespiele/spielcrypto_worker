use super::dtos::{FutureOperation, OpenOrders, Trades};
use super::helpers::{get_operation_type, OperationType};
use chrono::{TimeZone, Utc};
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

    pub fn get_buy_prices(&mut self) -> Result<Vec<FutureOperation>> {
        let trades_active = self.get_trades()?.trades;
        let current_balance = self.get_current_balance()?;

        let trades_to_operate: Vec<FutureOperation> = trades_active
            .into_iter()
            .filter(|(_key, trade)| {
                current_balance
                    .get(&trade.pair.replace("EUR", ""))
                    .is_some()
                    && trade.trade_type == get_operation_type(OperationType::BUY)
            })
            .map(|(_key, trade)| {
                let quantity = current_balance
                    .get(&trade.pair.replace("EUR", ""))
                    .unwrap()
                    .to_string();
                FutureOperation::from((trade, quantity))
            })
            .fold(&mut vec![], |acc: &mut Vec<FutureOperation>, curr| {
                let acc_copy = acc.to_vec();
                let prev_future_operation_option: Option<FutureOperation> = acc_copy
                    .into_iter()
                    .find(|future_operation| future_operation.pair == curr.pair);

                if let Some(prev_future_operation) = prev_future_operation_option {
                    if Utc
                        .timestamp(prev_future_operation.operation_time, 0)
                        .le(&Utc.timestamp(curr.operation_time, 0))
                    {
                        let prev_index = acc
                            .iter_mut()
                            .position(|future_operation| future_operation.pair == curr.pair)
                            .unwrap();
                        acc[prev_index] = curr;
                        acc
                    } else {
                        acc
                    }
                } else {
                    acc.push(curr);
                    acc
                }
            })
            .clone();

        Ok(trades_to_operate.to_vec())
    }
}
