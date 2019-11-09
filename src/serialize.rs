//! RESP serialize

use crate::value::Value;
use bytes::{BufMut, BytesMut};

const CRLF_BYTES: &[u8] = b"\r\n";

/// Encodes RESP value to RESP binary buffer.
/// # Examples
/// ```
/// # use self::resp::Value;
/// let val = Value::SimpleString(b"OK");
/// assert_eq!(val.to_vec().as_slice(), &[43, 79, 75, 13, 10]);
/// ```
pub fn encode(value: &Value, buf: &mut BytesMut) -> usize {
    let initial = buf.len();

    let value_len = value.serialize_len();
    buf.reserve(value_len);
    match value {
        Value::SimpleString(val) => {
            buf.put_u8(b'+');
            buf.put_slice(val);
            buf.put_slice(CRLF_BYTES);
        }
        Value::Error(val) => {
            buf.put_u8(b'-');
            buf.put_slice(val);
            buf.put_slice(CRLF_BYTES);
        }
        Value::Integer(val) => {
            buf.put_u8(b':');
            buf.put_slice(val.to_string().as_bytes());
            buf.put_slice(CRLF_BYTES);
        }
        Value::BulkString(val) => {
            buf.put_u8(b'$');
            match val {
                None => {
                    buf.put_slice(b"-1");
                    buf.put_slice(CRLF_BYTES);
                }
                Some(val) => {
                    buf.put_slice(val.len().to_string().as_bytes());
                    buf.put_slice(CRLF_BYTES);
                    buf.put_slice(val);
                    buf.put_slice(CRLF_BYTES);
                }
            }
        }
        Value::Array(val) => {
            buf.put_u8(b'*');
            match val {
                None => {
                    buf.put_slice(b"-1");
                    buf.put_slice(CRLF_BYTES);
                }
                Some(val) => {
                    buf.put_slice(val.len().to_string().as_bytes());
                    buf.put_slice(CRLF_BYTES);
                    for item in val {
                        encode(item, buf);
                    }
                }
            }
        }
    }

    let len = buf.len() - initial;
    assert_eq!(len, value_len);
    len
}
