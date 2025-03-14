use crate::common::*;



#[derive(Clone)]
pub struct SubscriptionRules {
    minimum_amounts: Vec<Amount>,
    fallback: Amount,
    pub max_depth: usize
}

impl SubscriptionRules {
    pub fn get_amount(self: &Self, index: usize) -> Amount {
        if index >= self.minimum_amounts.len() {
            return self.fallback;
        }
        return self.minimum_amounts[index];
    }
    pub fn new(minimum_amounts: Vec<Amount>,
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


