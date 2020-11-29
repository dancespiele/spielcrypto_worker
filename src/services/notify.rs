use crate::kraken::dtos::{Notify, NotifyEmail};
use celery::TaskResult;
use std::env;

#[celery::task]
fn add_stop_loss(notify: NotifyEmail) -> TaskResult<NotifyEmail> {
    Ok(notify)
}

pub async fn send_notification(notify: Notify) {
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

    println!("Email task with id {} sent", task_id)
}

#[cfg(test)]
mod tests {
    use super::send_notification;
    use crate::kraken::dtos::Notify;
    use agnostik::prelude::*;
    use dotenv::dotenv;

    #[test]
    fn should_send_notification() {
        dotenv().ok();

        let notify = Notify {
            pair: "KAVAEUR".to_string(),
            price: "4.0".to_string(),
            benefit: "40.0".to_string(),
        };

        let runtime = Agnostik::tokio();

        let notification = runtime.spawn(async move {
            send_notification(notify).await;
        });

        runtime.block_on(notification);
    }
}
