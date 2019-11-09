# RESP

RESP(REdis Serialization Protocol) Serialization for Rust.

## API

```Rust
extern crate resp;
use resp::Value;
```

### RESP Values

```Rust
enum Value<'a> {
    /// For Simple Strings the first byte of the reply is "+"
    SimpleString(&'a [u8]),
    /// For Errors the first byte of the reply is "-"
    Error(&'a [u8]),
    /// For Integers the first byte of the reply is ":"
    Integer(i64),
    /// For Bulk Strings the first byte of the reply is "$"
    BulkString(Option<&'a [u8]>),
    /// For Arrays the first byte of the reply is "*"
    Array(Option<Vec<Value<'a>>>),
}
```

#### `value.is_null() -> bool`

#### `value.is_error() -> bool`

### encode


#### `encode(&self, buf: &mut BytesMut) -> usize`

#### `to_vec(&self) -> Vec<u8>`

### Decoder

#### ``parse(buf: &[u8]) -> Result<(&[u8], Value), Error>``
