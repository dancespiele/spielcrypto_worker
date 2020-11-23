use super::dtos::{Notify, NotifyEmail};
use crate::db::DancespieleDB;
use celery::TaskResult;
use std::env;
use tokio::time::{delay_for, Duration};

#[celery::task]
fn add_stop_loss(notify: NotifyEmail) -> TaskResult<NotifyEmail> {
    Ok(notify)
}

pub async fn send_notification(notify: Notify, db_url: String) {
    let email = env::var("EMAIL").expect("Email should be set");

    let notify_email = NotifyEmail::from((notify, email));

    let amrq_addr = std::env::var("AMPQ_ADDR").unwrap_or_else(|_| "amqp://127.0.0.1:5672".into());
    let notification = celery::app!(
        broker = AMQP { amrq_addr },
        tasks = [
            add_stop_loss,
        ],
        task_routes = [
            "add_stop_loss" => "stop_loss_queue",
    ]);

    let task_id = notification
        .send_task(add_stop_loss::new(notify_email))
        .await
        .unwrap();
    
    delay_for(Duration::from_millis(1000)).await;

    let notification_response = DancespieleDB::find_task_id(task_id, &db_url)
        .await
        .unwrap_or_else(|_| String::from("Error to send notification"));
    println!("Notification: {}", notification_response);
}

#[cfg(test)]
mod tests {
    use super::super::dtos::Notify;
    use super::send_notification;
    use agnostik::prelude::*;
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

        let runtime = Agnostik::tokio();

        let notification = runtime.spawn(async move {
            send_notification(notify, sled_url).await;
        });

        runtime.block_on(notification);
    }
}
