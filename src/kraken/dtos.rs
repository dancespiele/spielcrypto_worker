use serde::{
    de::{value::SeqDeserializer, Error, Visitor},
    Deserialize, Serialize,
};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Trade {
    pub cost: String,
    pub fee: String,
    pub margin: String,
    pub misc: String,
    pub ordertxid: String,
    pub ordertype: String,
    pub pair: String,
    pub postxid: String,
    pub price: String,
    pub time: f64,
    #[serde(rename = "type")]
    pub trade_type: String,
    pub vol: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Trades {
    pub count: u32,
    pub trades: HashMap<String, Trade>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Description {
    pub close: String,
    pub leverage: String,
    pub order: String,
    #[serde(rename = "ordertype")]
    pub order_type: String,
    pub pair: String,
    pub price: String,
    pub price2: String,
    #[serde(rename = "type")]
    pub operation_type: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Order {
    pub cost: String,
    #[serde(rename = "descr")]
    pub description: Description,
    pub expiretm: f64,
    pub fee: String,
    #[serde(rename = "limitprice")]
    pub limit_price: String,
    pub misc: String,
    pub oflags: String,
    pub opentm: f64,
    pub price: String,
    pub refid: Option<String>,
    pub status: String,
    #[serde(rename = "stopprice")]
    pub stop_price: String,
    pub userref: u32,
    pub vol: String,
    pub vol_exec: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OpenOrders {
    pub open: HashMap<String, Order>,
}

#[derive(Clone, Debug)]
pub struct FutureOperation {
    pub buy_price: f32,
    pub pair: String,
    pub quantity: String,
    pub operation_time: i64,
}

impl From<(Trade, String)> for FutureOperation {
    fn from(future_operation: (Trade, String)) -> Self {
        let (trade, quantity) = future_operation;

        Self {
            buy_price: trade.price.parse().unwrap(),
            pair: trade.pair,
            quantity,
            operation_time: trade.time as i64,
        }
    }
}

#[derive(Clone, Debug)]
pub struct CurrentPrice {
    pub pair: String,
    pub price: f32,
}

impl From<(String, f32)> for CurrentPrice {
    fn from(current_price: (String, f32)) -> Self {
        let (pair, price) = current_price;

        Self { pair, price }
    }
}

#[derive(Clone, Debug)]
pub struct StopLossActive {
    pub order: String,
    pub price: f32,
    pub pair: String,
    pub current_price: f32,
}

impl From<(String, f32, CurrentPrice)> for StopLossActive {
    fn from(stop_loss_active: (String, f32, CurrentPrice)) -> Self {
        let (order, price, current_price) = stop_loss_active;

        Self {
            order,
            price,
            pair: current_price.pair,
            current_price: current_price.price,
        }
    }
}
