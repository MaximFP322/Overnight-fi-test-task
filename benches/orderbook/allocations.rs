use std::alloc::System;

use stats_alloc::{INSTRUMENTED_SYSTEM, Region, StatsAlloc};

#[global_allocator]
static ALLOCATOR: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

pub fn record(name: &str, operation: &str, book: &str, unit: &str, operations: usize, routine: impl FnOnce()) {
    let region = Region::new(ALLOCATOR);
    routine();
    let stats = region.change();
    println!(
        "allocations {name}/{operation}/{book}: allocs={} frees={} reallocs={} bytes_allocated={} bytes_freed={} realloc_bytes_delta={} allocs/{unit}={:.3} bytes/{unit}={:.3}",
        stats.allocations,
        stats.deallocations,
        stats.reallocations,
        stats.bytes_allocated,
        stats.bytes_deallocated,
        stats.bytes_reallocated,
        stats.allocations as f64 / operations as f64,
        stats.bytes_allocated as f64 / operations as f64,
    );
}
