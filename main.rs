use std::collections::BTreeMap;
use std::cmp::Ordering;
use std::convert::From;


/// Generic trait for order keys (BidKey and AskKey)
trait OrderKey: Ord + Eq + Copy {
    const MAX: u64;
}



/// Bid key (sorted descending)
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
struct BidKey(u64);
impl OrderKey for BidKey {
    const MAX: u64 = 0; 
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
    const MAX: u64 = u64::MAX;
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
    fn try_propogate_amount_surplus(
        self: &mut Self, 
        index: usize
    ) {
        let last_quote_amount = self.levels.get(&self.aggregated_levels[index].last_price).unwrap();
        /*while aggregated_levels[index].total_amount - last_quote_amount >= aggregation_table {

        }*/
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


        // надо добавить в levels
        match self.aggregated_levels.binary_search_by(|level| level.last_price.cmp(&price)) {
            Ok(index) => {
                self.aggregated_levels[index].total_amount += amount;
                //return;
            }
            Err(index) => {
                // ещё depth_price надо подвинуть
                if index > self.aggregated_levels.len() {
                    // ...
                }
                self.aggregated_levels[index].total_amount += amount;
                
                
            }
        }
        /*if Some(index)
        unwrap_or_else(|e| e);
        if pos > levels.size() {
            --pos;
            // hz перечитай
        }
        
        let key: K = price.into();
        if is_add {
            *book.entry(key).or_insert(0.0) += size;
        } else if let Some(total_size) = book.get_mut(&key) {
            *total_size -= size;
            if *total_size <= 0.0 {
                book.remove(&key);
            }
        }*/
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