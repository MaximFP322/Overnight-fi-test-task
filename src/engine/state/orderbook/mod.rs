mod btree;
mod paged;

use crate::market::BookLevel;
use crate::values::{Price, Quantity};

pub use btree::BTreeOrderBook;
pub use paged::{PagedOrderBook, PagedOrderBookError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BookSide {
    Bid,
    Ask,
}

pub trait OrderBook {
    fn clear(&mut self);
    fn update_level(&mut self, side: BookSide, price: Price, quantity: Quantity);
    fn apply_batch(&mut self, side: BookSide, levels: &mut [BookLevel]);

    fn best(&self, side: BookSide) -> Option<BookLevel>;
    fn top_n(&self, side: BookSide, out: &mut [BookLevel]) -> usize;

    fn levels(&self, side: BookSide) -> impl Iterator<Item = BookLevel> + '_;
}
