//! Utility functions for assisting with conversion of headers from and to the HTTP text form.

use std::io::IoResult;
use rfc2616::is_token;

/// Normalise an HTTP header name.
///
/// Rules:
///
/// - The first character is capitalised
/// - Any character immediately following `-` (HYPHEN-MINUS) is capitalised
/// - All other characters are made lowercase
///
/// This will fail if passed a non-ASCII name.
///
/// # Examples
///
/// ~~~ .{rust}
/// # use http::headers::serialization_utils::normalise_header_name;
/// assert_eq!(normalise_header_name(&String::from_str("foo-bar")), String::from_str("Foo-Bar"));
/// assert_eq!(normalise_header_name(&String::from_str("FOO-BAR")), String::from_str("Foo-Bar"));
/// ~~~
pub fn normalise_header_name(name: &String) -> String {
    let mut result: String = String::with_capacity(name.len());
    let mut capitalise = true;
    for c in name[].chars() {
        let c = match capitalise {
            true => c.to_ascii().to_uppercase(),
            false => c.to_ascii().to_lowercase(),
        };
        result.push(c.to_char());
        // ASCII 45 is '-': in that case, capitalise the next char
        capitalise = c.to_byte() == 45;
    }
    result
}

/// Split a value on commas, as is common for HTTP headers.
///
/// This does not handle quoted-strings intelligently.
///
/// # Examples
///
/// ~~~ .{rust}
/// # use http::headers::serialization_utils::comma_split;
/// assert_eq!(
///     comma_split(" en;q=0.8, en_AU, text/html"),
///     vec![String::from_str("en;q=0.8"), String::from_str("en_AU"), String::from_str("text/html")]
/// )
/// ~~~
pub fn comma_split(value: &str) -> Vec<String> {
    value.split(',').map(|w| String::from_str(w.trim_left())).collect()
}

pub fn comma_split_iter<'a>(value: &'a str)
        -> ::std::iter::Map<'a, &'a str, &'a str, ::std::str::CharSplits<'a, char>> {
    value.split(',').map(|w| w.trim_left())
}

pub trait WriterUtil: Writer {
    fn write_maybe_quoted_string(&mut self, s: &String) -> IoResult<()> {
        if is_token(s) {
            self.write(s.as_bytes())
        } else {
            self.write_quoted_string(s)
        }
    }

    fn write_quoted_string(&mut self, s: &String) -> IoResult<()> {
        try!(self.write(b"\""));
        for b in s.as_bytes().iter() {
            if *b == b'\\' || *b == b'"' {
                try!(self.write(b"\\"));
            }
            // XXX This doesn't seem right.
            try!(self.write(&[*b]));
        }
        self.write(b"\"")
    }

    fn write_parameter(&mut self, k: &str, v: &String) -> IoResult<()> {
        try!(self.write(k.as_bytes()));
        try!(self.write(b"="));
        self.write_maybe_quoted_string(v)
    }

    fn write_parameters(&mut self, parameters: &[(String, String)]) -> IoResult<()> {
        for &(ref k, ref v) in parameters.iter() {
            try!(self.write(b";"));
            try!(self.write_parameter(k[], v));
        }
        Ok(())
    }

    fn write_quality(&mut self, quality: Option<f64>) -> IoResult<()> {
        // TODO: remove second and third decimal places if zero, and use a better quality type anyway
        match quality {
            Some(qvalue) => write!(&mut *self, ";q={:0.3}", qvalue),
            None => Ok(()),
        }
    }

    #[inline]
    fn write_token(&mut self, token: &String) -> IoResult<()> {
        assert!(is_token(token));
        self.write(token.as_bytes())
    }
}

impl<W: Writer> WriterUtil for W { }

/// Join a vector of values with commas, as is common for HTTP headers.
///
/// # Examples
///
/// ~~~ .{rust}
/// # use http::headers::serialization_utils::comma_join;
/// assert_eq!(
///     comma_join(&[String::from_str("en;q=0.8"), String::from_str("en_AU"), String::from_str("text/html")]),
///     String::from_str("en;q=0.8, en_AU, text/html")
/// )
/// ~~~
#[inline]
pub fn comma_join(values: &[String]) -> String {
    let mut out = String::new();
    let mut iter = values.iter();
    match iter.next() {
        Some(s) => out.push_str(s[]),
        None => return out
    }

    for value in iter {
        out.push_str(", ");
        out.push_str(value[]);
    }
    out
}

/// Push a ( token | quoted-string ) onto a string and return it again
pub fn push_maybe_quoted_string(mut s: String, t: &String) -> String {
    if is_token(t) {
        s.push_str(t[]);
        s
    } else {
        push_quoted_string(s, t)
    }
}

/// Make a string into a ( token | quoted-string ), preferring a token
pub fn maybe_quoted_string(s: &String) -> String {
    if is_token(s) {
        s.clone()
    } else {
        quoted_string(s)
    }
}

/// Quote a string, to turn it into an RFC 2616 quoted-string
pub fn push_quoted_string(mut s: String, t: &String) -> String {
    let i = s.len();
    s.reserve(t.len() + i + 2);
    s.push('"');
    for c in t[].chars() {
        if c == '\\' || c == '"' {
            s.push('\\');
        }
        s.push(c);
    }
    s.push('"');
    s
}

/// Quote a string, to turn it into an RFC 2616 quoted-string
pub fn quoted_string(s: &String) -> String {
    push_quoted_string(String::new(), s)
}

/// Parse a quoted-string. Returns ``None`` if the string is not a valid quoted-string.
pub fn unquote_string(s: &String) -> Option<String> {
    enum State { Start, Normal, Escaping, End }

    let mut state = State::Start;
    let mut output = String::new();
    // Strings with escapes cause overallocation, but it's not worth a second pass to avoid this!
    output.reserve(s.len() - 2);
    let mut iter = s[].chars();
    loop {
        state = match (state, iter.next()) {
            (State::Start, Some(c)) if c == '"' => State::Normal,
            (State::Start, Some(_)) => return None,
            (State::Normal, Some(c)) if c == '\\' => State::Escaping,
            (State::Normal, Some(c)) if c == '"' => State::End,
            (State::Normal, Some(c)) | (State::Escaping, Some(c)) => {
                output.push(c);
                State::Normal
            },
            (State::End, Some(_)) => return None,
            (State::End, None) => return Some(output),
            (_, None) => return None,
        }
    }
}

/// Parse a ( token | quoted-string ). Returns ``None`` if it is not valid.
pub fn maybe_unquote_string(s: &String) -> Option<String> {
    if is_token(s) {
        Some(s.clone())
    } else {
        unquote_string(s)
    }
}

// Takes and emits the String instead of the &mut str for a simpler, fluid interface
pub fn push_parameter(mut s: String, k: &String, v: &String) -> String {
    s.push_str(k[]);
    s.push('=');
    push_maybe_quoted_string(s, v)
}

// pub fn push_parameters<K: Str, V: Str>(mut s: String, parameters: &[(K, V)]) -> String {
pub fn push_parameters(mut s: String, parameters: &[(String, String)]) -> String {
    for &(ref k, ref v) in parameters.iter() {
        s.push(';');
        s = push_parameter(s, k, v);
    }
    s
}

#[cfg(test)]
mod test {
    use super::{normalise_header_name, comma_split, comma_split_iter, comma_join,
                push_parameter, push_parameters, push_maybe_quoted_string, push_quoted_string,
                maybe_quoted_string, quoted_string, unquote_string, maybe_unquote_string};

    #[test]
    #[should_fail]
    fn test_normalise_header_name_fail() {
        normalise_header_name(&String::from_str("foö-bar"));
    }

    #[test]
    fn test_normalise_header_name() {
        assert_eq!(normalise_header_name(&String::from_str("foo-bar")), String::from_str("Foo-Bar"));
        assert_eq!(normalise_header_name(&String::from_str("FOO-BAR")), String::from_str("Foo-Bar"));
    }

    #[test]
    fn test_comma_split() {
        // Simple 0-element case
        assert_eq!(comma_split(""), vec!(String::new()));
        // Simple 1-element case
        assert_eq!(comma_split("foo"), vec!(String::from_str("foo")));
        // Simple 2-element case
        assert_eq!(comma_split("foo,bar"), vec!(String::from_str("foo"), String::from_str("bar")));
        // Simple >2-element case
        assert_eq!(comma_split("foo,bar,baz,quux"), vec!(String::from_str("foo"), String::from_str("bar"), String::from_str("baz"), String::from_str("quux")));
        // Doesn't handle quoted-string intelligently
        assert_eq!(comma_split("\"foo,bar\",baz"), vec!(String::from_str("\"foo"), String::from_str("bar\""), String::from_str("baz")));
        // Doesn't do right trimming, but does left
        assert_eq!(comma_split(" foo;q=0.8 , bar/* "), vec!(String::from_str("foo;q=0.8 "), String::from_str("bar/* ")));
    }

    #[test]
    fn test_comma_split_iter() {
        // These are the same cases as in test_comma_split above.
        let s = "";
        assert_eq!(comma_split_iter(s).collect::< Vec<&'static str> >(), vec![""]);
        let s = "foo";
        assert_eq!(comma_split_iter(s).collect::< Vec<&'static str> >(), vec!["foo"]);
        let s = "foo,bar";
        assert_eq!(comma_split_iter(s).collect::< Vec<&'static str> >(), vec!["foo", "bar"]);
        let s = "foo,bar,baz,quux";
        assert_eq!(comma_split_iter(s).collect::< Vec<&'static str> >(), vec!["foo", "bar", "baz", "quux"]);
        let s = "\"foo,bar\",baz";
        assert_eq!(comma_split_iter(s).collect::< Vec<&'static str> >(), vec!["\"foo", "bar\"", "baz"]);
        let s = " foo;q=0.8 , bar/* ";
        assert_eq!(comma_split_iter(s).collect::< Vec<&'static str> >(), vec!["foo;q=0.8 ", "bar/* "]);
    }

    #[test]
    fn test_comma_join() {
        assert_eq!(comma_join(&[String::new()]), String::new());
        assert_eq!(comma_join(&[String::from_str("foo")]), String::from_str("foo"));
        assert_eq!(comma_join(&[String::from_str("foo"), String::from_str("bar")]), String::from_str("foo, bar"));
        assert_eq!(comma_join(&[String::from_str("foo"), String::from_str("bar"), String::from_str("baz"), String::from_str("quux")]), String::from_str("foo, bar, baz, quux"));
        assert_eq!(comma_join(&[String::from_str("\"foo,bar\""), String::from_str("baz")]), String::from_str("\"foo,bar\", baz"));
        assert_eq!(comma_join(&[String::from_str(" foo;q=0.8 "), String::from_str("bar/* ")]), String::from_str(" foo;q=0.8 , bar/* "));
    }

    #[test]
    fn test_push_maybe_quoted_string() {
        assert_eq!(push_maybe_quoted_string(String::from_str("foo,"), &String::from_str("bar")), String::from_str("foo,bar"));
        assert_eq!(push_maybe_quoted_string(String::from_str("foo,"), &String::from_str("bar/baz")), String::from_str("foo,\"bar/baz\""));
    }

    #[test]
    fn test_maybe_quoted_string() {
        assert_eq!(maybe_quoted_string(&String::from_str("bar")), String::from_str("bar"));
        assert_eq!(maybe_quoted_string(&String::from_str("bar/baz \"yay\"")), String::from_str("\"bar/baz \\\"yay\\\"\""));
    }

    #[test]
    fn test_push_quoted_string() {
        assert_eq!(push_quoted_string(String::from_str("foo,"), &String::from_str("bar")), String::from_str("foo,\"bar\""));
        assert_eq!(push_quoted_string(String::from_str("foo,"), &String::from_str("bar/baz \"yay\\\"")),
                   String::from_str("foo,\"bar/baz \\\"yay\\\\\\\"\""));
    }

    #[test]
    fn test_quoted_string() {
        assert_eq!(quoted_string(&String::from_str("bar")), String::from_str("\"bar\""));
        assert_eq!(quoted_string(&String::from_str("bar/baz \"yay\\\"")), String::from_str("\"bar/baz \\\"yay\\\\\\\"\""));
    }

    #[test]
    fn test_unquote_string() {
        assert_eq!(unquote_string(&String::from_str("bar")), None);
        assert_eq!(unquote_string(&String::from_str("\"bar\"")), Some(String::from_str("bar")));
        assert_eq!(unquote_string(&String::from_str("\"bar/baz \\\"yay\\\\\\\"\"")), Some(String::from_str("bar/baz \"yay\\\"")));
        assert_eq!(unquote_string(&String::from_str("\"bar")), None);
        assert_eq!(unquote_string(&String::from_str(" \"bar\"")), None);
        assert_eq!(unquote_string(&String::from_str("\"bar\" ")), None);
        assert_eq!(unquote_string(&String::from_str("\"bar\" \"baz\"")), None);
        assert_eq!(unquote_string(&String::from_str("\"bar/baz \\\"yay\\\\\"\"")), None);
    }

    #[test]
    fn test_maybe_unquote_string() {
        assert_eq!(maybe_unquote_string(&String::from_str("bar")), Some(String::from_str("bar")));
        assert_eq!(maybe_unquote_string(&String::from_str("\"bar\"")), Some(String::from_str("bar")));
        assert_eq!(maybe_unquote_string(&String::from_str("\"bar/baz \\\"yay\\\\\\\"\"")), Some(String::from_str("bar/baz \"yay\\\"")));
        assert_eq!(maybe_unquote_string(&String::from_str("\"bar")), None);
        assert_eq!(maybe_unquote_string(&String::from_str(" \"bar\"")), None);
        assert_eq!(maybe_unquote_string(&String::from_str("\"bar\" ")), None);
        assert_eq!(maybe_unquote_string(&String::from_str("\"bar\" \"baz\"")), None);
        assert_eq!(maybe_unquote_string(&String::from_str("\"bar/baz \\\"yay\\\\\"\"")), None);
    }

    #[test]
    fn test_push_parameter() {
        assert_eq!(push_parameter(String::from_str("foo"), &String::from_str("bar"), &String::from_str("baz")), String::from_str("foobar=baz"));
        assert_eq!(push_parameter(String::from_str("foo"), &String::from_str("bar"), &String::from_str("baz/quux")), String::from_str("foobar=\"baz/quux\""));
    }

    #[test]
    fn test_push_parameters() {
        assert_eq!(push_parameters(String::from_str("foo"), [][]), String::from_str("foo"));
        assert_eq!(push_parameters(String::from_str("foo"), [(String::from_str("bar"), String::from_str("baz"))][]), String::from_str("foo;bar=baz"));
        assert_eq!(push_parameters(String::from_str("foo"), [(String::from_str("bar"), String::from_str("baz/quux"))][]), String::from_str("foo;bar=\"baz/quux\""));
        assert_eq!(push_parameters(String::from_str("foo"), [(String::from_str("bar"), String::from_str("baz")), (String::from_str("quux"), String::from_str("fuzz"))][]),
                   String::from_str("foo;bar=baz;quux=fuzz"));
        assert_eq!(push_parameters(String::from_str("foo"), [(String::from_str("bar"), String::from_str("baz")), (String::from_str("quux"), String::from_str("fuzz zee"))][]),
                   String::from_str("foo;bar=baz;quux=\"fuzz zee\""));
        assert_eq!(push_parameters(String::from_str("foo"), [(String::from_str("bar"), String::from_str("baz/quux")), (String::from_str("fuzz"), String::from_str("zee"))][]),
                   String::from_str("foo;bar=\"baz/quux\";fuzz=zee"));
    }
}
