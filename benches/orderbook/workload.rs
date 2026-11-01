use overnight_fi_test_task::engine::{BTreeOrderBook, BookSide, OrderBook, PagedOrderBook};
use overnight_fi_test_task::market::BookLevel;
use overnight_fi_test_task::values::{Price, Quantity};
use serde::Deserialize;

const TICK_RAW: i64 = 10_000_000;
const BASE_TICK: i64 = 1_000_000;
const DEPTH: usize = 1024;
const EVENTS: usize = 256;
const UPDATES_PER_SIDE: usize = 16;

#[derive(Clone)]
pub struct Updates {
    pub bids: Vec<BookLevel>,
    pub asks: Vec<BookLevel>,
}

pub struct Workload {
    pub name: String,
    pub tick_size: Price,
    pub snapshot: Updates,
    pub deltas: Vec<Updates>,
}

impl Workload {
    pub fn seed(&self, book: &mut impl OrderBook) {
        book.clear();
        for (side, levels) in [(BookSide::Bid, &self.snapshot.bids), (BookSide::Ask, &self.snapshot.asks)] {
            for level in levels {
                book.update_level(side, level.price, level.quantity);
            }
        }
    }

    pub fn level_updates(&self) -> usize {
        self.deltas
            .iter()
            .map(|delta| delta.bids.len() + delta.asks.len())
            .sum()
    }

    pub fn validate(&self) -> Result<(), String> {
        let mut btree = BTreeOrderBook::default();
        let mut paged = PagedOrderBook::new(self.tick_size, 1024).map_err(|error| format!("{}: {error:?}", self.name))?;
        for updates in std::iter::once(&self.snapshot).chain(&self.deltas) {
            if updates
                .bids
                .iter()
                .chain(&updates.asks)
                .any(|level| level.price.raw() % self.tick_size.raw() != 0)
            {
                return Err(format!("{}: price is off the configured tick grid", self.name));
            }
        }
        self.seed(&mut btree);
        self.seed(&mut paged);
        self.compare(&btree, &paged, 0)?;
        for (index, delta) in self.deltas.iter().enumerate() {
            let mut expected = delta.clone();
            let mut actual = delta.clone();
            apply(&mut btree, &mut expected);
            apply(&mut paged, &mut actual);
            self.compare(&btree, &paged, index + 1)?;
        }
        Ok(())
    }

    fn compare(&self, expected: &impl OrderBook, actual: &impl OrderBook, step: usize) -> Result<(), String> {
        for side in [BookSide::Bid, BookSide::Ask] {
            if !expected.levels(side).eq(actual.levels(side)) {
                return Err(format!("{}: all levels differ after step {step}, {side:?}", self.name));
            }
            if expected.best(side) != actual.best(side) {
                return Err(format!("{}: best differs after step {step}, {side:?}", self.name));
            }
            self.compare_top::<10>(expected, actual, side, step)?;
            self.compare_top::<20>(expected, actual, side, step)?;
        }
        Ok(())
    }

    fn compare_top<const N: usize>(&self, expected: &impl OrderBook, actual: &impl OrderBook, side: BookSide, step: usize) -> Result<(), String> {
        let mut expected_levels: [_; N] = std::array::from_fn(|_| level(0, 0));
        let mut actual_levels: [_; N] = std::array::from_fn(|_| level(0, 0));
        let expected_count = expected.top_n(side, &mut expected_levels);
        let actual_count = actual.top_n(side, &mut actual_levels);
        if expected_count != actual_count || expected_levels[..expected_count] != actual_levels[..actual_count] {
            return Err(format!("{}: top{N} differs after step {step}, {side:?}", self.name));
        }
        Ok(())
    }
}

fn apply(book: &mut impl OrderBook, updates: &mut Updates) {
    book.apply_batch(BookSide::Bid, &mut updates.bids);
    book.apply_batch(BookSide::Ask, &mut updates.asks);
}

#[derive(Deserialize)]
struct RawLevel {
    price: i64,
    quantity: i64,
}

impl From<RawLevel> for BookLevel {
    fn from(value: RawLevel) -> Self {
        level(value.price, value.quantity)
    }
}

#[derive(Deserialize)]
struct Record {
    snapshot: bool,
    bids: Vec<RawLevel>,
    asks: Vec<RawLevel>,
}

impl From<Record> for Updates {
    fn from(value: Record) -> Self {
        Self {
            bids: value.bids.into_iter().map(Into::into).collect(),
            asks: value.asks.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Deserialize)]
struct Dataset {
    tick_raw: i64,
    records: Vec<Record>,
}

fn fixture(name: &str, bytes: &[u8]) -> Result<Workload, String> {
    let dataset: Dataset = sonic_rs::from_slice(bytes).map_err(|error| format!("{name}: {error}"))?;
    let mut records = dataset.records.into_iter();
    let snapshot = records
        .next()
        .ok_or_else(|| format!("{name}: missing snapshot"))?;
    if !snapshot.snapshot {
        return Err(format!("{name}: first record must be a snapshot"));
    }
    let deltas = records
        .map(|record| {
            if record.snapshot {
                Err(format!("{name}: unexpected second snapshot"))
            } else {
                Ok(record.into())
            }
        })
        .collect::<Result<_, _>>()?;
    Ok(Workload {
        name: name.to_owned(),
        tick_size: Price::from(dataset.tick_raw),
        snapshot: snapshot.into(),
        deltas,
    })
}

fn level(price: i64, quantity: i64) -> BookLevel {
    BookLevel {
        price: Price::from(price),
        quantity: Quantity::from(quantity),
    }
}

fn synthetic_level(side: BookSide, index: usize, stride: i64, quantity: i64) -> BookLevel {
    let offset = (index as i64 + 1) * stride;
    let tick = match side {
        BookSide::Bid => BASE_TICK - offset,
        BookSide::Ask => BASE_TICK + offset,
    };
    level(tick * TICK_RAW, quantity)
}

fn snapshot(stride: i64) -> Updates {
    let side = |side| {
        (0..DEPTH)
            .map(|index| synthetic_level(side, index, stride, 100_000))
            .collect()
    };
    Updates {
        bids: side(BookSide::Bid),
        asks: side(BookSide::Ask),
    }
}

fn synthetic(name: &str, stride: i64, deltas: Vec<Updates>) -> Workload {
    Workload {
        name: format!("synthetic/{name}"),
        tick_size: Price::from(TICK_RAW),
        snapshot: snapshot(stride),
        deltas,
    }
}

fn existing_quantity() -> Workload {
    let deltas = (0..EVENTS)
        .map(|event| {
            let side = |side| {
                (0..UPDATES_PER_SIDE)
                    .map(|offset| {
                        let index = (event * UPDATES_PER_SIDE + offset) % DEPTH;
                        synthetic_level(side, index, 1, (event as i64 + 2) * 100_000)
                    })
                    .collect()
            };
            Updates {
                bids: side(BookSide::Bid),
                asks: side(BookSide::Ask),
            }
        })
        .collect();
    synthetic("existing_quantity", 1, deltas)
}

fn dense_insert() -> Workload {
    let deltas = (0..EVENTS)
        .map(|event| {
            let side = |side| {
                (0..UPDATES_PER_SIDE)
                    .map(|offset| synthetic_level(side, DEPTH + event * UPDATES_PER_SIDE + offset, 1, 200_000))
                    .collect()
            };
            Updates {
                bids: side(BookSide::Bid),
                asks: side(BookSide::Ask),
            }
        })
        .collect();
    synthetic("dense_insert", 1, deltas)
}

fn dense_delete() -> Workload {
    let deltas = (0..DEPTH / UPDATES_PER_SIDE / 2)
        .map(|event| {
            let side = |side| {
                (0..UPDATES_PER_SIDE)
                    .map(|offset| synthetic_level(side, event * UPDATES_PER_SIDE + offset, 1, 0))
                    .collect()
            };
            Updates {
                bids: side(BookSide::Bid),
                asks: side(BookSide::Ask),
            }
        })
        .collect();
    synthetic("dense_delete", 1, deltas)
}

fn sparse_churn() -> Workload {
    let deltas = (0..EVENTS)
        .map(|event| {
            let side = |side| {
                (0..UPDATES_PER_SIDE / 2)
                    .flat_map(|offset| {
                        let index = event * (UPDATES_PER_SIDE / 2) + offset;
                        [synthetic_level(side, index, 128, 0), synthetic_level(side, DEPTH + index, 128, 200_000)]
                    })
                    .collect()
            };
            Updates {
                bids: side(BookSide::Bid),
                asks: side(BookSide::Ask),
            }
        })
        .collect();
    synthetic("sparse_churn", 128, deltas)
}

pub fn load_all() -> Result<Vec<Workload>, String> {
    Ok(vec![
        fixture("fixture/binance_btcusdt", include_bytes!("fixtures/binance_btcusdt.json"))?,
        fixture("fixture/aster_btcusdt", include_bytes!("fixtures/aster_btcusdt.json"))?,
        existing_quantity(),
        dense_insert(),
        dense_delete(),
        sparse_churn(),
    ])
}
