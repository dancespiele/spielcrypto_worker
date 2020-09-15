pub enum OperationType {
    BUY,
    SELL,
}

pub fn get_operation_type(operation_type: OperationType) -> String {
    match operation_type {
        OperationType::BUY => String::from("buy"),
        OperationType::SELL => String::from("sell"),
    }
}
