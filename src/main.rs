mod db;
mod kraken;

use coinnect::kraken::KrakenCreds;
use cronjob::CronJob;
use dotenv::dotenv;
use kraken::KrakenOpr;
use std::env;
use std::path::Path;

fn main() {
    dotenv().ok();
    // Create the `CronJob` object.
    let mut cron = CronJob::new("Dancespiele", on_cron);
    cron.minutes("/2");
    cron.start_job();
}

// Our cronjob handler.
fn on_cron(_name: &str) {
    let sled_url = env::var("SLED_URL").expect("SLED_URL must be set");

    let cred =
        KrakenCreds::new_from_file("account_kraken", Path::new("keys.json").to_path_buf()).unwrap();

    let mut kraken_opr = KrakenOpr::new(cred, &sled_url);

    let result = kraken_opr.brain().unwrap_or_else(|err| err.to_string());
    println!("{}", result);
}
