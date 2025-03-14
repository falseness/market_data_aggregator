
use crate::common::*;
use crate::subscription::*;
use crate::fast_solution::*;
use crate::slow_solution_for_comparison::*;
use crate::aggregated_l2_trait::*;

use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufRead, BufReader};
use serde_json::Result;

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")] // Ensure consistent casing if needed
enum Side {
    Bid,
    Ask,
}


#[derive(Debug, Deserialize, Serialize)]
struct Trade {
    platform_time: u64,
    exchange_time: u64,
    seq_no: Option<u64>,  // Nullable sequence number
    side: Side,         // "Bid" or "Ask"
    price: f64,
    amount: f64,
    is_eot: bool,
}

fn is_integer(num: f64) -> bool {
    (num.round() - num).abs() < 1e-5
}


use std::time::Instant;


fn measure_time<SolutionAsk: AgregatedL2Trait<AskKey>, SolutionBid: AgregatedL2Trait<BidKey>>(arr: &Vec::<Trade>, table: &AggregationTable) {
    let ratio: f64 = 1e8;

    let start = Instant::now();
    for i in 0..40000 {
        let mut solution_for_ask = SolutionAsk::new(table.clone());
        let mut solution_for_bid = SolutionBid::new(table.clone());
        for trade in arr.iter() {
            let price = (trade.price * ratio).round() as u64;
            let amount = (trade.amount * ratio).round() as u64;
            assert!(is_integer(trade.price * ratio));
            assert!(is_integer(trade.amount * ratio));


            match trade.side {
                Side::Bid => solution_for_bid.set_quote(price, amount),
                Side::Ask => solution_for_ask.set_quote(price, amount),
            }
            if solution_for_bid.get_levels().is_empty() || solution_for_ask.get_levels().is_empty() {
                continue;
            } 
            let ask = u64::from(*solution_for_ask.get_levels().first_key_value().unwrap().0);
            let bid = u64::from(*solution_for_bid.get_levels().first_key_value().unwrap().0);
            assert!(ask > bid);
        }
    }
    let duration = start.elapsed();
    println!("Time taken: {:.2?}", duration);
}

pub fn measure_time_for_both_solutions() {
    let file = File::open("l2.json").expect("Cannot open file");
    let reader = BufReader::new(file);

    let table = AggregationTable::new(vec![5e13 as u64, 2e14 as u64, 3e13 as u64, 4e12 as u64], 2e13 as u64, 300);

    let mut arr = Vec::<Trade>::new();

    for line in reader.lines() {
        let line = line.expect("Error reading line");
        let trade: Trade = serde_json::from_str(&line).expect("Invalid JSON format");
        arr.push(trade);
    }

    println!("Fast solution: ");
    measure_time::<AggregatedL2<AskKey>, AggregatedL2<BidKey>>(&arr, &table);

    println!("\nSlow obvious solution:");
    measure_time::<SlowAggregatedL2ForTests<AskKey>, SlowAggregatedL2ForTests<BidKey>>(&arr, &table);
}