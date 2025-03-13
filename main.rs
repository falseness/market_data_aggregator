#![feature(btree_cursors)]
#![feature(map_try_insert)]

use std::borrow::Borrow;
use std::collections::BTreeMap;
use std::cmp::Ordering;
use std::convert::From;
use std::ops::Bound;


/// Generic trait for order keys (BidKey and AskKey)
trait OrderKey: Ord + Eq + Copy + std::fmt::Debug + From<u64> {
    const MAX: Self;
}
/*
impl From<u64> for AskKey {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

impl From<u64> for BidKey {
    fn from(value: u64) -> Self {
        Self(value)
    }
}*/

impl From<AskKey> for u64 {
    fn from(order_key: AskKey) -> Self {
        order_key.0
    }
}


impl From<BidKey> for u64 {
    fn from(order_key: BidKey) -> Self {
        order_key.0
    }
}


/// Bid key (sorted descending)
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
struct BidKey(u64);
impl OrderKey for BidKey {
    const MAX: Self = Self(0); 
}
impl Ord for BidKey {
    fn cmp(&self, other: &Self) -> Ordering {
        other.0.cmp(&self.0) // Reverse order for highest bid first
    }
}
impl PartialOrd for BidKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl From<u64> for BidKey {
    fn from(price: u64) -> Self {
        BidKey(price)
    }
}

/// Ask key (sorted ascending)
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
struct AskKey(u64);
impl OrderKey for AskKey {
    const MAX: Self = Self(u64::MAX);
}
impl Ord for AskKey {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0) // Normal order for lowest ask first
    }
}
impl From<u64> for AskKey {
    fn from(price: u64) -> Self {
        AskKey(price)
    }
}

impl PartialOrd for AskKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}


type Amount = u64;
/*
struct AggregatedLevel<Price: OrderKey> {
    Price last_price;
    Amount total_amount;
    BTreeMap<Price, Amount> contained_levels;
};

impl AggregatedLevel<Price: OrderKey> {
    fn new(Price price, Amount amount) -> Self {
        let mut map = BTreeMap::new();
        map.insert(price, amount);
        return Self {
            price, amount, map
        }
    }
}*/

#[derive(Debug, PartialEq, Clone)]
struct AggregatedLevel<Price: OrderKey> {
    last_price: Price,
    total_amount: Amount
}

impl<Price: OrderKey> AggregatedLevel<Price> {
    fn new(last_price: Price, total_amount: Amount) -> Self {
        return Self{
            last_price, total_amount
        }
    }
}


#[derive(Clone)]
struct AggregationTable {
    minimum_amounts: Vec<Amount>,
    fallback: Amount,
    max_depth: usize
}

impl AggregationTable {
    fn get_amount(self: &Self, index: usize) -> Amount {
        if index >= self.minimum_amounts.len() {
            return self.fallback;
        }
        return self.minimum_amounts[index];
    }
    fn new(minimum_amounts: Vec<Amount>,
        fallback: Amount,
        max_depth: usize) -> Self {

        assert!(minimum_amounts.iter().all(|&x| x > 0));
        assert!(fallback > 0);
        assert!(max_depth > 0);
        
        return Self {
            minimum_amounts,
            fallback,
            max_depth
        }
    }
}

struct AggregatedL2<Price: OrderKey> {
    levels: BTreeMap<Price, Amount>,
    max_depth_price: Price,
    aggregated_levels: Vec<AggregatedLevel<Price>>,
    aggregation_table: AggregationTable
}

// проверь книгу на пустоту
impl<Price: OrderKey> AggregatedL2<Price> 
where u64: From<Price>, Price: From<u64> {
    fn does_level_have_surplus(self: &Self, 
        index: usize, cursor_to_last_element: &std::collections::btree_map::Cursor<Price, Amount>) -> bool {
        return self.aggregated_levels[index].total_amount - cursor_to_last_element.peek_next().unwrap().1 >= self.aggregation_table.get_amount(index);
    }

    fn try_propogate_amount_surplus(
        self: &mut Self, 
        index: usize
    ) {
        // invariant: уже добавили всё тут в levels
        let mut cursor = self.levels.lower_bound(Bound::Included(&self.aggregated_levels[index].last_price));
        debug_assert!(*cursor.peek_next().unwrap().0 == self.aggregated_levels[index].last_price);
        
        if !self.does_level_have_surplus(index, &cursor) {
            return;
        }
        while true {
            let (&price, &amount) = cursor.peek_next().unwrap();
            // вот ето можно будет удалить, если всё делать в правильном порядке
            if self.aggregated_levels[index].last_price <= self.max_depth_price {
                if index + 1 == self.aggregated_levels.len() {
                    self.aggregated_levels.push(AggregatedLevel::new(price, amount));
                }
                else {
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
        let cursor = self.levels.lower_bound(Bound::Included(&last_level.last_price));
        let current_amount = *cursor.peek_next().unwrap().1;
        if let Some((&previous_price, _)) = cursor.peek_prev() {
            last_level.total_amount -= current_amount;
            last_level.last_price = previous_price;
            if last_level.total_amount == 0 {
                self.aggregated_levels.pop();
            }
        }
        else {
            debug_assert!(last_level.total_amount == current_amount);
            self.aggregated_levels.pop();
        }
    }
    fn try_update_max_depth_price(self: &mut Self) {
        // invariant: был добавлен элемент СЛЕВА от self.max_depth_price 
        let has_cut_by_depth = self.max_depth_price != Price::MAX;
        if !has_cut_by_depth {
            // предполагаем, что max_depth != 0
            if self.levels.len() == self.aggregation_table.max_depth {
                self.max_depth_price = *self.levels.last_key_value().unwrap().0;
            }
            return;
        }
        let cursor = self.levels.lower_bound(Bound::Included(&self.max_depth_price));
        debug_assert!(*cursor.peek_next().unwrap().0 == self.max_depth_price);
        self.max_depth_price = *cursor.peek_prev().unwrap().0;
    }

    fn try_update_max_depth_price_remove_quote(self: &mut Self) {
        // invariant: был УДАЛЕН элемент СЛЕВА от self.max_depth_price 
        let has_cut_by_depth = self.max_depth_price != Price::MAX;
        if !has_cut_by_depth {
            return;
        }
        let mut cursor = self.levels.lower_bound(Bound::Included(&self.max_depth_price));
        debug_assert!(*cursor.peek_next().unwrap().0 == self.max_depth_price);
        cursor.next();
        if let Some((&price, &amount)) = cursor.peek_next() {
            self.max_depth_price = price;
            let index = self.aggregated_levels.len() - 1;
            if self.aggregated_levels[index].total_amount < self.aggregation_table.get_amount(index) {
                self.aggregated_levels[index].last_price = price;
                self.aggregated_levels[index].total_amount += amount;
            }
            else {
                self.aggregated_levels.push(AggregatedLevel::new(price, amount));
            }
        }
        else {
            self.max_depth_price = Price::MAX;
        }
    }
    fn add_quote(
        self: &mut Self, 
        price: Price, 
        amount: Amount
    ) {
        debug_assert!(self.levels.is_empty() == self.aggregated_levels.is_empty());
        if self.levels.is_empty() {
            self.levels.insert(price, amount);
            self.aggregated_levels.push(AggregatedLevel::new(price, amount));
            if self.aggregation_table.max_depth == 1 {
                self.max_depth_price = price;
            }
            return;
        }
        let is_price_new = match self.levels.try_insert(price, amount) {
            Ok(_) => true,
            Err(entry) => {
                *entry.entry.into_mut() += amount;
                false
            }
        };

        
        
        
        match self.aggregated_levels.binary_search_by(|level| level.last_price.cmp(&price)) {
            Ok(index) => {
                self.aggregated_levels[index].total_amount += amount;
                return;
            }
            Err(mut index) => {
                if index == self.aggregated_levels.len() {
                    if price > self.max_depth_price {
                        return;
                    }
                    debug_assert!(self.max_depth_price == Price::MAX);
                    
                    if self.levels.len() == self.aggregation_table.max_depth {
                        debug_assert!(is_price_new);
                        self.max_depth_price = price;
                    }
                    index -= 1;
                    self.aggregated_levels[index].last_price = price;    
                    self.aggregated_levels[index].total_amount += amount;
                }
                else {
                    self.aggregated_levels[index].total_amount += amount;
                    if is_price_new {
                        debug_assert!(price < self.max_depth_price);
                        self.try_update_max_depth_price();
                        self.try_cut_by_max_depth();
                    }
                }
                
                self.try_propogate_amount_surplus(index);   
            }
        }
    }
    fn try_propogate_shortage(self: &mut Self, mut index: usize) {
        // There may be levels with total_amount == 0 after the method execution
        if self.aggregated_levels[index].total_amount >= self.aggregation_table.get_amount(index) {
            return;
        } 
        let mut cursor = self.levels.lower_bound(Bound::Included(&self.aggregated_levels[index].last_price));
        debug_assert!(*cursor.peek_next().unwrap().0 == self.aggregated_levels[index].last_price);
        
        cursor.next();

        let mut index_to_steal_quotes = index + 1;
        while let Some((&price, &amount)) = cursor.peek_next() {
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
            if self.aggregated_levels[index].total_amount < self.aggregation_table.get_amount(index) {
                cursor.next();
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
            cursor.next();
        }
    }
    fn remove_quote(self: &mut Self, 
        price: Price, 
        amount: Amount) {
        let mut current_amount = self.levels.get_mut(&price).unwrap();
        debug_assert!(*current_amount >= amount);
        *current_amount -= amount;

        let should_remove_quote = *current_amount == 0;
        /*let should_remove_quote = match self.levels.entry(price) {
            std::collections::btree_map::Entry::Occupied(mut entry) => {
                let value = entry.get_mut();
                *value -= amount;
                if *value == 0 {
                    //entry.remove();
                    true
                }
                else {false}
            }
            std::collections::btree_map::Entry::Vacant(_) => {
                panic!("incorrect request in remove_quote!");
            }
        };*/

        if should_remove_quote && price <= self.max_depth_price  {
            self.try_update_max_depth_price_remove_quote()
        }
        match self.aggregated_levels.binary_search_by(|level| level.last_price.cmp(&price)) {
            Ok(index) => 'block: {
                self.aggregated_levels[index].total_amount -= amount;
                
                self.try_propogate_shortage(index);
                if !should_remove_quote {
                    while !self.aggregated_levels.is_empty() && self.aggregated_levels.last().unwrap().total_amount == 0 {
                        self.aggregated_levels.pop();
                    }
                    break 'block;
                }
                // last_price был обновлен элементами справа
                if self.aggregated_levels[index].last_price != price {
                    while !self.aggregated_levels.is_empty() && self.aggregated_levels.last().unwrap().total_amount == 0 {
                        self.aggregated_levels.pop();
                    }
                    break 'block;
                }
                let cursor = self.levels.lower_bound(Bound::Included(&self.aggregated_levels[index].last_price));
                debug_assert!(*cursor.peek_next().unwrap().0 == self.aggregated_levels[index].last_price);
                
                if self.aggregated_levels[index].total_amount == 0 {
                    debug_assert!(index + 1 == self.aggregated_levels.len());
                    self.aggregated_levels.pop();
                    break 'block;
                } 
                if let Some((&price, _)) = cursor.peek_prev() {
                    self.aggregated_levels[index].last_price = price;
                }
                else {
                    debug_assert!(index + 1 == self.aggregated_levels.len());
                    self.aggregated_levels.pop();
                }
            }
            Err(index) => 'block: {
                if index == self.aggregated_levels.len() {
                    break 'block;
                }
                self.aggregated_levels[index].total_amount -= amount;
                self.try_propogate_shortage(index);
                while !self.aggregated_levels.is_empty() && self.aggregated_levels.last().unwrap().total_amount == 0 {
                    self.aggregated_levels.pop();
                }
            }
        };
        if should_remove_quote {
            self.levels.remove(&price);
        }
    }
    fn new(table: AggregationTable) -> Self {
        Self {
            levels: BTreeMap::new(),
            max_depth_price: Price::MAX,
            aggregated_levels: Vec::new(),
            aggregation_table: table,
        }
    }
    fn set_quote(self: &mut Self, 
        price_: u64, 
        new_amount: Amount) {
        let price = Price::from(price_);
        if let Some(&current_amount) = self.levels.get(&price) {
            match new_amount.cmp(&current_amount) {
                std::cmp::Ordering::Greater => self.add_quote(price, new_amount - current_amount),
                std::cmp::Ordering::Less => self.remove_quote(price, current_amount - new_amount),
                std::cmp::Ordering::Equal => (),
            }
        } 
        else if new_amount != 0 {
            self.add_quote(price, new_amount);
        }
    }
    fn get_max_depth_price(&self) -> Price {
        return self.max_depth_price
    }
    fn get_levels(&self) -> &BTreeMap<Price, Amount>{
        return &self.levels;
    }

    fn get_aggregated_levels(&self) -> &Vec<AggregatedLevel<Price>>{
        return &self.aggregated_levels;
    }
    fn get_aggregated_levels_tuples(&self) -> Vec<(u64, u64)> {
        let mut result_clone = self.aggregated_levels.clone();
        result_clone.into_iter().map(|level| (level.last_price.into(), level.total_amount)).collect()
    }
}

struct SlowAggregatedL2ForTests<Price: OrderKey> {
    levels: BTreeMap<Price, Amount>,
    max_depth_price: Price,
    aggregated_levels: Vec<AggregatedLevel<Price>>,
    aggregation_table: AggregationTable
}


impl<Price: OrderKey> SlowAggregatedL2ForTests<Price>
where Price: From<u64>  {
    fn new(table: AggregationTable) -> Self {
        Self {
            levels: BTreeMap::new(),
            aggregated_levels: Vec::new(),
            aggregation_table: table,
            max_depth_price: Price::MAX
        }
    }
    fn set_quote(self: &mut Self, 
        price_: u64, 
        new_amount: Amount) {
        let price = Price::from(price_);
        match self.levels.try_insert(price, new_amount) {
            Ok(_) => {
                if new_amount == 0 {
                    self.levels.remove(&price);
                }
            },
            Err(entry) => {
                if new_amount == 0 {
                    entry.entry.remove();  
                }
                else {
                    *entry.entry.into_mut() = new_amount;
                }
            }
        };
        self.aggregated_levels.clear();
        for (quote_index, (&price, &amount)) in self.levels.iter().enumerate() {
            debug_assert!(amount > 0);
            if quote_index + 1 > self.aggregation_table.max_depth {
                break;
            }
            self.max_depth_price = price;
            if self.aggregated_levels.is_empty() {
                self.aggregated_levels.push(AggregatedLevel::new(price, amount));
                continue;
            }
            let index = self.aggregated_levels.len() - 1;
            if self.aggregated_levels[index].total_amount >= self.aggregation_table.get_amount(index) {
                self.aggregated_levels.push(AggregatedLevel::new(price, amount));
            }
            else {
                self.aggregated_levels[index].last_price = price;
                self.aggregated_levels[index].total_amount += amount;
            }
        }
        if self.levels.len() < self.aggregation_table.max_depth {
            self.max_depth_price = Price::MAX;
        }
    }

    fn get_max_depth_price(&self) -> Price {
        return self.max_depth_price
    }

    fn get_levels(&self) -> &BTreeMap<Price, Amount>{
        return &self.levels;
    }

    fn get_aggregated_levels(&self) -> &Vec<AggregatedLevel<Price>>{
        return &self.aggregated_levels;
    }
}

use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_problem_statement() {
        let table = AggregationTable::new([3, 5, 15].into(), 1, 999);
        let l2 = [(1, 2), (2, 2), (4, 1), (5, 4), (6, 8), (7, 10)];
        
        let mut solution = AggregatedL2::<AskKey>::new(table);
        for (price, amount) in l2 {
            solution.set_quote(price, amount);
        }
        assert_eq!(solution.get_aggregated_levels_tuples(), [(2, 4), (5, 5), (7, 18)]);
    }

    #[test]
    fn test_simple_with_removes() {
        let table = AggregationTable::new([2, 5, 3].into(), 1, 2);
        let mut solution = AggregatedL2::<AskKey>::new(table);
        solution.set_quote(1, 2);
        solution.set_quote(3, 2);
        solution.set_quote(3, 7);
        assert_eq!(solution.get_aggregated_levels_tuples(), [(1, 2), (3, 7)]);

        solution.set_quote(2, 4);
        solution.set_quote(2, 5);
        assert_eq!(solution.get_aggregated_levels_tuples(), [(1, 2), (2, 5)]);
        solution.set_quote(3, 1);
        solution.set_quote(1, 0);
        assert_eq!(solution.get_aggregated_levels_tuples(), [(2, 5), (3, 1)]);
        solution.set_quote(2, 1);
        assert_eq!(solution.get_aggregated_levels_tuples(), [(3, 2)]);
    }

    fn run_stress<Price: OrderKey>()
    where u64: From<Price> {
        let table = AggregationTable::new(vec![2, 6, 15, 8, 80], 12, 30);
        let mut fast_solution = AggregatedL2::<Price>::new(table.clone());
        let mut slow_solution = SlowAggregatedL2ForTests::<Price>::new(table.clone());
        
        let mut rng = ChaCha8Rng::seed_from_u64(0);;
        
        for i in 0..100000 {
            let price = rng.gen_range(1..=42);

            let mut amount: u64 = rng.gen_range(0..=17);
            if rng.gen_range(0..=100) == 0 { 
                amount = 0;
            }

            fast_solution.set_quote(price, amount);
            slow_solution.set_quote(price, amount);

            assert!(*fast_solution.get_levels() == *slow_solution.get_levels());
            assert!(*fast_solution.get_aggregated_levels() == *slow_solution.get_aggregated_levels());
            assert!(fast_solution.get_max_depth_price() == slow_solution.get_max_depth_price());
        }
    }


    #[test]
    fn test_stress_ask() {
        run_stress::<AskKey>(); 
    }

    #[test]
    fn test_stress_bid() {
        run_stress::<BidKey>(); 
    }

}

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

fn main() {
    let file = File::open("l2.json").expect("Cannot open file");
    let reader = BufReader::new(file);

    let table = AggregationTable::new(vec![1e12 as u64, 2e12 as u64, 1e13 as u64, 1e10 as u64], 1e12 as u64, 50);
    //let mut fast_solution = AggregatedL2::<Price>::new(table.clone());
    //let mut slow_solution = SlowAggregatedL2ForTests::<Price>::new(table.clone());
    let mut solution_for_ask = AggregatedL2::<AskKey>::new(table.clone());
    let mut solution_for_bid = AggregatedL2::<BidKey>::new(table.clone());
    
    let ratio: f64 = 1e8;

    for line in reader.lines() {
        let line = line.expect("Error reading line");
        let trade: Trade = serde_json::from_str(&line).expect("Invalid JSON format");
        
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
    
    println!("Hello world");
    
    /*
    result.set_quote(AskKey(1), 2);
    result.print(); 
    result.set_quote(AskKey(3), 2);
    result.print(); 
    result.set_quote(AskKey(3), 7);
    result.print(); 
    result.set_quote(AskKey(2), 4);
    result.print(); 
    result.set_quote(AskKey(2), 5);
    result.print();
    result.set_quote(AskKey(2), 2);
    result.print();*/
    /*
    let mut order_book = OrderBook::new();

    // Add bids
    order_book.add(100, 10.0, true);
    order_book.add(101, 5.0, true);
    order_book.add(102, 7.0, true);
    order_book.add(101, 3.0, true); // Merges at price level 101

    // Add asks
    order_book.add(103, 8.0, false);
    order_book.add(104, 12.0, false);
    order_book.add(103, 6.0, false); // Merges at price level 103

    order_book.print_order_book();

    // Get best bid/ask
    let (best_bid, best_ask) = order_book.best_bid_ask();
    println!("Best Bid: {:?}, Best Ask: {:?}", best_bid, best_ask);

    // Remove a bid order
    order_book.remove(101, 5.0, true);
    println!("\nAfter Removing 5 from 101 Bid:");
    order_book.print_order_book();*/
}


//type AgregatedL2<Key> = vec!<BTreMap<Keey, f64>>;


/*
/// Order book with L2 depth (price levels)
#[derive(Debug)]
struct OrderBook {
    
    bids: L2<BidKey>, // Price → Total Size (Descending)
    asks: L2<AskKey>, // Price → Total Size (Ascending)
}

impl OrderBook {
    fn new() -> Self {
        Self {
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
        }
    }

    /// Generic helper to modify an order book side
    fn modify_order<K: OrderKey + From<u64>>(
        book: &mut BTreeMap<K, f64>, 
        price: u64, 
        size: f64, 
        is_add: bool,
    ) {
        let key: K = price.into();
        if is_add {
            *book.entry(key).or_insert(0.0) += size;
        } else if let Some(total_size) = book.get_mut(&key) {
            *total_size -= size;
            if *total_size <= 0.0 {
                book.remove(&key);
            }
        }
    }

    /// Add liquidity to a price level
    fn add(&mut self, price: u64, size: f64, is_bid: bool) {
        if is_bid {
            Self::modify_order(&mut self.bids, price, size, true);
        } else {
            Self::modify_order(&mut self.asks, price, size, true);
        }
    }

    /// Remove liquidity from a price level
    fn remove(&mut self, price: u64, size: f64, is_bid: bool) {
        if is_bid {
            Self::modify_order(&mut self.bids, price, size, false);
        } else {
            Self::modify_order(&mut self.asks, price, size, false);
        }
    }

    /// Get best bid (highest) and best ask (lowest)
    fn best_bid_ask(&self) -> (Option<(u64, f64)>, Option<(u64, f64)>) {
        let best_bid = self.bids.iter().next().map(|(k, &v)| (k.0, v));
        let best_ask = self.asks.iter().next().map(|(k, &v)| (k.0, v));
        (best_bid, best_ask)
    }

    /// Print order book
    fn print_order_book(&self) {
        println!("Bids (Highest to Lowest):");
        for (BidKey(price), size) in &self.bids {
            println!("Price: {}, Total Size: {}", price, size);
        }

        println!("Asks (Lowest to Highest):");
        for (AskKey(price), size) in &self.asks {
            println!("Price: {}, Total Size: {}", price, size);
        }
    }
}
*/
