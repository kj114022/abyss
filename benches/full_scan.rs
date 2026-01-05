use criterion::{Criterion, criterion_group, criterion_main};
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use tempfile::TempDir;

fn create_test_repo(file_count: usize) -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    // Create source directory
    std::fs::create_dir_all(root.join("src")).unwrap();

    for i in 0..file_count {
        let content = format!(
            r#"
/// File {} documentation
fn function_{}() -> i32 {{
    let x = {};
    println!("Computing...");
    x * 2
}}

struct Data{} {{
    id: u32,
    name: String,
}}

impl Data{} {{
    fn new() -> Self {{
        Self {{ id: {}, name: "test".to_string() }}
    }}
}}
"#,
            i, i, i, i, i, i
        );

        let path = if i % 3 == 0 {
            root.join("src").join(format!("mod_{}.rs", i))
        } else {
            root.join(format!("file_{}.rs", i))
        };

        let mut file = File::create(path).unwrap();
        file.write_all(content.as_bytes()).unwrap();
    }

    temp_dir
}

fn full_scan_benchmark(c: &mut Criterion) {
    let temp_10 = create_test_repo(10);
    let temp_50 = create_test_repo(50);
    let temp_100 = create_test_repo(100);

    let mut group = c.benchmark_group("full_scan");
    group.sample_size(20);

    group.bench_function("10_files", |b| {
        b.iter(|| {
            let config = abyss::AbyssConfig {
                path: temp_10.path().to_path_buf(),
                output: PathBuf::from("/dev/null"),
                verbose: false,
                ..Default::default()
            };
            let _ = abyss::run(config);
        })
    });

    group.bench_function("50_files", |b| {
        b.iter(|| {
            let config = abyss::AbyssConfig {
                path: temp_50.path().to_path_buf(),
                output: PathBuf::from("/dev/null"),
                verbose: false,
                ..Default::default()
            };
            let _ = abyss::run(config);
        })
    });

    group.bench_function("100_files", |b| {
        b.iter(|| {
            let config = abyss::AbyssConfig {
                path: temp_100.path().to_path_buf(),
                output: PathBuf::from("/dev/null"),
                verbose: false,
                ..Default::default()
            };
            let _ = abyss::run(config);
        })
    });

    group.finish();
}

criterion_group!(benches, full_scan_benchmark);
criterion_main!(benches);
