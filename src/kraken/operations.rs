use super::dtos::{CurrentPrice, FutureOperation, Notify, OpenOrders, StopLossActive, Trades};
use super::helpers::{get_operation_type, get_order_type, OperationType, OrderType};
use super::notify::send_notification;
use crate::db::{DancespieleDB, Percentage};
use chrono::{TimeZone, Utc};
use coinnect::error::{Error, ErrorKind, Result};
use coinnect::kraken::{KrakenApi, KrakenCreds};
use std::collections::HashMap;

pub struct KrakenOpr {
    kraken_api: KrakenApi,
    percentages: Vec<Percentage>,
    trading_agreement: String,
}

impl KrakenOpr {
    pub fn new(cred: KrakenCreds, db_url: &str, trading_agreement: String) -> Self {
        let kraken_api = KrakenApi::new(cred).unwrap();
        let mut dancespiele_db = DancespieleDB::new(db_url);
        let percentages = dancespiele_db.fetch_coins_percentages_stop_loss().unwrap();

        Self {
            kraken_api,
            percentages,
            trading_agreement,
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
                    && current_balance
                        .get(&trade.pair.replace("EUR", ""))
                        .unwrap()
                        .parse::<f32>()
                        .unwrap()
                        > 0.00001
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

    fn add_stop_loss(
        &mut self,
        buy_price: FutureOperation,
        current_assest: CurrentPrice,
        benefit: String,
        order_opt: Option<String>,
    ) {
        let mut send_order = false;
        let mut stop_loss_price = 0.0;
        let percentage_to_stop_loss = self
            .percentages
            .clone()
            .into_iter()
            .find(|p| p.pair == current_assest.pair)
            .ok_or_else(|| eprintln!("pair does not exist in the database"))
            .unwrap();

        if let Some(order) = order_opt {
            if percentage_to_stop_loss
                .next_stop_loss
                .parse::<f32>()
                .unwrap()
                <= benefit.parse::<f32>().unwrap()
            {
                stop_loss_price = current_assest.price - (current_assest.price * 0.02);

                self.kraken_api.cancel_open_order(&order).unwrap();
                send_order = true;
            }
        } else if percentage_to_stop_loss
            .new_stop_loss
            .parse::<f32>()
            .unwrap()
            <= benefit.parse::<f32>().unwrap()
        {
            stop_loss_price = current_assest.price - (current_assest.price * 0.02);
            send_order = true;
        }

        if send_order {
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
                    &self.trading_agreement,
                )
                .unwrap();

            send_notification(Notify::from((
                current_assest.pair,
                stop_loss_price.to_string(),
                benefit,
            )));
        }
    }

    pub fn brain(&mut self) -> Result<String> {
        let buy_prices = self.get_buy_prices()?;
        let active_orders = self.get_active_orders()?.open;

        let percentages = self.percentages.clone();
        let current_prices: Vec<CurrentPrice> = buy_prices
            .clone()
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
            .filter(|c| percentages.clone().into_iter().any(|p| c.pair == p.pair))
            .collect();

        let stop_loses: Vec<StopLossActive> = active_orders
            .into_iter()
            .filter(|(_key, order)| {
                order.description.order_type == get_order_type(OrderType::StopLoss)
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

        current_prices.clone().into_iter().for_each(move |cp| {
            let buy_price_opt = buy_prices.clone().into_iter().find(|bp| bp.pair == cp.pair);

            let active_order_opt = stop_loses
                .clone()
                .into_iter()
                .find(|order| order.pair == cp.pair);

            if let Some(buy_price) = buy_price_opt {
                let benefit = self.calc_benefit(
                    if let Some(active_order) = active_order_opt.clone() {
                        active_order.price
                    } else {
                        buy_price.buy_price
                    },
                    cp.price,
                );

                self.add_stop_loss(
                    buy_price,
                    cp,
                    benefit,
                    if let Some(active_order) = active_order_opt {
                        Some(active_order.order)
                    } else {
                        None
                    },
                )
            } else {
                eprint!("Error or not found current assets");
            }
        });

        Ok(format!("Brain executed: \n{:#?}", current_prices))
    }
}

#[cfg(test)]
mod tests {
    use super::super::dtos::{
        CurrentPrice, Description, FutureOperation, OpenOrders, Order, StopLossActive,
    };
    use super::super::helpers::{get_operation_type, get_order_type, OperationType, OrderType};
    use crate::db::Percentage;
    use std::collections::HashMap;

    #[derive(Debug, PartialEq)]
    struct OrderSent {
        pair: String,
        type_order: String,
        ordertype: String,
        price: String,
        price2: String,
        volume: String,
        leverage: String,
        oflags: String,
        starttm: String,
        expiretm: String,
        userref: String,
        validate: String,
    }

    fn calc_benefit(price_ordered: f32, current_price: f32) -> String {
        let result = current_price - price_ordered;
        if result.is_sign_negative() {
            0.0.to_string()
        } else {
            (result / price_ordered * 100.0).to_string()
        }
    }

    fn cancel_open_order(txid: &str) {
        assert_eq!(txid, "3344de344");
    }

    #[allow(clippy::too_many_arguments)]
    fn add_standard_order(
        pair: &str,
        type_order: &str,
        ordertype: &str,
        price: &str,
        price2: &str,
        volume: &str,
        leverage: &str,
        oflags: &str,
        starttm: &str,
        expiretm: &str,
        userref: &str,
        validate: &str,
    ) -> OrderSent {
        OrderSent {
            pair: pair.to_string(),
            type_order: type_order.to_string(),
            ordertype: ordertype.to_string(),
            price: price.to_string(),
            price2: price2.to_string(),
            volume: volume.to_string(),
            leverage: leverage.to_string(),
            oflags: oflags.to_string(),
            starttm: starttm.to_string(),
            expiretm: expiretm.to_string(),
            userref: userref.to_string(),
            validate: validate.to_string(),
        }
    }

    fn get_buy_prices() -> Vec<FutureOperation> {
        vec![
            FutureOperation {
                pair: String::from("OXTEUR"),
                buy_price: 0.29,
                operation_time: 160000,
                quantity: String::from("4000"),
            },
            FutureOperation {
                pair: String::from("KAVAEUR"),
                buy_price: 2.0,
                operation_time: 160000,
                quantity: String::from("1500"),
            },
            FutureOperation {
                pair: String::from("CRVEUR"),
                buy_price: 5.0,
                operation_time: 160000,
                quantity: String::from("3000"),
            },
        ]
    }

    fn get_active_orders() -> OpenOrders {
        let mut order = HashMap::new();
        order.insert(
            String::from("3344de344"),
            Order {
                cost: String::from(""),
                fee: String::from("1.0"),
                limit_price: String::from(""),
                expiretm: 160000.0,
                misc: String::from(""),
                oflags: String::from(""),
                opentm: 160000.0,
                price: String::from("0.3"),
                refid: None,
                status: String::from("ACTIVE"),
                stop_price: String::from(""),
                description: Description {
                    close: String::from(""),
                    order: String::from("3344de344"),
                    operation_type: String::from("sell"),
                    order_type: String::from("stop-loss"),
                    leverage: String::from(""),
                    pair: String::from("KAVAEUR"),
                    price: String::from("0.3"),
                    price2: String::from(""),
                },
                userref: 2123444,
                vol: String::from("1500"),
                vol_exec: String::from(""),
            },
        );

        OpenOrders { open: order }
    }

    fn get_price(pair: &str) -> String {
        if pair == "KAVAEUR" {
            "3.5".to_string()
        } else {
            "0.40".to_string()
        }
    }

    fn add_stop_loss(
        buy_price: FutureOperation,
        current_assest: CurrentPrice,
        benefit: String,
        order_opt: Option<String>,
        percentages: Vec<Percentage>,
    ) {
        let percentage_to_stop_loss = percentages
            .into_iter()
            .find(|p| p.pair == current_assest.pair)
            .ok_or_else(|| eprintln!("pair does not exist in the database"))
            .unwrap();

        if let Some(order) = order_opt {
            if percentage_to_stop_loss
                .next_stop_loss
                .parse::<f32>()
                .unwrap()
                <= benefit.parse::<f32>().unwrap()
            {
                let stop_loss_price = current_assest.price - (current_assest.price * 0.02);
                cancel_open_order(&order);

                let order = add_standard_order(
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
                );

                assert_eq!(
                    order,
                    OrderSent {
                        pair: "KAVAEUR".to_string(),
                        type_order: "sell".to_string(),
                        ordertype: "stop-loss".to_string(),
                        price: "3.43".to_string(),
                        price2: "".to_string(),
                        volume: "1500".to_string(),
                        leverage: String::from(""),
                        oflags: String::from(""),
                        starttm: String::from(""),
                        expiretm: String::from(""),
                        userref: String::from(""),
                        validate: String::from(""),
                    }
                );
            }
        } else if percentage_to_stop_loss
            .new_stop_loss
            .parse::<f32>()
            .unwrap()
            <= benefit.parse::<f32>().unwrap()
        {
            let stop_loss_price = current_assest.price - (current_assest.price * 0.02);
            let order = add_standard_order(
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
            );

            assert_eq!(
                order,
                OrderSent {
                    pair: "OXTEUR".to_string(),
                    type_order: "sell".to_string(),
                    ordertype: "stop-loss".to_string(),
                    price: "0.39200002".to_string(),
                    price2: "".to_string(),
                    volume: "4000".to_string(),
                    leverage: String::from(""),
                    oflags: String::from(""),
                    starttm: String::from(""),
                    expiretm: String::from(""),
                    userref: String::from(""),
                    validate: String::from(""),
                }
            );
        }
    }

    fn brain() {
        let buy_prices = get_buy_prices();
        let active_orders = get_active_orders().open;
        let percentages = vec![
            Percentage {
                new_stop_loss: String::from("40.0"),
                next_stop_loss: String::from("14.0"),
                pair: String::from("KAVAEUR"),
            },
            Percentage {
                new_stop_loss: String::from("30.0"),
                next_stop_loss: String::from("5.0"),
                pair: String::from("OXTEUR"),
            },
        ];
        let current_prices: Vec<CurrentPrice> = buy_prices
            .clone()
            .into_iter()
            .map(|fo| {
                CurrentPrice::from((
                    fo.pair.clone(),
                    get_price(&fo.pair).parse::<f32>().unwrap_or_else(|err| {
                        println!("Error: {}", err);
                        0.0000
                    }),
                ))
            })
            .filter(|c| percentages.clone().into_iter().any(|p| c.pair == p.pair))
            .collect();

        let stop_loses: Vec<StopLossActive> = active_orders
            .into_iter()
            .filter(|(_key, order)| {
                order.description.order_type == get_order_type(OrderType::StopLoss)
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

        current_prices.into_iter().for_each(move |cp| {
            let buy_price_opt = buy_prices.clone().into_iter().find(|bp| bp.pair == cp.pair);

            let active_order_opt = stop_loses
                .clone()
                .into_iter()
                .find(|order| order.pair == cp.pair);

            if let Some(buy_price) = buy_price_opt {
                let benefit = calc_benefit(
                    if let Some(active_order) = active_order_opt.clone() {
                        active_order.price
                    } else {
                        buy_price.buy_price
                    },
                    cp.price,
                );

                add_stop_loss(
                    buy_price,
                    cp,
                    benefit,
                    if let Some(active_order) = active_order_opt {
                        Some(active_order.order)
                    } else {
                        None
                    },
                    percentages.clone(),
                )
            } else {
                eprint!("Error or not found current assets");
            }
        });
    }

    #[test]
    fn should_calculate_benefit() {
        let benefit = calc_benefit(0.29, 0.4);

        assert_eq!(benefit, "37.93104".to_string());
    }

    #[test]
    fn should_add_stop_loss() {
        let buy_price = FutureOperation {
            pair: String::from("OXTEUR"),
            buy_price: 0.29,
            operation_time: 160000,
            quantity: String::from("4000"),
        };

        let current_assest = CurrentPrice {
            pair: String::from("OXTEUR"),
            price: 0.4,
        };

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

        add_stop_loss(
            buy_price,
            current_assest,
            String::from("37.93104"),
            None,
            percentages,
        );
    }

    #[test]
    fn should_set_next_stop_loss() {
        let buy_price = FutureOperation {
            pair: String::from("KAVAEUR"),
            buy_price: 3.0,
            operation_time: 160000,
            quantity: String::from("1500"),
        };

        let current_assest = CurrentPrice {
            pair: String::from("KAVAEUR"),
            price: 3.5,
        };

        let percentages = vec![
            Percentage {
                new_stop_loss: String::from("40.0"),
                next_stop_loss: String::from("14.0"),
                pair: String::from("KAVAEUR"),
            },
            Percentage {
                new_stop_loss: String::from("30.0"),
                next_stop_loss: String::from("5.0"),
                pair: String::from("OXTEUR"),
            },
        ];

        add_stop_loss(
            buy_price,
            current_assest,
            String::from("16.66666"),
            Some(String::from("3344de344")),
            percentages,
        );
    }

    #[test]
    fn should_brain_play_in_kraken() {
        brain();
    }

    #[test]
    fn should_compare_numbers_string() {
        if "2.5".parse::<f32>().unwrap() < "2.6".parse::<f32>().unwrap() {
            println!("works 1");
        }
        if "3.43".parse::<f32>().unwrap() > "3.41".parse::<f32>().unwrap() {
            println!("works 2");
        }

        if "4.34".parse::<f32>().unwrap() > "10.32".parse::<f32>().unwrap() {
            println!("works 3");
        }

        if "34.443".parse::<f32>().unwrap() > "100.34".parse::<f32>().unwrap() {
            panic!("should not work");
        }
    }
}
