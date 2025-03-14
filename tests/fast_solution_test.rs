#![feature(btree_cursors)]
#![feature(map_try_insert)]

pub use market_data_aggregator::common::*;
pub use market_data_aggregator::solutions::fast::*;
pub use market_data_aggregator::solutions::slow_for_comparisons::*;

use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_problem_statement() {
        let table = SubscriptionRules::new([3, 5, 15].into(), 1, 999);
        let l2 = [(1, 2), (2, 2), (4, 1), (5, 4), (6, 8), (7, 10)];

        let mut solution = AggregatedL2::<AskKey>::new(table);
        for (price, amount) in l2 {
            solution.set_quote(price, amount);
        }
        assert_eq!(
            solution.get_aggregated_levels_tuples(),
            [(2, 4), (5, 5), (7, 18)]
        );
    }

    #[test]
    fn test_simple_with_removes() {
        let table = SubscriptionRules::new([2, 5, 3].into(), 1, 2);
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
    where
        u64: From<Price>,
    {
        let table = SubscriptionRules::new(vec![2, 6, 15, 8, 80], 12, 30);
        let mut fast_solution = AggregatedL2::<Price>::new(table.clone());
        let mut slow_solution = SlowAggregatedL2ForComparisons::<Price>::new(table.clone());

        let mut rng = ChaCha8Rng::seed_from_u64(0);

        for _ in 0..100000 {
            let price = rng.gen_range(1..=42);

            let mut amount: u64 = rng.gen_range(0..=17);
            if rng.gen_range(0..=100) == 0 {
                amount = 0;
            }

            fast_solution.set_quote(price, amount);
            slow_solution.set_quote(price, amount);

            assert!(*fast_solution.get_levels() == *slow_solution.get_levels());
            assert!(
                *fast_solution.get_aggregated_levels() == *slow_solution.get_aggregated_levels()
            );
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
