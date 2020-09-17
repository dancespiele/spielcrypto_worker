mod db;
mod kraken;

use coinnect::kraken::KrakenCreds;
use cronjob::CronJob;
use kraken::KrakenOpr;
use std::path::Path;

fn main() {
    // Create the `CronJob` object.
    let mut cron = CronJob::new("Test Kraken", on_cron);
    cron.start_job();
}

// Our cronjob handler.
fn on_cron(_name: &str) {
    // let cred =
    //     KrakenCreds::new_from_file("account_kraken", Path::new("keys.json").to_path_buf()).unwrap();

    // let mut kraken_opr = KrakenOpr::new(cred);

    // let kraken_open_order = kraken_opr.get_buy_prices();
    println!("Hello dancespiele");
}
