use std::collections::BTreeMap;

use crate::market::BookLevel;
use crate::values::{Price, Quantity};

use super::{BookSide, OrderBook};

#[derive(Debug, Default)]
pub struct BTreeOrderBook {
    bids: BTreeMap<Price, Quantity>,
    asks: BTreeMap<Price, Quantity>,
}

impl OrderBook for BTreeOrderBook {
    fn clear(&mut self) {
        self.bids.clear();
        self.asks.clear();
    }

    fn update_level(&mut self, side: BookSide, price: Price, quantity: Quantity) {
        let levels = match side {
            BookSide::Bid => &mut self.bids,
            BookSide::Ask => &mut self.asks,
        };
        if quantity.raw() == 0 {
            levels.remove(&price);
        } else {
            levels.insert(price, quantity);
        }
    }

    fn apply_batch(&mut self, side: BookSide, levels: &mut [BookLevel]) {
        for level in levels {
            self.update_level(side, level.price, level.quantity);
        }
    }

    fn best(&self, side: BookSide) -> Option<BookLevel> {
        let level = match side {
            BookSide::Bid => self.bids.last_key_value(),
            BookSide::Ask => self.asks.first_key_value(),
        };
        level.map(|(&price, &quantity)| BookLevel { price, quantity })
    }

    fn top_n(&self, side: BookSide, out: &mut [BookLevel]) -> usize {
        let mut written = 0;
        for (slot, level) in out.iter_mut().zip(self.levels(side)) {
            *slot = level;
            written += 1;
        }
        written
    }

    fn levels(&self, side: BookSide) -> impl Iterator<Item = BookLevel> + '_ {
        let mut levels = match side {
            BookSide::Bid => self.bids.iter(),
            BookSide::Ask => self.asks.iter(),
        };
        std::iter::from_fn(move || {
            let level = match side {
                BookSide::Bid => levels.next_back(),
                BookSide::Ask => levels.next(),
            };
            level.map(|(&price, &quantity)| BookLevel { price, quantity })
        })
    }
}
