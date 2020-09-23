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

    let multiples = get_multiples(2);
    // Create the `CronJob` object.
    let mut cron = CronJob::new("Dancespiele", on_cron);
    cron.minutes(&multiples);
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

fn get_multiples(mult: i32) -> String {
    let mut multiples: Vec<String> = vec![];
    for n in (0..60).filter(|r| r % mult == 0).collect::<Vec<i32>>() {
        let value: String = n.to_string();
        multiples.push(value);
    }

    multiples.join(",")
}
