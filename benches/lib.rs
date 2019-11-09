use criterion::{criterion_group, criterion_main, Criterion};

use resp::{Decoder, Value};
use std::io::BufReader;

fn prepare_values() -> Value {
    let a = vec![
        Value::Null,
        Value::NullArray,
        Value::String("OKOKOKOKOKOKOKOKOKOKOKOKOKOKOKOKOKOKOKOKOKOKOKOKOKOKOKOKOKOKOK".to_string()),
        Value::Error("ErrErrErrErrErrErrErrErrErrErrErrErrErrErrErrErrErrErrErrErrErr".to_string()),
        Value::Integer(1234567890),
        Value::Bulk(
            "Bulk String Bulk String Bulk String Bulk String Bulk String Bulk String".to_string(),
        ),
        Value::Array(vec![
            Value::Null,
            Value::Integer(123),
            Value::Bulk("Bulk String Bulk String".to_string()),
        ]),
    ];
    let mut b = a.clone();
    b.push(Value::Array(a));
    b.push(Value::Null);

    let mut a = b.clone();
    a.push(Value::Array(b));
    a.push(Value::Null);

    Value::Array(a)
}

fn bench_encode_values(c: &mut Criterion) {
    let value = prepare_values();
    c.bench_function("encode_values", |b| b.iter(|| value.encode()));
}

fn bench_decode_values(c: &mut Criterion) {
    let value = prepare_values();
    let buf = value.encode();
    c.bench_function("decode_value", |b| {
        b.iter(|| {
            let mut decoder = Decoder::new(BufReader::new(buf.as_slice()));
            assert_eq!(decoder.decode().unwrap(), value);
            assert!(decoder.decode().is_err());
        })
    });
}

criterion_group!(benches, bench_encode_values, bench_decode_values);
criterion_main!(benches);
