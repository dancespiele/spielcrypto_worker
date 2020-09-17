use super::dtos::{CurrentPrice, FutureOperation, OpenOrders, StopLossActive, Trades};
use super::helpers::{get_operation_type, get_order_type, OperationType, OrderType};
use crate::db::{DancespieleDB, Percentage};
use chrono::{TimeZone, Utc};
use coinnect::error::{Error, ErrorKind, Result};
use coinnect::kraken::{KrakenApi, KrakenCreds};
use serde_json::{self, Map, Value};
use std::collections::HashMap;

pub struct KrakenOpr {
    kraken_api: KrakenApi,
    percentages: Vec<Percentage>,
}

impl KrakenOpr {
    pub fn new(cred: KrakenCreds, db_url: &str) -> Self {
        let kraken_api = KrakenApi::new(cred).unwrap();
        let mut dancespiele_db = DancespieleDB::new(db_url);
        let percentages = dancespiele_db.fetch_coins_percentages_stop_loss().unwrap();

        Self {
            kraken_api,
            percentages,
        }
    }

    fn get_trades(&mut self) -> Result<Trades> {
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

    fn get_current_balance(&mut self) -> Result<HashMap<String, String>> {
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

    fn get_active_orders(&mut self) -> Result<OpenOrders> {
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

    fn get_buy_prices(&mut self) -> Result<Vec<FutureOperation>> {
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

    fn get_price(&mut self, pair: &str) -> Result<String> {
        let price_result = self.kraken_api.get_ohlc_data(pair, "", "")?;
        let result = price_result
            .get("result")
            .ok_or_else(|| Error::from_kind(ErrorKind::MissingField("result".to_string())))?;

        let ohlcs = result
            .as_object()
            .ok_or_else(|| Error::from_kind(ErrorKind::BadParse))?;

        let ohlcs_pair = ohlcs
            .get(pair)
            .ok_or_else(|| Error::from_kind(ErrorKind::MissingField(pair.to_string())))?;

        let prices = ohlcs_pair
            .as_array()
            .ok_or_else(|| Error::from_kind(ErrorKind::BadParse))?;
        let last_price = prices
            .last()
            .ok_or_else(|| Error::from_kind(ErrorKind::MissingField("last array".to_string())))?;

        let price_close_value = last_price
            .get(4)
            .ok_or_else(|| Error::from_kind(ErrorKind::MissingField("4".to_string())))?;

        let price_close = price_close_value
            .as_str()
            .ok_or_else(|| Error::from_kind(ErrorKind::BadParse))?;

        Ok(price_close.to_string())
    }

    fn calc_benefit(&mut self, price_ordered: f32, current_price: f32) -> String {
        let result = current_price - price_ordered;

        if result.is_sign_negative() {
            0.0.to_string()
        } else {
            (result / price_ordered * 100.0).to_string()
        }
    }

    fn set_stop_loss(
        &mut self,
        buy_price: FutureOperation,
        current_assest: CurrentPrice,
        benefit: String,
    ) {
        let percentage_to_stop_loss = self
            .percentages
            .clone()
            .into_iter()
            .find(|p| p.pair == current_assest.pair)
            .ok_or_else(|| eprintln!("pair does not exist in the database"))
            .unwrap();

        if percentage_to_stop_loss.new_stop_loss <= benefit {
            let stop_loss_price = current_assest.price - (current_assest.price * 0.02);

            self.kraken_api
                .add_standard_order(
                    &current_assest.pair,
                    &get_operation_type(OperationType::SELL),
                    &get_order_type(OrderType::StopLoss),
                    &stop_loss_price.to_string(),
                    "",
                    &buy_price.quantity,
                    "",
                    "",
                    "",
                    "",
                    "",
                    "",
                )
                .unwrap();
        }
    }

    pub fn brain(&mut self) -> Result<String> {
        let buy_prices = self.get_buy_prices()?;
        let active_orders = self.get_active_orders()?.open;
        let current_prices: Vec<CurrentPrice> = buy_prices
            .into_iter()
            .map(|fo| {
                CurrentPrice::from((
                    fo.pair.clone(),
                    self.get_price(&fo.pair)
                        .unwrap_or_else(|err| {
                            println!("Error: {}", err);
                            "0.0000".to_string()
                        })
                        .parse::<f32>()
                        .unwrap_or_else(|err| {
                            println!("Error: {}", err);
                            0.0000
                        }),
                ))
            })
            .collect();

        let stop_loses: Vec<StopLossActive> = active_orders
            .into_iter()
            .filter(|(_key, order)| {
                order.description.operation_type == get_order_type(OrderType::StopLoss)
                    && order.description.operation_type == get_operation_type(OperationType::SELL)
            })
            .map(|(key, order)| {
                StopLossActive::from((
                    key,
                    order.price.parse().unwrap_or_else(|err| {
                        println!("Error: {}", err);
                        0.0000
                    }),
                    current_prices
                        .clone()
                        .into_iter()
                        .find(|cp| cp.pair == order.description.pair)
                        .unwrap(),
                ))
            })
            .collect();

        if stop_loses.is_empty() {}

        Ok("Ok".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::KrakenOpr;
    use coinnect::kraken::KrakenCreds;
    use std::path::Path;

    #[test]
    fn should_get_price() {
        let creds =
            KrakenCreds::new_from_file("account_kraken", Path::new("keys.json").to_path_buf())
                .unwrap();
        let mut kraken_opr = KrakenOpr::new(creds);

        let price = kraken_opr.get_price("KAVAEUR").unwrap();
        println!("price: {:#?}", price);
    }
}
