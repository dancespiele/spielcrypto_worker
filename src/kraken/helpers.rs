pub enum OperationType {
    BUY,
    SELL,
}

pub enum OrderType {
    Limit,
    StopLoss,
}

pub fn get_operation_type(operation_type: OperationType) -> String {
    match operation_type {
        OperationType::BUY => String::from("buy"),
        OperationType::SELL => String::from("sell"),
    }
}

pub fn get_order_type(order_type: OrderType) -> String {
    match order_type {
        OrderType::Limit => String::from("limit"),
        OrderType::StopLoss => String::from("stop-loss"),
    }
}
