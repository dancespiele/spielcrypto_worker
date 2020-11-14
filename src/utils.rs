pub fn substract_pair(pair: &str) -> String {
    let mut current_currency = String::from("");

    let current_currencies = vec![
        "XBT", "ETH", "USDT", "XRP", "LINK", "BCH", "LTC", "DOT", "ADA", "USDC", "EOS", "XMR",
        "TRX", "XLM", "XTZ", "ATOM", "DAI", "FIL", "UNI", "DASH", "ZEC", "ETC", "YFI", "OMG",
        "COMP", "SNX", "WAVES", "XDG", "KSM", "ALGO", "BAT", "ICX", "QTUM", "KNC", "REP", "REPV2",
        "LSK", "SC", "NANO", "BAL", "OXT", "CRV", "PAXG", "STORJ", "KAVA", "GNO", "MLN",
    ];

    current_currencies.into_iter().for_each(|c| {
        if pair.starts_with(c) {
            current_currency = c.to_string();
        }
    });

    current_currency
}

#[test]
fn should_substract_the_currency_from_pair() {
    let currencies_to_change = vec![
        "EUR", "USD", "XBT", "ETH", "CAD", "JPY", "CHF", "GBP", "USDT", "USDC", "DAI", "AUD",
    ];
    let current_currencies = vec![
        "XBT", "ETH", "USDT", "XRP", "LINK", "BCH", "LTC", "DOT", "ADA", "USDC", "EOS", "XMR",
        "TRX", "XLM", "XTZ", "ATOM", "DAI", "FIL", "UNI", "DASH", "ZEC", "ETC", "YFI", "OMG",
        "COMP", "SNX", "WAVES", "XDG", "KSM", "ALGO", "BAT", "ICX", "QTUM", "KNC", "REP", "REPV2",
        "LSK", "SC", "NANO", "BAL", "OXT", "CRV", "PAXG", "STORJ", "KAVA", "GNO", "MLN",
    ];

    current_currencies.into_iter().for_each(|cc| {
        currencies_to_change.clone().into_iter().for_each(|ctc| {
            let mut cc_copy = cc.to_string();
            cc_copy.push_str(ctc);

            let currency = substract_pair(&cc_copy);

            assert_eq!(currency, cc);
        })
    });
}
