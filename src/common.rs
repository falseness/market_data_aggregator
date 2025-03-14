use std::cmp::Ordering;

pub trait OrderKey: Ord + Eq + Copy + std::fmt::Debug + From<u64> {
    const MAX: Self;
}

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
pub struct BidKey(u64);
impl OrderKey for BidKey {
    const MAX: Self = Self(0); 
}
impl Ord for BidKey {
    fn cmp(&self, other: &Self) -> Ordering {
        other.0.cmp(&self.0)
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
pub struct AskKey(u64);
impl OrderKey for AskKey {
    const MAX: Self = Self(u64::MAX);
}
impl Ord for AskKey {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
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

pub type Amount = u64;


#[derive(Debug, PartialEq, Clone)]
pub struct AggregatedLevel<Price: OrderKey> {
    pub last_price: Price,
    pub total_amount: Amount
}

