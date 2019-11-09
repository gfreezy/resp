use super::value::{Slice, Value};
use nom::bytes::streaming::{tag, take};
use nom::character::streaming::{crlf, digit1};
use nom::combinator::{map_res, opt};
use nom::error::ErrorKind;
use nom::multi::many_m_n;
use nom::sequence::{pair, preceded, terminated};
use nom::IResult;

fn not_crlf(buf: Slice) -> IResult<Slice, Slice> {
    let mut index = 0;
    for i in 0..(buf.len() - 1) {
        if buf[i] == b'\r' && buf[i + 1] == b'\n' {
            index = i;
            break;
        }
    }
    Ok((&buf[index..], &buf[0..index]))
}

fn simple_string(buf: Slice) -> IResult<Slice, Slice> {
    preceded(tag(b"+"), terminated(not_crlf, crlf))(buf)
}

fn error(buf: Slice) -> IResult<Slice, Slice> {
    preceded(tag(b"-"), terminated(not_crlf, crlf))(buf)
}

fn parse_integer((flag, num): (Option<Slice>, Slice)) -> Result<i64, ()> {
    let s = if let Some(flag) = flag {
        [flag, num].concat()
    } else {
        num.to_vec()
    };
    std::str::from_utf8(&s).or(Err(()))?.parse().or(Err(()))
}

fn string_integer(buf: Slice) -> IResult<Slice, i64> {
    map_res(pair(opt(tag(b"-")), digit1), parse_integer)(buf)
}

fn integer(buf: Slice) -> IResult<Slice, i64> {
    preceded(tag(b":"), terminated(string_integer, crlf))(buf)
}

fn bulk_string(buf: Slice) -> IResult<Slice, Option<Slice>> {
    let (left, _) = tag(b"$")(buf)?;
    let (left, size) = string_integer(left)?;
    let (left, _) = crlf(left)?;
    if size >= 0 {
        let (left, bulk_str) = take(size as usize)(left)?;
        let (left, _) = crlf(left)?;
        Ok((left, Some(bulk_str)))
    } else {
        Ok((left, None))
    }
}

fn array(buf: Slice) -> IResult<Slice, Option<Vec<Value>>> {
    let (left, _) = tag(b"*")(buf)?;
    let (left, size) = string_integer(left)?;
    let (left, _) = crlf(left)?;

    if size < 0 {
        Ok((left, None))
    } else if size == 0 {
        Ok((left, Some(Vec::new())))
    } else {
        let (left, v): (Slice, Vec<Value>) =
            many_m_n(size as usize, size as usize, resp_value)(left)?;
        Ok((left, Some(v)))
    }
}

fn resp_value(buf: Slice) -> IResult<Slice, Value> {
    if let Ok((left, output)) = simple_string(buf) {
        Ok((left, Value::SimpleString(output)))
    } else if let Ok((left, output)) = error(buf) {
        Ok((left, Value::Error(output)))
    } else if let Ok((left, output)) = integer(buf) {
        Ok((left, Value::Integer(output)))
    } else if let Ok((left, output)) = bulk_string(buf) {
        Ok((left, Value::BulkString(output)))
    } else if let Ok((left, output)) = array(buf) {
        Ok((left, Value::Array(output)))
    } else {
        Err(nom::Err::Error((buf, ErrorKind::Alt)))
    }
}

pub fn parse_resp_value(buf: Slice) -> IResult<Slice, Value> {
    resp_value(buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_string() {
        assert!(simple_string(b"as").is_err());
        assert!(simple_string(b"+as").is_err());
        assert!(simple_string(b"+as\r").is_err());
        assert_eq!(simple_string(b"+\r\n"), Ok((&[][..], &b""[..])));
        assert_eq!(simple_string(b"+as\r\n"), Ok((&[][..], &b"as"[..])));
        assert_eq!(simple_string(b"+as\r\r\n"), Ok((&[][..], &b"as\r"[..])));
        assert_eq!(simple_string(b"++as\r\r\n"), Ok((&[][..], &b"+as\r"[..])));
        assert_eq!(
            simple_string(b"++as\r\nsdf\r\n"),
            Ok((&b"sdf\r\n"[..], &b"+as"[..]))
        );
    }

    #[test]
    fn test_parse_integer() {
        assert!(parse_integer((Some(b"-"), b"as")).is_err());
        assert_eq!(parse_integer((Some(b"-"), b"10")), Ok(-10));
        assert_eq!(parse_integer((Some(b"-"), b"0")), Ok(0));
        assert_eq!(parse_integer((None, b"0")), Ok(0));
        assert_eq!(parse_integer((None, b"10")), Ok(10));
    }

    #[test]
    fn test_string_integer() {
        assert!(string_integer(b"as").is_err());
        assert_eq!(string_integer(b"10a"), Ok((b"a".as_ref(), 10)));
        assert_eq!(string_integer(b"-10d"), Ok((b"d".as_ref(), -10)));
    }

    #[test]
    fn test_integer() {
        assert!(integer(b"as").is_err());
        assert!(integer(b"+as").is_err());
        assert!(integer(b"+as\r").is_err());
        assert!(integer(b"+1as\r").is_err());
        assert_eq!(integer(b":1\r\n"), Ok((&[][..], 1)));
        assert_eq!(integer(b":31\r\n"), Ok((&[][..], 31)));
        assert_eq!(integer(b":-31\r\n"), Ok((&[][..], -31)));
        assert_eq!(integer(b":0\r\n"), Ok((&[][..], 0)));
    }

    #[test]
    fn test_bulk_string() {
        assert!(bulk_string(b"$0as").is_err());
        assert!(bulk_string(b"$0\r\n").is_err());
        assert!(bulk_string(b"$1\r\nas\r\n").is_err());
        assert_eq!(
            bulk_string(b"$0\r\n\r\n"),
            Ok((b"".as_ref(), Some(b"".as_ref())))
        );
        assert_eq!(bulk_string(b"$-1\r\n\r\n"), Ok((b"\r\n".as_ref(), None)));
        assert_eq!(
            bulk_string(b"$1\r\na\r\na"),
            Ok((b"a".as_ref(), Some(b"a".as_ref())))
        );
        assert_eq!(
            bulk_string(b"$1\r\na\r\n"),
            Ok((b"".as_ref(), Some(b"a".as_ref())))
        );
    }

    #[test]
    fn test_array() {
        assert!(array(b"$0as").is_err());
        assert_eq!(array(b"*0\r\n"), Ok((b"".as_ref(), Some(Vec::new()))));
        assert_eq!(array(b"*-1\r\n"), Ok((b"".as_ref(), None)));
        assert_eq!(
            array(b"*0\r\n\r\n"),
            Ok((b"\r\n".as_ref(), Some(Vec::new())))
        );
        assert_eq!(
            array(b"*2\r\n$3\r\nfoo\r\n$3\r\nbar\r\n"),
            Ok((
                b"".as_ref(),
                Some(vec![
                    Value::BulkString(Some(b"foo".as_ref())),
                    Value::BulkString(Some(b"bar".as_ref()))
                ])
            ))
        );
        assert_eq!(
            array(b"*5\r\n:1\r\n:2\r\n:3\r\n:4\r\n$6\r\nfoobar\r\n"),
            Ok((
                b"".as_ref(),
                Some(vec![
                    Value::Integer(1),
                    Value::Integer(2),
                    Value::Integer(3),
                    Value::Integer(4),
                    Value::BulkString(Some(b"foobar".as_ref()))
                ])
            ))
        );
        assert_eq!(
            array(b"*2\r\n*3\r\n:1\r\n:2\r\n:3\r\n*2\r\n+Foo\r\n-Bar\r\n"),
            Ok((
                b"".as_ref(),
                Some(vec![
                    Value::Array(Some(vec![
                        Value::Integer(1),
                        Value::Integer(2),
                        Value::Integer(3),
                    ])),
                    Value::Array(Some(vec![
                        Value::SimpleString(b"Foo".as_ref()),
                        Value::Error(b"Bar".as_ref())
                    ]))
                ])
            ))
        );
        assert_eq!(
            array(b"*3\r\n$3\r\nfoo\r\n$-1\r\n$3\r\nbar\r\n"),
            Ok((
                b"".as_ref(),
                Some(vec![
                    Value::BulkString(Some(b"foo".as_ref())),
                    Value::BulkString(None),
                    Value::BulkString(Some(b"bar".as_ref())),
                ])
            ))
        );
    }
}
