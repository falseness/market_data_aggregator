use crate::common::*;


pub trait AgregatedL2Trait<Price> {
    fn set_quote(&mut self, price_: Price, new_amount: Amount);
    fn get_levels(&self) -> &BTreeMap<Price, Amount>;
    fn get_aggregated_levels(&self) -> &Vec<AggregatedLevel<Price>>;
    fn get_aggregated_levels_tuples(&self) -> Vec<(u64, u64)>;
}