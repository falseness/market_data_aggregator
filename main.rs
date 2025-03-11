#![feature(btree_cursors)]
#![feature(map_try_insert)]

use std::borrow::Borrow;
use std::collections::BTreeMap;
use std::cmp::Ordering;
use std::convert::From;
use std::ops::Bound;


/// Generic trait for order keys (BidKey and AskKey)
trait OrderKey: Ord + Eq + Copy {
    const MAX: Self;
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


struct BidSide {

}
struct AskSide {}


trait Side {
    type PriceKey;
}

impl Side for BidSide {
    type PriceKey = BidKey;
}

impl Side for AskSide {
    type PriceKey = AskKey;
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



struct AggregationTable {
    minimum_amounts: Vec<Amount>,
    fallback: Amount,
    max_depth: usize
}

impl AggregationTable {
    fn get_amount(self: &Self, index: usize) -> Amount {
        if index > self.minimum_amounts.len() {
            return self.fallback;
        }
        return self.minimum_amounts[index];
    }
}

struct AggregatedL2<Price: OrderKey> {
    levels: BTreeMap<Price, Amount>,
    max_depth_price: Price,
    aggregated_levels: Vec<AggregatedLevel<Price>>,
    aggregation_table: AggregationTable
}

// проверь книгу на пустоту
impl<Price: OrderKey> AggregatedL2<Price> {
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
                if index + 1 > self.aggregated_levels.len() {
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
    // удали нули из aggregation_table
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
        // тут сложнее же, чем insert???
        let is_price_new = match self.levels.try_insert(price, amount) {
            Ok(_) => true,
            Err(entry) => {
                *entry.entry.into_mut() += amount;
                false
            }
        };

        if is_price_new && price < self.max_depth_price {
            self.try_update_max_depth_price();
            self.try_cut_by_max_depth();
        }
        
        
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
                }
                else {
                }
                self.aggregated_levels[index].total_amount += amount;
                self.try_propogate_amount_surplus(index);   
            }
        }
    }
    fn try_propogate_shortage(self: &mut Self, mut index: usize) {
        // удали total_amount == 0 с конца
        
        if self.aggregated_levels[index].total_amount >= self.aggregation_table.get_amount(index) {
            return;
        } 
        // пока верим, что есть такой элемент в btreemap
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

        if price < self.max_depth_price {
            self.try_update_max_depth_price_remove_quote()
        }
        match self.aggregated_levels.binary_search_by(|level| level.last_price.cmp(&price)) {
            Ok(index) => 'block: {
                self.aggregated_levels[index].total_amount -= amount;
                
                self.try_propogate_shortage(index);
                if !should_remove_quote {
                    if self.aggregated_levels.last().unwrap().total_amount == 0 {
                        self.aggregated_levels.pop();
                    }
                    break 'block;
                }
                // last_price был обновлен элементами справа
                if self.aggregated_levels[index].last_price != price {
                    if self.aggregated_levels.last().unwrap().total_amount == 0 {
                        self.aggregated_levels.pop();
                    }
                    break 'block;
                }
                let cursor = self.levels.lower_bound(Bound::Included(&self.aggregated_levels[index].last_price));
                debug_assert!(*cursor.peek_next().unwrap().0 == self.aggregated_levels[index].last_price);
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
                if self.aggregated_levels.last().unwrap().total_amount == 0 {
                    self.aggregated_levels.pop();
                }
            }
        };
        self.levels.remove(&price);
    }

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
fn main() {
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