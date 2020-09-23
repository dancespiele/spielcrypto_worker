use super::dtos::{Claims, Notify};
use chrono::prelude::*;
use chrono::Duration;
use curl::easy::{Easy, List};
use jsonwebtoken::{encode, EncodingKey, Header};
use std::env;
use std::io::Read;

pub fn send_notification(notify: Notify) {
    let api_url = env::var("API_URL").expect("API_URL must be set");
    let token = create_system_token();
    let body = serde_json::to_string(&notify).unwrap();
    let mut data = body.as_bytes();
    let mut list = List::new();
    list.append("Content-Type: application/json").unwrap();
    list.append(&format!("Authorization: {}", token)).unwrap();

    let mut easy = Easy::new();
    easy.post(true).unwrap();
    easy.url(&format!("{}/mail/", api_url)).unwrap();
    easy.post_fields_copy(data).unwrap();
    easy.http_headers(list).unwrap();
    let mut transfer = easy.transfer();
    transfer
        .read_function(|buf| Ok(data.read(buf).unwrap_or(0)))
        .unwrap();
    transfer.perform().unwrap();
}

fn create_system_token() -> String {
    let email = env::var("EMAIL").expect("EMAIL must be set");
    let secret = env::var("SECRET").expect("SECRET must be set");
    let expire_token: DateTime<Utc> = Utc::now() + Duration::days(1);

    let claims = Claims {
        sub: String::from("SYSTEM"),
        iss: String::from("system"),
        email,
        iat: Utc::now().timestamp(),
        exp: expire_token.timestamp(),
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_ref()),
    )
    .unwrap();

    token
}

#[cfg(test)]
mod tests {
    use super::super::dtos::Notify;
    use super::send_notification;
    use dotenv::dotenv;

    #[test]
    fn should_send_notification() {
        dotenv().ok();
        let notify = Notify {
            pair: "KAVAEUR".to_string(),
            price: "4.0".to_string(),
            benefit: "40.0".to_string(),
        };

        send_notification(notify);
    }
}
