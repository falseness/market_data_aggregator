use std::collections::BTreeMap;
use std::cmp::Ordering;
use std::convert::From;


/// Generic trait for order keys (BidKey and AskKey)
trait OrderKey: Ord + Eq + Copy {}



/// Bid key (sorted descending)
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
struct BidKey(u64);
impl OrderKey for BidKey {}
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
impl OrderKey for AskKey {}
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

/// Order book with L2 depth (price levels)
#[derive(Debug)]
struct OrderBook {
    bids: BTreeMap<BidKey, f64>, // Price → Total Size (Descending)
    asks: BTreeMap<AskKey, f64>, // Price → Total Size (Ascending)
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

fn main() {
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
    order_book.print_order_book();
}