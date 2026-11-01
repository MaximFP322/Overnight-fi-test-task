use std::hint::black_box;

use criterion::measurement::WallTime;
use criterion::{BatchSize, BenchmarkGroup, BenchmarkId, Criterion, Throughput};
use overnight_fi_test_task::engine::{BTreeOrderBook, BookSide, OrderBook, PagedOrderBook};
use overnight_fi_test_task::market::BookLevel;
use overnight_fi_test_task::values::{Price, Quantity};

use crate::workload::{Updates, Workload};

pub fn register(criterion: &mut Criterion, workload: &Workload) {
    let tick_size = black_box(workload.tick_size);
    let btree = BTreeOrderBook::default;
    let paged = || PagedOrderBook::new(tick_size, 1024).expect("workload validation guarantees a positive tick size");

    let mut replay = criterion.benchmark_group(format!("{}/replay", workload.name));
    replay.throughput(Throughput::Elements(workload.level_updates() as u64));
    replay_case::<_, 0>(&mut replay, workload, "apply", "BTree", btree);
    replay_case::<_, 0>(&mut replay, workload, "apply", "Paged", paged);
    replay_case::<_, 1>(&mut replay, workload, "apply_best", "BTree", btree);
    replay_case::<_, 1>(&mut replay, workload, "apply_best", "Paged", paged);
    replay_case::<_, 10>(&mut replay, workload, "apply_top10", "BTree", btree);
    replay_case::<_, 10>(&mut replay, workload, "apply_top10", "Paged", paged);
    replay_case::<_, 20>(&mut replay, workload, "apply_top20", "BTree", btree);
    replay_case::<_, 20>(&mut replay, workload, "apply_top20", "Paged", paged);
    replay.finish();

    let mut reads = criterion.benchmark_group(format!("{}/read", workload.name));
    reads.throughput(Throughput::Elements(1));
    read_case::<_, 1>(&mut reads, workload, "best", "BTree", btree);
    read_case::<_, 1>(&mut reads, workload, "best", "Paged", paged);
    read_case::<_, 10>(&mut reads, workload, "top10", "BTree", btree);
    read_case::<_, 10>(&mut reads, workload, "top10", "Paged", paged);
    read_case::<_, 20>(&mut reads, workload, "top20", "BTree", btree);
    read_case::<_, 20>(&mut reads, workload, "top20", "Paged", paged);
    reads.finish();
}

fn replay_case<B: OrderBook, const N: usize>(
    group: &mut BenchmarkGroup<'_, WallTime>,
    workload: &Workload,
    operation: &str,
    book_name: &str,
    make: impl Fn() -> B,
) {
    #[cfg(feature = "bench-allocations")]
    let mut allocations_recorded = false;
    group.bench_function(BenchmarkId::new(operation, book_name), |bencher| {
        #[cfg(feature = "bench-allocations")]
        if !allocations_recorded {
            let mut book = make();
            workload.seed(&mut book);
            let mut deltas = workload.deltas.clone();
            let mut out = empty_levels();
            crate::allocations::record(&workload.name, operation, book_name, "delta", deltas.len(), || {
                run_replay::<_, N>(&mut book, &mut deltas, &mut out);
            });
            allocations_recorded = true;
        }
        bencher.iter_batched_ref(
            || {
                let mut book = make();
                workload.seed(&mut book);
                (book, workload.deltas.clone(), empty_levels())
            },
            |(book, deltas, out)| {
                run_replay::<_, N>(book, deltas, out);
            },
            BatchSize::LargeInput,
        );
    });
}

fn read_case<B: OrderBook, const N: usize>(
    group: &mut BenchmarkGroup<'_, WallTime>,
    workload: &Workload,
    operation: &str,
    book_name: &str,
    make: impl Fn() -> B,
) {
    #[cfg(feature = "bench-allocations")]
    let mut allocations_recorded = false;
    group.bench_function(BenchmarkId::new(operation, book_name), |bencher| {
        let mut book = make();
        workload.seed(&mut book);
        let mut deltas = workload.deltas.clone();
        for delta in &mut deltas {
            book.apply_batch(BookSide::Bid, &mut delta.bids);
            book.apply_batch(BookSide::Ask, &mut delta.asks);
        }
        let mut out = empty_levels();
        #[cfg(feature = "bench-allocations")]
        if !allocations_recorded {
            crate::allocations::record(&workload.name, operation, book_name, "query", 1, || {
                query::<_, N>(&book, &mut out);
            });
            allocations_recorded = true;
        }
        bencher.iter(|| query::<_, N>(&book, &mut out));
    });
}

fn run_replay<B: OrderBook, const N: usize>(book: &mut B, deltas: &mut [Updates], out: &mut [BookLevel; 20]) {
    for delta in black_box(deltas) {
        book.apply_batch(BookSide::Bid, &mut delta.bids);
        book.apply_batch(BookSide::Ask, &mut delta.asks);
        query::<_, N>(book, out);
    }
    black_box(book);
}

fn empty_levels() -> [BookLevel; 20] {
    std::array::from_fn(|_| BookLevel {
        price: Price::from(0),
        quantity: Quantity::from(0),
    })
}

#[inline(always)]
fn query<B: OrderBook, const N: usize>(book: &B, out: &mut [BookLevel; 20]) {
    if N == 1 {
        black_box(black_box(book).best(BookSide::Bid));
        black_box(black_box(book).best(BookSide::Ask));
    } else if N > 1 {
        let count = black_box(book).top_n(BookSide::Bid, &mut out[..N]);
        black_box(&out[..count]);
        let count = black_box(book).top_n(BookSide::Ask, &mut out[..N]);
        black_box(&out[..count]);
    }
}
