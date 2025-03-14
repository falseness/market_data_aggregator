use crate::common::*;
use crate::subscription::*;

use std::collections::BTreeMap;

pub trait AgregatedL2Trait<Price: OrderKey> {
    fn new(subscription: SubscriptionRules) -> Self;
    fn set_quote(&mut self, price_: u64, new_amount: Amount);
    fn get_levels(&self) -> &BTreeMap<Price, Amount>;
    fn get_aggregated_levels(&self) -> &Vec<AggregatedLevel<Price>>;
    fn get_aggregated_levels_tuples(&self) -> Vec<(u64, u64)>;
}
