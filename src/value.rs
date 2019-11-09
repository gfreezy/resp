//! RESP Value

use super::serialize::encode;
use crate::parser::parse_resp_value;
use bytes::BytesMut;
use std::vec::Vec;

pub enum Error {
    InvalidData,
    NeedMoreData,
}

pub type Slice<'a> = &'a [u8];

/// Represents a RESP value, see [Redis Protocol specification](http://redis.io/topics/protocol).
#[derive(Debug, PartialEq)]
pub enum Value<'a> {
    SimpleString(Slice<'a>),
    Error(Slice<'a>),
    Integer(i64),
    BulkString(Option<Slice<'a>>),
    Array(Option<Vec<Value<'a>>>),
}

impl<'a> Value<'a> {
    pub fn parse(buf: Slice) -> Result<(Slice, Value), Error> {
        match parse_resp_value(buf) {
            Ok(v) => Ok(v),
            Err(nom::Err::Incomplete(_)) => Err(Error::NeedMoreData),
            Err(nom::Err::Error(_)) | Err(nom::Err::Failure(_)) => Err(Error::InvalidData),
        }
    }

    /// Returns `true` if the value is a `Null` or `NullArray`. Returns `false` otherwise.
    /// # Examples
    /// ```
    /// # use self::resp::{Value};
    /// assert_eq!(Value::Array(None).is_null(), true);
    /// assert_eq!(Value::BulkString(None).is_null(), true);
    /// assert_eq!(Value::Integer(123).is_null(), false);
    /// ```
    pub fn is_null(&self) -> bool {
        match *self {
            Value::Array(None) | Value::BulkString(None) => true,
            _ => false,
        }
    }

    /// Returns `true` if the value is a `Error`. Returns `false` otherwise.
    /// # Examples
    /// ```
    /// # use self::resp::{Value};
    /// assert_eq!(Value::SimpleString(b"aa").is_error(), false);
    /// assert_eq!(Value::Error(b"").is_error(), true);
    /// ```
    pub fn is_error(&self) -> bool {
        match *self {
            Value::Error(_) => true,
            _ => false,
        }
    }

    pub fn encode(&self, buf: &mut BytesMut) -> usize {
        encode(self, buf)
    }

    /// Encode the value to RESP binary buffer.
    /// # Examples
    /// ```
    /// # use self::resp::{Value};
    /// let val = Value::SimpleString("OK正".as_bytes());
    /// assert_eq!(val.to_vec(), vec![43, 79, 75, 230, 173, 163, 13, 10]);
    /// ```
    pub fn to_vec(&self) -> Vec<u8> {
        let mut buf = BytesMut::new();
        self.encode(&mut buf);
        buf.to_vec()
    }

    pub fn serialize_len(&self) -> usize {
        const CRLF_LEN: usize = 2;
        match self {
            Value::SimpleString(s) => 1 + s.len() + CRLF_LEN,
            Value::Error(e) => 1 + e.len() + CRLF_LEN,
            Value::Integer(i) => 1 + i.to_string().len() + CRLF_LEN,
            Value::BulkString(None) => 1 + b"-1".len() + CRLF_LEN,
            Value::BulkString(Some(s)) => {
                1 + s.len().to_string().len() + CRLF_LEN + s.len() + CRLF_LEN
            }
            Value::Array(None) => 1 + b"-1".len() + CRLF_LEN,
            Value::Array(Some(array)) => {
                1 + array.len().to_string().len()
                    + CRLF_LEN
                    + array.iter().map(|s| s.serialize_len()).sum::<usize>()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enum_is_null() {
        assert_eq!(Value::BulkString(None).is_null(), true);
        assert_eq!(Value::Array(None).is_null(), true);
        assert_eq!(Value::SimpleString(b"OK").is_null(), false);
        assert_eq!(Value::Error(b"aa").is_null(), false);
        assert_eq!(Value::Integer(123).is_null(), false);
        assert_eq!(Value::BulkString(Some(b"Bulk")).is_null(), false);
        assert_eq!(
            Value::BulkString(Some(vec![79, 75].as_slice())).is_null(),
            false
        );
        assert_eq!(
            Value::Array(Some(vec![Value::BulkString(None), Value::Integer(123)])).is_null(),
            false
        );
    }

    #[test]
    fn enum_is_error() {
        assert_eq!(Value::BulkString(None).is_error(), false);
        assert_eq!(Value::Array(None).is_error(), false);
        assert_eq!(Value::SimpleString(b"OK").is_error(), false);
        assert_eq!(Value::Error(b"").is_error(), true);
        assert_eq!(Value::Error(b"Err").is_error(), true);
        assert_eq!(Value::Integer(123).is_error(), false);
        assert_eq!(Value::BulkString(Some(b"Bulk")).is_error(), false);
        assert_eq!(
            Value::BulkString(Some(vec![79, 75].as_slice())).is_error(),
            false
        );
        assert_eq!(
            Value::Array(Some(vec![Value::BulkString(None), Value::Integer(123)])).is_error(),
            false
        );
    }

    #[test]
    fn enum_encode_null() {
        let val = Value::BulkString(None);
        assert_eq!(val.to_vec().as_slice(), b"$-1\r\n");
    }

    #[test]
    fn enum_encode_nullarray() {
        let val = Value::Array(None);
        assert_eq!(val.to_vec().as_slice(), b"*-1\r\n");
    }

    #[test]
    fn enum_encode_string() {
        let val = Value::SimpleString("OK正".as_bytes());
        assert_eq!(val.to_vec().as_slice(), "+OK正\r\n".as_bytes());
    }

    #[test]
    fn enum_encode_error() {
        let val = Value::Error(b"error message");
        assert_eq!(val.to_vec().as_slice(), b"-error message\r\n");
    }

    #[test]
    fn enum_encode_integer() {
        let val = Value::Integer(123456789);
        assert_eq!(val.to_vec().as_slice(), b":123456789\r\n");

        let val = Value::Integer(-123456789);
        assert_eq!(val.to_vec().as_slice(), b":-123456789\r\n");
    }

    #[test]
    fn enum_encode_bulk() {
        let val = Value::BulkString(Some("OK正".as_bytes()));
        assert_eq!(val.to_vec().as_slice(), "$5\r\nOK正\r\n".as_bytes());
    }

    #[test]
    fn enum_encode_bufbulk() {
        let val = Value::BulkString(Some(&[79, 75]));
        assert_eq!(&val.to_vec(), b"$2\r\nOK\r\n");
    }

    #[test]
    fn enum_encode_array() {
        let val = Value::Array(Some(Vec::new()));
        assert_eq!(val.to_vec(), b"*0\r\n".to_vec());

        let mut vec: Vec<Value> = Vec::new();
        vec.push(Value::BulkString(None));
        vec.push(Value::Array(None));
        vec.push(Value::SimpleString(b"OK"));
        vec.push(Value::Error(b"message"));
        vec.push(Value::Integer(123456789));
        vec.push(Value::BulkString(Some(b"Hello")));
        let s = vec![79, 75];
        vec.push(Value::BulkString(Some(&s)));
        let val = Value::Array(Some(vec));
        assert_eq!(
            val.to_vec(),
            b"*7\r\n$-1\r\n*-1\r\n+OK\r\n-message\r\n:123456789\r\n$5\r\nHello\r\n\
             $2\r\nOK\r\n"
                .to_vec()
        );
    }
}
