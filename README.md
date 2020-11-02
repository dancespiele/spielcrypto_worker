# Dancespiele

Dancespiele is an cron job task application that add stop loss order in exchange crytoconcurrency platforms.

## How it works

First you need to set the increment percent of your current coins price that you wish to put a stop loss using Dancespiele API.
For example imagine that you have `ETH` in [Kraken](https://www.kraken.com/) and its current price is `300 EUR` and you set in dancespiele api the parameter `new_stop_loss` an increment of `0.20` (20%) then `ETH` increase to `370 EUR` (more than 20%) in the future, the Dancespiele cron job will add automatically a stop loss with a price of `354 EUR` (always 2% less than the stop loss set) guaranteeing a benefit of `54 EUR`, now you set the paremeter `next_stop_loss` to `0.10` (10%) and `ETH` increase to `410` (more than 10%), the application will set a stop loss of `398,86 €` based in the increment from the previous stop loss and it will continue setting new stop loss each time that price increase more than 10%.