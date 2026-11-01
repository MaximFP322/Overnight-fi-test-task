use crate::instrument::InstrumentId;
use crate::market::{BookDelta, FeedState, FeedStatus, MarketEvent, Venue};

use super::super::ResyncRequest;
use super::{BookSide, OrderBook};

#[derive(Default)]
pub struct MarketState<BOOK> {
    binance: BookState<BOOK>,
    aster: BookState<BOOK>,
}

#[derive(Default)]
pub struct BookState<BOOK> {
    book: BOOK,
    state: FeedState,
    last_update_id: Option<u64>,
    last_received_ts_ns: Option<u64>,
    awaiting_bridge: bool,
    resync_pending: bool,
}

impl<BOOK> MarketState<BOOK> {
    pub fn new(binance: BOOK, aster: BOOK) -> Self {
        Self {
            binance: BookState::new(binance),
            aster: BookState::new(aster),
        }
    }

    pub fn book(&self, venue: Venue) -> &BookState<BOOK> {
        match venue {
            Venue::Binance => &self.binance,
            Venue::Aster => &self.aster,
        }
    }

    pub(crate) fn apply_status(&mut self, status: FeedStatus) {
        self.book_mut(status.venue).state = status.state;
    }

    fn book_mut(&mut self, venue: Venue) -> &mut BookState<BOOK> {
        match venue {
            Venue::Binance => &mut self.binance,
            Venue::Aster => &mut self.aster,
        }
    }
}

impl<BOOK: OrderBook> MarketState<BOOK> {
    pub(crate) fn apply_market(&mut self, event: MarketEvent, received_ts_ns: Option<u64>) -> Option<ResyncRequest> {
        let (venue, instrument_id) = match &event {
            MarketEvent::Snapshot(snapshot) => (snapshot.venue, snapshot.instrument_id),
            MarketEvent::Delta(delta) => (delta.venue, delta.instrument_id),
        };
        let state = self.book_mut(venue);
        match event {
            MarketEvent::Snapshot(mut snapshot) => {
                state.book.clear();
                state.book.apply_batch(BookSide::Bid, &mut snapshot.bids);
                state.book.apply_batch(BookSide::Ask, &mut snapshot.asks);
                state.last_update_id = Some(snapshot.last_update_id);
                state.last_received_ts_ns = received_ts_ns;
                state.awaiting_bridge = true;
                state.resync_pending = false;
                None
            }
            MarketEvent::Delta(mut delta) => {
                if !state.can_apply_delta(&delta) {
                    state.invalidate();
                    return state.request_resync(venue, instrument_id);
                }
                state.book.apply_batch(BookSide::Bid, &mut delta.bids);
                state.book.apply_batch(BookSide::Ask, &mut delta.asks);
                state.last_update_id = Some(delta.final_update_id);
                state.last_received_ts_ns = received_ts_ns;
                state.awaiting_bridge = false;
                None
            }
        }
    }
}

impl<BOOK> BookState<BOOK> {
    fn new(book: BOOK) -> Self {
        Self {
            book,
            state: FeedState::Disconnected,
            last_update_id: None,
            last_received_ts_ns: None,
            awaiting_bridge: false,
            resync_pending: false,
        }
    }

    pub fn book(&self) -> &BOOK {
        &self.book
    }

    pub fn state(&self) -> FeedState {
        self.state
    }

    pub fn is_live(&self) -> bool {
        self.state == FeedState::Live && self.last_update_id.is_some() && !self.awaiting_bridge
    }

    pub fn last_update_id(&self) -> Option<u64> {
        self.last_update_id
    }

    pub fn last_received_ts_ns(&self) -> Option<u64> {
        self.last_received_ts_ns
    }
}

impl<BOOK: OrderBook> BookState<BOOK> {
    fn invalidate(&mut self) {
        self.book.clear();
        self.last_update_id = None;
        self.last_received_ts_ns = None;
        self.awaiting_bridge = false;
    }

    fn can_apply_delta(&self, delta: &BookDelta) -> bool {
        match self.last_update_id {
            Some(last_update_id) if self.awaiting_bridge => delta.first_update_id <= last_update_id,
            Some(last_update_id) => delta.previous_final_update_id == Some(last_update_id),
            None => false,
        }
    }

    fn request_resync(&mut self, venue: Venue, instrument_id: InstrumentId) -> Option<ResyncRequest> {
        self.state = FeedState::Stale;
        if self.resync_pending {
            return None;
        }
        self.resync_pending = true;
        Some(ResyncRequest { venue, instrument_id })
    }
}
