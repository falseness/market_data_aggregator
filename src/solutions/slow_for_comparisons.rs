use crate::common::*;
use crate::solutions::aggregated_l2_trait::AgregatedL2Trait;
use crate::subscription::*;

use std::collections::BTreeMap;

pub struct SlowAggregatedL2ForComparisons<Price: OrderKey> {
    levels: BTreeMap<Price, Amount>,
    max_depth_price: Price,
    aggregated_levels: Vec<AggregatedLevel<Price>>,
    subscription_rules: SubscriptionRules,
}

impl<Price: OrderKey> SlowAggregatedL2ForComparisons<Price>
where
    u64: From<Price>,
    Price: From<u64>,
{
    pub fn get_max_depth_price(&self) -> Price {
        return self.max_depth_price;
    }
}

impl<Price: OrderKey> AgregatedL2Trait<Price> for SlowAggregatedL2ForComparisons<Price>
where
    u64: From<Price>,
    Price: From<u64>,
{
    fn new(subscription_rules: SubscriptionRules) -> Self {
        Self {
            levels: BTreeMap::new(),
            aggregated_levels: Vec::new(),
            subscription_rules: subscription_rules,
            max_depth_price: Price::MAX,
        }
    }
    fn set_quote(self: &mut Self, price_: u64, new_amount: Amount) {
        let price = Price::from(price_);
        match self.levels.try_insert(price, new_amount) {
            Ok(_) => {
                if new_amount == 0 {
                    self.levels.remove(&price);
                }
            }
            Err(entry) => {
                if new_amount == 0 {
                    entry.entry.remove();
                } else {
                    *entry.entry.into_mut() = new_amount;
                }
            }
        };
        self.aggregated_levels.clear();
        for (quote_index, (&price, &amount)) in self.levels.iter().enumerate() {
            debug_assert!(amount > 0);
            if quote_index + 1 > self.subscription_rules.max_depth {
                break;
            }
            self.max_depth_price = price;
            if self.aggregated_levels.is_empty() {
                self.aggregated_levels.push(AggregatedLevel {
                    last_price: price,
                    total_amount: amount,
                });
                continue;
            }
            let index = self.aggregated_levels.len() - 1;
            if self.aggregated_levels[index].total_amount
                >= self.subscription_rules.get_amount(index)
            {
                self.aggregated_levels.push(AggregatedLevel {
                    last_price: price,
                    total_amount: amount,
                });
            } else {
                self.aggregated_levels[index].last_price = price;
                self.aggregated_levels[index].total_amount += amount;
            }
        }
        if self.levels.len() < self.subscription_rules.max_depth {
            self.max_depth_price = Price::MAX;
        }
    }
    fn get_levels(&self) -> &BTreeMap<Price, Amount> {
        return &self.levels;
    }
    fn get_aggregated_levels(&self) -> &Vec<AggregatedLevel<Price>> {
        return &self.aggregated_levels;
    }
    fn get_aggregated_levels_tuples(&self) -> Vec<(u64, u64)> {
        let result_clone = self.aggregated_levels.clone();
        result_clone
            .into_iter()
            .map(|level| (level.last_price.into(), level.total_amount))
            .collect()
    }
}
