use std::collections::{BTreeMap, HashMap};
use std::hash::{BuildHasherDefault, Hasher};
use std::marker::PhantomData;

use crate::market::BookLevel;
use crate::values::{Price, Quantity};

use super::{BookSide, OrderBook};

const PAGE_BITS: u32 = 6;
const PAGE_SIZE: usize = 1 << PAGE_BITS;
const NONE: usize = usize::MAX;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PagedOrderBookError {
    InvalidTickSize(Price),
}

#[derive(Debug)]
pub struct PagedOrderBook {
    tick_raw: i64,
    bids: Side,
    asks: Side,
}

impl PagedOrderBook {
    pub fn new(tick_size: Price, reserved_pages: usize) -> Result<Self, PagedOrderBookError> {
        Self::validate(tick_size)?;
        Ok(Self {
            tick_raw: tick_size.raw(),
            bids: Side::new(reserved_pages),
            asks: Side::new(reserved_pages),
        })
    }

    fn validate(tick_size: Price) -> Result<(), PagedOrderBookError> {
        if tick_size.raw() <= 0 {
            return Err(PagedOrderBookError::InvalidTickSize(tick_size));
        }
        Ok(())
    }

    fn side_mut(&mut self, side: BookSide) -> &mut Side {
        match side {
            BookSide::Bid => &mut self.bids,
            BookSide::Ask => &mut self.asks,
        }
    }
}

impl OrderBook for PagedOrderBook {
    fn clear(&mut self) {
        self.bids.clear();
        self.asks.clear();
    }

    fn update_level(&mut self, side: BookSide, price: Price, quantity: Quantity) {
        let tick = price.raw() / self.tick_raw;
        self.side_mut(side).update_tick(tick, quantity);
    }

    fn apply_batch(&mut self, side: BookSide, levels: &mut [BookLevel]) {
        let tick_raw = self.tick_raw;
        let side = self.side_mut(side);
        let mut cached_key = 0_i64;
        let mut cached_id = NONE;
        for level in levels {
            let tick = level.price.raw() / tick_raw;
            let key = tick >> PAGE_BITS;
            cached_id = if cached_id != NONE && key == cached_key {
                side.update_page(cached_id, (tick as usize) & (PAGE_SIZE - 1), level.quantity)
            } else {
                cached_key = key;
                side.update_tick(tick, level.quantity)
            };
        }
    }

    fn best(&self, side: BookSide) -> Option<BookLevel> {
        match side {
            BookSide::Bid => self.bids.best::<Bid>(self.tick_raw),
            BookSide::Ask => self.asks.best::<Ask>(self.tick_raw),
        }
    }

    fn top_n(&self, side: BookSide, out: &mut [BookLevel]) -> usize {
        match side {
            BookSide::Bid => self.bids.top_n::<Bid>(self.tick_raw, out),
            BookSide::Ask => self.asks.top_n::<Ask>(self.tick_raw, out),
        }
    }

    fn levels(&self, side: BookSide) -> impl Iterator<Item = BookLevel> + '_ {
        match side {
            BookSide::Bid => Levels::Bids(PageLevels::new(&self.bids, self.tick_raw)),
            BookSide::Ask => Levels::Asks(PageLevels::new(&self.asks, self.tick_raw)),
        }
    }
}

#[derive(Debug, Default)]
struct IntegerHasher(u64);

impl Hasher for IntegerHasher {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write_i64(&mut self, value: i64) {
        let mut hash = value as u64;
        hash ^= hash >> 30;
        hash = hash.wrapping_mul(0xbf58_476d_1ce4_e5b9);
        hash ^= hash >> 27;
        hash = hash.wrapping_mul(0x94d0_49bb_1331_11eb);
        self.0 = hash ^ (hash >> 31);
    }

    fn write(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.0 = (self.0 ^ u64::from(byte)).wrapping_mul(0x100_0000_01b3);
        }
    }
}

#[derive(Debug)]
struct Page {
    key: i64,
    occupied: u64,
    prev: usize,
    next: usize,
    quantities: [Quantity; PAGE_SIZE],
}

#[derive(Debug)]
struct Side {
    by_page: HashMap<i64, usize, BuildHasherDefault<IntegerHasher>>,
    ordered: BTreeMap<i64, usize>,
    pages: Vec<Page>,
    first: usize,
    last: usize,
    free_head: usize,
}

impl Side {
    fn new(reserved_pages: usize) -> Self {
        Self {
            by_page: HashMap::with_capacity_and_hasher(reserved_pages, BuildHasherDefault::default()),
            ordered: BTreeMap::new(),
            pages: Vec::with_capacity(reserved_pages),
            first: NONE,
            last: NONE,
            free_head: NONE,
        }
    }

    fn clear(&mut self) {
        self.by_page.clear();
        self.ordered.clear();
        self.pages.clear();
        self.first = NONE;
        self.last = NONE;
        self.free_head = NONE;
    }

    #[inline]
    fn update_tick(&mut self, tick: i64, quantity: Quantity) -> usize {
        let key = tick >> PAGE_BITS;
        let offset = (tick as usize) & (PAGE_SIZE - 1);
        let Some(&id) = self.by_page.get(&key) else {
            return if quantity.raw() == 0 {
                NONE
            } else {
                self.insert_page(key, offset, quantity)
            };
        };
        self.update_page(id, offset, quantity)
    }

    #[inline]
    fn update_page(&mut self, id: usize, offset: usize, quantity: Quantity) -> usize {
        let bit = 1_u64 << offset;
        let page = &mut self.pages[id];
        if quantity.raw() == 0 {
            page.occupied &= !bit;
            if page.occupied == 0 {
                self.remove_page(id);
                return NONE;
            }
        } else {
            page.quantities[offset] = quantity;
            page.occupied |= bit;
        }
        id
    }

    #[inline(never)]
    fn insert_page(&mut self, key: i64, offset: usize, quantity: Quantity) -> usize {
        let prev = self
            .ordered
            .range(..key)
            .next_back()
            .map_or(NONE, |(_, &id)| id);
        let next = if prev == NONE { self.first } else { self.pages[prev].next };
        let id = if self.free_head == NONE {
            let id = self.pages.len();
            self.pages.push(Page {
                key,
                occupied: 1_u64 << offset,
                prev,
                next,
                quantities: [Quantity::default(); PAGE_SIZE],
            });
            id
        } else {
            let id = self.free_head;
            self.free_head = self.pages[id].next;
            let page = &mut self.pages[id];
            page.key = key;
            page.occupied = 1_u64 << offset;
            page.prev = prev;
            page.next = next;
            id
        };
        self.pages[id].quantities[offset] = quantity;
        if prev == NONE {
            self.first = id;
        } else {
            self.pages[prev].next = id;
        }
        if next == NONE {
            self.last = id;
        } else {
            self.pages[next].prev = id;
        }
        self.by_page.insert(key, id);
        self.ordered.insert(key, id);
        id
    }

    #[inline(never)]
    fn remove_page(&mut self, id: usize) {
        let page = &self.pages[id];
        let (key, prev, next) = (page.key, page.prev, page.next);
        if prev == NONE {
            self.first = next;
        } else {
            self.pages[prev].next = next;
        }
        if next == NONE {
            self.last = prev;
        } else {
            self.pages[next].prev = prev;
        }
        self.by_page.remove(&key);
        self.ordered.remove(&key);
        self.pages[id].next = self.free_head;
        self.free_head = id;
    }

    fn best<D: Direction>(&self, tick_raw: i64) -> Option<BookLevel> {
        let id = D::first(self);
        if id == NONE {
            return None;
        }
        let page = &self.pages[id];
        Some(level(page, D::offset(page.occupied), tick_raw))
    }

    fn top_n<D: Direction>(&self, tick_raw: i64, out: &mut [BookLevel]) -> usize {
        let mut written = 0;
        for (slot, value) in out.iter_mut().zip(PageLevels::<D>::new(self, tick_raw)) {
            *slot = value;
            written += 1;
        }
        written
    }
}

trait Direction {
    fn first(side: &Side) -> usize;
    fn next(page: &Page) -> usize;
    fn offset(occupied: u64) -> u32;
    fn take(remaining: &mut u64) -> u32;
}

struct Bid;
struct Ask;

impl Direction for Bid {
    fn first(side: &Side) -> usize {
        side.last
    }

    fn next(page: &Page) -> usize {
        page.prev
    }

    fn offset(occupied: u64) -> u32 {
        63 - occupied.leading_zeros()
    }
    fn take(remaining: &mut u64) -> u32 {
        let offset = Self::offset(*remaining);
        *remaining &= !(1_u64 << offset);
        offset
    }
}

impl Direction for Ask {
    fn first(side: &Side) -> usize {
        side.first
    }

    fn next(page: &Page) -> usize {
        page.next
    }

    fn offset(occupied: u64) -> u32 {
        occupied.trailing_zeros()
    }
    fn take(remaining: &mut u64) -> u32 {
        let offset = remaining.trailing_zeros();
        *remaining &= *remaining - 1;
        offset
    }
}

#[inline]
fn level(page: &Page, offset: u32, tick_raw: i64) -> BookLevel {
    let tick = (page.key << PAGE_BITS) | i64::from(offset);
    BookLevel {
        price: Price::from(tick * tick_raw),
        quantity: page.quantities[offset as usize],
    }
}

struct PageLevels<'a, D> {
    side: &'a Side,
    tick_raw: i64,
    page: usize,
    remaining: u64,
    direction: PhantomData<D>,
}

impl<'a, D: Direction> PageLevels<'a, D> {
    fn new(side: &'a Side, tick_raw: i64) -> Self {
        let page = D::first(side);
        let remaining = if page == NONE { 0 } else { side.pages[page].occupied };
        Self {
            side,
            tick_raw,
            page,
            remaining,
            direction: PhantomData,
        }
    }
}

impl<D: Direction> Iterator for PageLevels<'_, D> {
    type Item = BookLevel;

    fn next(&mut self) -> Option<Self::Item> {
        if self.page == NONE {
            return None;
        }
        if self.remaining == 0 {
            self.page = D::next(&self.side.pages[self.page]);
            if self.page == NONE {
                return None;
            }
            self.remaining = self.side.pages[self.page].occupied;
        }
        let offset = D::take(&mut self.remaining);
        Some(level(&self.side.pages[self.page], offset, self.tick_raw))
    }
}

enum Levels<'a> {
    Bids(PageLevels<'a, Bid>),
    Asks(PageLevels<'a, Ask>),
}

impl Iterator for Levels<'_> {
    type Item = BookLevel;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Bids(iter) => iter.next(),
            Self::Asks(iter) => iter.next(),
        }
    }
}
