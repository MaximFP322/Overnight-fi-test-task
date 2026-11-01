use crate::engine::strategy::SignalBias;
use crate::engine::{BookSide, BookState, MarketState, OrderBook};
use crate::market::Venue;
use crate::values::Price;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrossVenueSpreadSignal {
    pub bias: SignalBias,
    pub reason: SignalReason,
    pub binance: SignalBook,
    pub aster: SignalBook,
    pub long_bps_x100: Option<i128>,
    pub short_bps_x100: Option<i128>,
    pub threshold_bps: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalReason {
    SpreadExceeded,
    BelowThreshold,
    NotLive(Venue),
    Stale(Venue),
    MissingTimestamp(Venue),
    EmptyBook(Venue),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SignalBook {
    pub bid: Option<Price>,
    pub ask: Option<Price>,
    pub age_ns: Option<u64>,
}

#[derive(Debug)]
pub struct CrossVenueSpread {
    threshold_bps: u32,
    max_age_ns: u64,
    last_decision: Option<(SignalBias, SignalReason)>,
}

impl CrossVenueSpread {
    pub const fn new(threshold_bps: u32, max_age_ns: u64) -> Self {
        Self {
            threshold_bps,
            max_age_ns,
            last_decision: None,
        }
    }

    pub fn on_state(&mut self, state: &MarketState<impl OrderBook>, now_ns: u64) -> Option<CrossVenueSpreadSignal> {
        let binance_book = state.book(Venue::Binance);
        let aster_book = state.book(Venue::Aster);
        let binance = Self::read_book(binance_book, now_ns);
        let aster = Self::read_book(aster_book, now_ns);
        let mut blocked = None;
        for (venue, book, values) in [(Venue::Binance, binance_book, &binance), (Venue::Aster, aster_book, &aster)] {
            blocked = match (book.is_live(), values.age_ns) {
                (false, _) => Some(SignalReason::NotLive(venue)),
                (true, None) => Some(SignalReason::MissingTimestamp(venue)),
                (true, Some(age_ns)) if age_ns > self.max_age_ns => Some(SignalReason::Stale(venue)),
                (true, Some(_age_ns)) => None,
            };
            if blocked.is_some() {
                break;
            }
        }

        let (bias, reason, long_bps_x100, short_bps_x100) = match blocked {
            Some(reason) => (SignalBias::Hold, reason, None, None),
            None => match (binance.bid, binance.ask, aster.bid, aster.ask) {
                (Some(binance_bid), Some(binance_ask), Some(aster_bid), Some(aster_ask)) => {
                    let binance_ask = i128::from(binance_ask.raw());
                    let aster_ask = i128::from(aster_ask.raw());
                    let long_numerator = (i128::from(binance_bid.raw()) - aster_ask) * 10_000;
                    let short_numerator = (i128::from(aster_bid.raw()) - binance_ask) * 10_000;
                    let threshold = i128::from(self.threshold_bps);
                    let (bias, reason) = if long_numerator > threshold * aster_ask {
                        (SignalBias::LongBias, SignalReason::SpreadExceeded)
                    } else if short_numerator > threshold * binance_ask {
                        (SignalBias::ShortBias, SignalReason::SpreadExceeded)
                    } else {
                        (SignalBias::Hold, SignalReason::BelowThreshold)
                    };
                    (
                        bias,
                        reason,
                        Some(long_numerator * 100 / aster_ask),
                        Some(short_numerator * 100 / binance_ask),
                    )
                }
                (None, _, _, _) | (_, None, _, _) => (SignalBias::Hold, SignalReason::EmptyBook(Venue::Binance), None, None),
                (_, _, None, _) | (_, _, _, None) => (SignalBias::Hold, SignalReason::EmptyBook(Venue::Aster), None, None),
            },
        };

        if self.last_decision == Some((bias, reason)) {
            return None;
        }
        self.last_decision = Some((bias, reason));
        Some(CrossVenueSpreadSignal {
            bias,
            reason,
            binance,
            aster,
            long_bps_x100,
            short_bps_x100,
            threshold_bps: self.threshold_bps,
        })
    }

    fn read_book(book: &BookState<impl OrderBook>, now_ns: u64) -> SignalBook {
        SignalBook {
            bid: book.book().best(BookSide::Bid).map(|level| level.price),
            ask: book.book().best(BookSide::Ask).map(|level| level.price),
            age_ns: book
                .last_received_ts_ns()
                .map(|received_ns| now_ns.saturating_sub(received_ns)),
        }
    }
}
