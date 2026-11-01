#[cfg(feature = "bench-allocations")]
#[path = "orderbook/allocations.rs"]
mod allocations;
#[path = "orderbook/measure.rs"]
mod measure;
#[path = "orderbook/workload.rs"]
mod workload;

use criterion::{Criterion, criterion_group, criterion_main};
use std::time::Duration;

fn orderbook(criterion: &mut Criterion) {
    let workloads = workload::load_all().expect("committed benchmark fixtures must parse");
    for workload in &workloads {
        workload
            .validate()
            .expect("benchmark books must agree for every input event");
        measure::register(criterion, workload);
    }
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .sample_size(50)
        .warm_up_time(Duration::from_secs(2))
        .measurement_time(Duration::from_secs(4))
        .nresamples(10_000);
    targets = orderbook
}
criterion_main!(benches);
