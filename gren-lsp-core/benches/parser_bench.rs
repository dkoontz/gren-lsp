use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use gren_lsp_core::Parser;

/// Generate Gren source code of varying sizes for benchmarking
fn generate_gren_source(size: usize) -> String {
    let mut source = String::from("module BenchModule exposing (..)\n\nimport Array\n\n");

    for i in 0..size {
        source.push_str(&format!(
            r#"
function{} : Int -> Int
function{} x =
    x * {} + {}

constant{} : Int
constant{} =
    {}
"#,
            i,
            i,
            i + 1,
            i * 2,
            i,
            i,
            i * 3
        ));
    }

    // Add a more complex function at the end
    source.push_str(
        r#"
processNumbers : Array Int -> Array Int
processNumbers nums =
    nums
        |> Array.map (\x -> x * 2)
        |> Array.filter (\x -> x > 10)
        |> Array.take 100

type alias Record =
    { field1 : String
    , field2 : Int
    , field3 : Bool
    }

processRecord : Record -> String
processRecord record =
    if record.field3 then
        record.field1 ++ String.fromInt record.field2
    else
        "default"
"#,
    );

    source
}

/// Benchmark initial parsing performance
fn bench_parse_initial(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse_initial");

    for size in [10, 50, 100, 500].iter() {
        let source = generate_gren_source(*size);

        group.bench_with_input(BenchmarkId::from_parameter(size), &source, |b, source| {
            b.iter(|| {
                let mut parser = Parser::new().expect("Parser creation failed");
                let tree = parser.parse(black_box(source)).expect("Parse failed");
                black_box(tree);
            });
        });
    }

    group.finish();
}

/// Benchmark incremental parsing performance
fn bench_parse_incremental(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse_incremental");

    for size in [10, 50, 100, 500].iter() {
        let original_source = generate_gren_source(*size);
        let modified_source = original_source.replace("x * 2", "x * 3");

        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            &(original_source, modified_source),
            |b, (original, modified)| {
                // Setup: create initial tree
                let mut parser = Parser::new().expect("Parser creation failed");
                let old_tree = parser
                    .parse(original)
                    .expect("Initial parse failed")
                    .unwrap();

                b.iter(|| {
                    let tree = parser
                        .parse_incremental(black_box(modified), Some(&old_tree))
                        .expect("Incremental parse failed");
                    black_box(tree);
                });
            },
        );
    }

    group.finish();
}

/// Benchmark parser initialization
fn bench_parser_creation(c: &mut Criterion) {
    c.bench_function("parser_creation", |b| {
        b.iter(|| {
            let parser = Parser::new().expect("Parser creation failed");
            black_box(parser);
        });
    });
}

/// Benchmark error detection performance
fn bench_error_detection(c: &mut Criterion) {
    let mut group = c.benchmark_group("error_detection");

    // Valid source without errors
    let valid_source = generate_gren_source(100);

    // Invalid source with various errors
    let invalid_source = r#"
module ErrorModule exposing (..)

-- Missing type annotation
invalidFunction = 
    "missing body"

-- Syntax errors
anotherFunction : Int ->
    42

-- Unbalanced parentheses
calculate x =
    (x + 1 * 2
"#;

    group.bench_function("valid_source", |b| {
        let mut parser = Parser::new().expect("Parser creation failed");
        let tree = parser.parse(&valid_source).expect("Parse failed").unwrap();

        b.iter(|| {
            let has_errors = Parser::has_errors(black_box(&tree));
            black_box(has_errors);
        });
    });

    group.bench_function("invalid_source", |b| {
        let mut parser = Parser::new().expect("Parser creation failed");
        let tree = parser
            .parse(&invalid_source)
            .expect("Parse failed")
            .unwrap();

        b.iter(|| {
            let errors = Parser::extract_errors(black_box(&tree));
            black_box(errors);
        });
    });

    group.finish();
}

/// Benchmark parsing of different Gren language constructs
fn bench_language_constructs(c: &mut Criterion) {
    let mut group = c.benchmark_group("language_constructs");

    let simple_function = r#"
module Simple exposing (..)

add : Int -> Int -> Int
add x y = x + y
"#;

    let complex_types = r#"
module ComplexTypes exposing (..)

type alias User = 
    { name : String
    , age : Int
    , email : String
    , preferences : UserPrefs
    }

type UserPrefs
    = DarkMode Bool
    | Theme String
    | Settings (Dict String String)

type Result error value
    = Ok value
    | Err error
"#;

    let pattern_matching = r#"
module PatternMatching exposing (..)

process : Maybe (List String) -> String
process maybeList =
    case maybeList of
        Nothing ->
            "empty"
            
        Just [] ->
            "empty list"
            
        Just [single] ->
            "single: " ++ single
            
        Just (first :: rest) ->
            "first: " ++ first ++ ", rest: " ++ String.fromInt (List.length rest)
"#;

    for (name, source) in [
        ("simple_function", simple_function),
        ("complex_types", complex_types),
        ("pattern_matching", pattern_matching),
    ] {
        group.bench_function(name, |b| {
            b.iter(|| {
                let mut parser = Parser::new().expect("Parser creation failed");
                let tree = parser.parse(black_box(source)).expect("Parse failed");
                black_box(tree);
            });
        });
    }

    group.finish();
}

/// Benchmark memory usage by parsing large files
fn bench_large_files(c: &mut Criterion) {
    let mut group = c.benchmark_group("large_files");
    group.sample_size(10); // Reduce sample size for large benchmarks

    for size in [1000, 2000, 5000].iter() {
        let source = generate_gren_source(*size);

        group.bench_with_input(BenchmarkId::from_parameter(size), &source, |b, source| {
            b.iter(|| {
                let mut parser = Parser::new().expect("Parser creation failed");
                let tree = parser.parse(black_box(source)).expect("Parse failed");

                // Also benchmark error checking on large files
                let has_errors = Parser::has_errors(&tree.as_ref().unwrap());
                black_box((tree, has_errors));
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_parser_creation,
    bench_parse_initial,
    bench_parse_incremental,
    bench_error_detection,
    bench_language_constructs,
    bench_large_files
);

criterion_main!(benches);
