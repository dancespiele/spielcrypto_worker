use super::dtos::{Notify, NotifyEmail};
use crate::db::DancespieleDB;
use celery::TaskResult;
use std::env;

#[celery::task]
fn add_stop_loss(notify: Notify) -> TaskResult<NotifyEmail> {
    let email = env::var("EMAIL").expect("Email should be set");

    Ok(NotifyEmail::from((notify, email)))
}

pub async fn send_notification(notify: Notify, db_url: String) {
    let notification = celery::app!(
        broker = AMQP { std::env::var("AMPQ_ADDR").unwrap_or_else(|_| "amqp://127.0.0.1:5672".into())},
        tasks = [
            add_stop_loss,
        ],
        task_routes = [
            "add_stop_loss" => "stop_loss_queue",
    ]);

    let task_id = notification
        .send_task(add_stop_loss::new(notify))
        .await
        .unwrap();

    let notification_response = DancespieleDB::find_task_id(task_id, &db_url)
        .unwrap_or_else(|_| String::from("Error to send notification"));
    println!("Notification: {}", notification_response);
}

#[cfg(test)]
mod tests {
    use super::super::dtos::Notify;
    use super::send_notification;
    use dotenv::dotenv;
    use std::env;

    #[test]
    fn should_send_notification() {
        dotenv().ok();
        let sled_url = env::var("SLED_URL_TEST").expect("SLED_URL_TEST must be set");

        let notify = Notify {
            pair: "KAVAEUR".to_string(),
            price: "4.0".to_string(),
            benefit: "40.0".to_string(),
        };

        send_notification(notify, sled_url);
    }
}
