use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use std::hint::black_box;

fn count_tokens_benchmark(c: &mut Criterion) {
    let small_content = "fn main() { println!(\"Hello, world!\"); }";
    let medium_content = small_content.repeat(100);
    let large_content = small_content.repeat(1000);

    let mut group = c.benchmark_group("tokenization");

    group.throughput(Throughput::Bytes(small_content.len() as u64));
    group.bench_function("small_40b", |b| {
        b.iter(|| abyss::utils::tokens::count_tokens(black_box(small_content)))
    });

    group.throughput(Throughput::Bytes(medium_content.len() as u64));
    group.bench_function("medium_4kb", |b| {
        b.iter(|| abyss::utils::tokens::count_tokens(black_box(&medium_content)))
    });

    group.throughput(Throughput::Bytes(large_content.len() as u64));
    group.bench_function("large_40kb", |b| {
        b.iter(|| abyss::utils::tokens::count_tokens(black_box(&large_content)))
    });

    group.finish();
}

fn parallel_tokenization_benchmark(c: &mut Criterion) {
    use rayon::prelude::*;

    let files: Vec<String> = (0..100)
        .map(|i| format!("fn file{}() {{ println!(\"content {}\"); }}", i, i).repeat(50))
        .collect();

    let mut group = c.benchmark_group("parallel_tokenization");

    group.bench_function("sequential", |b| {
        b.iter(|| {
            files
                .iter()
                .filter_map(|f| abyss::utils::tokens::count_tokens(f).ok())
                .sum::<usize>()
        })
    });

    group.bench_function("parallel_rayon", |b| {
        b.iter(|| {
            files
                .par_iter()
                .filter_map(|f| abyss::utils::tokens::count_tokens(f).ok())
                .sum::<usize>()
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    count_tokens_benchmark,
    parallel_tokenization_benchmark
);
criterion_main!(benches);
