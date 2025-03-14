pub use crate::common::*;
pub use crate::solutions::aggregated_l2_trait::*;
pub use crate::subscription::*;

use std::collections::BTreeMap;
use std::collections::btree_map::Entry;
use std::ops::Bound;

pub struct AggregatedL2<Price: OrderKey> {
    levels: BTreeMap<Price, Amount>,
    max_depth_price: Price,
    aggregated_levels: Vec<AggregatedLevel<Price>>,
    subscription_rules: SubscriptionRules,
}

impl<Price: OrderKey> AggregatedL2<Price>
where
    u64: From<Price>,
    Price: From<u64>,
{
    fn does_level_have_surplus(
        self: &Self,
        index: usize,
        cursor_to_last_element: &std::collections::btree_map::Cursor<Price, Amount>,
    ) -> bool {
        return self.aggregated_levels[index].total_amount
            - cursor_to_last_element.peek_next().unwrap().1
            >= self.subscription_rules.get_amount(index);
    }

    fn try_propogate_amount_surplus(self: &mut Self, index: usize) {
        // invariant: уже добавили всё тут в levels
        let mut cursor = self
            .levels
            .lower_bound(Bound::Included(&self.aggregated_levels[index].last_price));
        debug_assert!(*cursor.peek_next().unwrap().0 == self.aggregated_levels[index].last_price);

        if !self.does_level_have_surplus(index, &cursor) {
            return;
        }
        loop {
            let (&price, &amount) = cursor.peek_next().unwrap();
            // вот ето можно будет удалить, если всё делать в правильном порядке
            if self.aggregated_levels[index].last_price <= self.max_depth_price {
                if index + 1 == self.aggregated_levels.len() {
                    self.aggregated_levels.push(AggregatedLevel {
                        last_price: price,
                        total_amount: amount,
                    });
                } else {
                    self.aggregated_levels[index + 1].total_amount += amount;
                }
            }
            self.aggregated_levels[index].total_amount -= amount;
            cursor.prev();
            self.aggregated_levels[index].last_price = *cursor.peek_next().unwrap().0;
            if !self.does_level_have_surplus(index, &cursor) {
                break;
            }
        }
        self.try_propogate_amount_surplus(index + 1);
    }
    fn try_cut_by_max_depth(self: &mut Self) {
        // invariant: only 1 element difference and max_depth_price is actual
        let last_level = self.aggregated_levels.last_mut().unwrap();

        if last_level.last_price <= self.max_depth_price {
            return;
        }
        let cursor = self
            .levels
            .lower_bound(Bound::Included(&last_level.last_price));
        let current_amount = *cursor.peek_next().unwrap().1;
        if let Some((&previous_price, _)) = cursor.peek_prev() {
            last_level.total_amount -= current_amount;
            last_level.last_price = previous_price;
            if last_level.total_amount == 0 {
                self.aggregated_levels.pop();
            }
        } else {
            debug_assert!(last_level.total_amount == current_amount);
            self.aggregated_levels.pop();
        }
    }
    fn try_update_max_depth_price(self: &mut Self) {
        // invariant: element wath added to the left of self.max_depth_price
        let has_cut_by_depth = self.max_depth_price != Price::MAX;
        if !has_cut_by_depth {
            debug_assert!(self.subscription_rules.max_depth != 0);
            if self.levels.len() == self.subscription_rules.max_depth {
                self.max_depth_price = *self.levels.last_key_value().unwrap().0;
            }
            return;
        }
        let cursor = self
            .levels
            .lower_bound(Bound::Included(&self.max_depth_price));
        debug_assert!(*cursor.peek_next().unwrap().0 == self.max_depth_price);
        self.max_depth_price = *cursor.peek_prev().unwrap().0;
    }

    fn try_update_max_depth_price_remove_quote(self: &mut Self) {
        // invariant: element wath removed to the left of self.max_depth_price
        let has_cut_by_depth = self.max_depth_price != Price::MAX;
        if !has_cut_by_depth {
            return;
        }
        let cursor = self
            .levels
            .lower_bound(Bound::Excluded(&self.max_depth_price));
        if let Some((&price, &amount)) = cursor.peek_next() {
            self.max_depth_price = price;
            let index = self.aggregated_levels.len() - 1;
            if self.aggregated_levels[index].total_amount
                < self.subscription_rules.get_amount(index)
            {
                self.aggregated_levels[index].last_price = price;
                self.aggregated_levels[index].total_amount += amount;
            } else {
                self.aggregated_levels.push(AggregatedLevel {
                    last_price: price,
                    total_amount: amount,
                });
            }
        } else {
            self.max_depth_price = Price::MAX;
        }
    }
    fn add_quote_not_found_in_aggregated_levels(self: &mut Self, price: Price, amount: Amount, is_price_new: bool, mut index: usize) {
        if index == self.aggregated_levels.len() {
            if price > self.max_depth_price {
                return;
            }
            debug_assert!(self.max_depth_price == Price::MAX);

            if self.levels.len() == self.subscription_rules.max_depth {
                debug_assert!(is_price_new);
                self.max_depth_price = price;
            }
            index -= 1;
            self.aggregated_levels[index].last_price = price;
            self.aggregated_levels[index].total_amount += amount;
        } else {
            self.aggregated_levels[index].total_amount += amount;
            if is_price_new {
                debug_assert!(price < self.max_depth_price);
                self.try_update_max_depth_price();
                self.try_cut_by_max_depth();
            }
        }

        self.try_propogate_amount_surplus(index);
    }
    fn add_quote(self: &mut Self, price: Price, amount: Amount, is_price_new: bool) {
        if self.levels.len() == 1 && is_price_new {
            self.levels.insert(price, amount);
            self.aggregated_levels.push(AggregatedLevel {
                last_price: price,
                total_amount: amount,
            });
            if self.subscription_rules.max_depth == 1 {
                self.max_depth_price = price;
            }
            return;
        }

        match self
            .aggregated_levels
            .binary_search_by(|level| level.last_price.cmp(&price))
        {
            Ok(index) => {
                self.aggregated_levels[index].total_amount += amount;
                return;
            }
            Err(index) => {
                self.add_quote_not_found_in_aggregated_levels(price, amount, is_price_new, index);
            }
        }
    }
    fn try_propogate_shortage(self: &mut Self, mut index: usize) {
        // There may be levels with total_amount == 0 after the method execution
        if self.aggregated_levels[index].total_amount >= self.subscription_rules.get_amount(index) {
            return;
        }
        let mut cursor = self
            .levels
            .lower_bound(Bound::Excluded(&self.aggregated_levels[index].last_price));

        let mut index_to_steal_quotes = index + 1;
        while let Some((&price, &amount)) = cursor.next() {
            if price > self.max_depth_price {
                return;
            }
            self.aggregated_levels[index].last_price = price;
            self.aggregated_levels[index].total_amount += amount;
            if index_to_steal_quotes < self.aggregated_levels.len() {
                self.aggregated_levels[index_to_steal_quotes].total_amount -= amount;
                if self.aggregated_levels[index_to_steal_quotes].total_amount == 0 {
                    index_to_steal_quotes += 1;
                }
            }
            if self.aggregated_levels[index].total_amount
                < self.subscription_rules.get_amount(index)
            {
                continue;
            }
            if index + 1 >= self.aggregated_levels.len() {
                return;
            }
            if self.aggregated_levels[index + 1].total_amount != 0 {
                self.try_propogate_shortage(index + 1);
                return;
            }
            debug_assert!(index_to_steal_quotes > index + 1);
            index += 1;
        }
    }
    fn remove_last_quote_in_level(self: &mut Self, price: Price, amount: Amount, has_removed_quote: bool, index: usize) {
        debug_assert!(self.aggregated_levels[index].last_price == price);
        self.aggregated_levels[index].total_amount -= amount;

        self.try_propogate_shortage(index);
        if !has_removed_quote || self.aggregated_levels[index].last_price != price {
            while !self.aggregated_levels.is_empty()
                && self.aggregated_levels.last().unwrap().total_amount == 0
            {
                self.aggregated_levels.pop();
            }
            return;
        }

        if self.aggregated_levels[index].total_amount == 0 {
            debug_assert!(index + 1 == self.aggregated_levels.len());
            self.aggregated_levels.pop();
            return;
        }
        // last_price was not updated by element to the right. so update it by previous element
        let cursor = self
            .levels
            .lower_bound(Bound::Included(&self.aggregated_levels[index].last_price));
        debug_assert!(
            cursor.peek_next().is_none() ||
            *cursor.peek_next().unwrap().0 != self.aggregated_levels[index].last_price
        );

        let (&price, _) = cursor.peek_prev().unwrap();
        self.aggregated_levels[index].last_price = price;
    }
    fn remove_quote(self: &mut Self, price: Price, amount: Amount, has_removed_quote: bool) {
        if has_removed_quote && price <= self.max_depth_price {
            self.try_update_max_depth_price_remove_quote()
        }
        match self
            .aggregated_levels
            .binary_search_by(|level| level.last_price.cmp(&price))
        {
            Ok(index) => {
                self.remove_last_quote_in_level(price, amount, has_removed_quote, index);
            }
            Err(index) => {
                if index == self.aggregated_levels.len() {
                    return;
                }
                self.aggregated_levels[index].total_amount -= amount;
                self.try_propogate_shortage(index);
                while !self.aggregated_levels.is_empty()
                    && self.aggregated_levels.last().unwrap().total_amount == 0
                {
                    self.aggregated_levels.pop();
                }
            }
        };
    }
    pub fn get_max_depth_price(&self) -> Price {
        return self.max_depth_price;
    }
}

impl<Price: OrderKey> AgregatedL2Trait<Price> for AggregatedL2<Price>
where
    u64: From<Price>,
    Price: From<u64>,
{
    fn new(table: SubscriptionRules) -> Self {
        Self {
            levels: BTreeMap::new(),
            max_depth_price: Price::MAX,
            aggregated_levels: Vec::new(),
            subscription_rules: table,
        }
    }
    fn set_quote(self: &mut Self, price_: u64, new_amount: Amount) {
        let price = Price::from(price_);

        match self.levels.entry(price) {
            Entry::Vacant(entry) => {
                if new_amount != 0 {
                    entry.insert(new_amount);
                    self.add_quote(price, new_amount, /*is_price_new=*/true)
                }
            },
            Entry::Occupied(mut entry) => {
                let current_amount = *entry.get();
                *entry.get_mut() = new_amount;

                match new_amount.cmp(&current_amount) {
                    std::cmp::Ordering::Greater => self.add_quote(price, new_amount - current_amount, /*is_price_new=*/false),
                    std::cmp::Ordering::Less => {
                        let should_remove_quote = new_amount == 0;
                        if should_remove_quote {
                            entry.remove();
                        }
                        self.remove_quote(price, current_amount - new_amount, should_remove_quote);
                    },
                    std::cmp::Ordering::Equal => (),
                }
            }
        };
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
