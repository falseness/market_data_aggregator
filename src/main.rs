#![feature(btree_cursors)]
#![feature(map_try_insert)]

pub use market_data_aggregator::measure_time::*;

fn main() {
    measure_time_for_both_solutions()
}
