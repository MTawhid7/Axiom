//! A minimal HTTP/1.1 client side, enough to fetch: a GET request, and a
//! response parser that takes bytes as they arrive. Framing is strict, as
//! NW3 asks of Axiom's own: conflicting lengths, malformed chunk sizes and
//! oversized heads are errors. Bodies are counted, not kept, apart from a
//! short sample. Throwaway: Axiom's HTTP/1.1 is designed later.

/// The largest response head accepted.
const MAX_HEAD: usize = 64 * 1024;
/// How much of the body is kept, for checks.
const SAMPLE: usize = 4096;

/// A GET request for `path` on `host`.
pub(crate) fn request(host: &str, path: &str, keep_alive: bool) -> Vec<u8> {
    let connection = if keep_alive { "keep-alive" } else { "close" };
    format!(
        "GET {path} HTTP/1.1\r\nHost: {host}\r\nUser-Agent: axiom-s7-spike\r\nAccept: */*\r\nAccept-Encoding: identity\r\nConnection: {connection}\r\n\r\n"
    )
    .into_bytes()
}

/// How the body is delimited.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Body {
    /// Still reading the head.
    Head,
    Length(u64),
    Chunked(Chunk),
    UntilClose,
    Done,
}

/// The chunked decoder's position.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Chunk {
    /// Reading a chunk size; `ext` once past a `;`.
    Size {
        value: u64,
        digits: u32,
        ext: bool,
    },
    SizeLf {
        value: u64,
    },
    Data(u64),
    DataCr,
    DataLf,
    /// Reading trailer lines; `empty` while the current line is empty.
    Trailer {
        empty: bool,
    },
    TrailerLf {
        empty: bool,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Response {
    head: Vec<u8>,
    body: Body,
    pub(crate) status: Option<u16>,
    pub(crate) body_bytes: u64,
    /// The first bytes of the body.
    pub(crate) sample: Vec<u8>,
    /// The server will close the connection after this response.
    pub(crate) close: bool,
}

impl Default for Response {
    fn default() -> Self {
        Self {
            head: Vec::new(),
            body: Body::Head,
            status: None,
            body_bytes: 0,
            sample: Vec::new(),
            close: false,
        }
    }
}

impl Response {
    pub(crate) fn complete(&self) -> bool {
        self.body == Body::Done
    }

    /// Take the next bytes from the connection.
    pub(crate) fn feed(&mut self, mut data: &[u8]) -> Result<(), String> {
        while !data.is_empty() {
            match self.body {
                Body::Head => data = self.feed_head(data)?,
                Body::Done => return Err("bytes after the response's end".to_owned()),
                Body::Length(remaining) => {
                    let n = take(remaining, data.len());
                    self.count(&data[..n]);
                    data = &data[n..];
                    self.body = if remaining == as_u64(n) {
                        Body::Done
                    } else {
                        Body::Length(remaining - as_u64(n))
                    };
                }
                Body::UntilClose => {
                    self.count(data);
                    data = &[];
                }
                Body::Chunked(chunk) => data = self.feed_chunked(chunk, data)?,
            }
        }
        Ok(())
    }

    /// The connection ended: complete if the body runs until close.
    pub(crate) fn end(&mut self) -> Result<(), String> {
        match self.body {
            Body::UntilClose | Body::Done => {
                self.body = Body::Done;
                Ok(())
            }
            _ => Err("the connection ended before the response did".to_owned()),
        }
    }

    fn count(&mut self, bytes: &[u8]) {
        self.body_bytes += as_u64(bytes.len());
        let room = SAMPLE.saturating_sub(self.sample.len());
        self.sample
            .extend_from_slice(&bytes[..room.min(bytes.len())]);
    }

    fn feed_head<'a>(&mut self, data: &'a [u8]) -> Result<&'a [u8], String> {
        let searched = self.head.len().saturating_sub(3);
        self.head.extend_from_slice(data);
        let Some(end) = find(&self.head[searched..], b"\r\n\r\n").map(|i| searched + i + 4) else {
            if self.head.len() > MAX_HEAD {
                return Err("the response head is too large".to_owned());
            }
            return Ok(&[]);
        };
        let rest = self.head.len() - end;
        let head = std::mem::take(&mut self.head);
        let status = self.parse_head(&head[..end])?;
        if (100..200).contains(&status) {
            // An interim response; the final one follows.
            self.body = Body::Head;
            self.close = false;
        } else {
            self.status = Some(status);
        }
        Ok(&data[data.len() - rest..])
    }

    /// Parse a head; set the body's framing. Returns the status.
    fn parse_head(&mut self, head: &[u8]) -> Result<u16, String> {
        let text = std::str::from_utf8(head).map_err(|_| "the head is not UTF-8".to_owned())?;
        let mut lines = text.split("\r\n");
        let status_line = lines.next().unwrap_or_default();
        let mut parts = status_line.splitn(3, ' ');
        let (version, code) = (parts.next(), parts.next());
        if version != Some("HTTP/1.1") && version != Some("HTTP/1.0") {
            return Err(format!("unexpected status line {status_line:?}"));
        }
        let status: u16 = code
            .filter(|c| c.len() == 3 && c.bytes().all(|b| b.is_ascii_digit()))
            .and_then(|c| c.parse().ok())
            .ok_or_else(|| format!("unexpected status line {status_line:?}"))?;
        let (mut length, mut chunked, mut coded) = (None, false, false);
        self.close = version == Some("HTTP/1.0");
        for line in lines.take_while(|l| !l.is_empty()) {
            let (name, value) = line
                .split_once(':')
                .ok_or_else(|| format!("malformed field line {line:?}"))?;
            if name.is_empty() || name.bytes().any(|b| b.is_ascii_whitespace()) {
                return Err(format!("malformed field name {name:?}"));
            }
            let value = value.trim_matches([' ', '\t']);
            if name.eq_ignore_ascii_case("content-length") {
                let n: u64 = value
                    .parse()
                    .map_err(|_| format!("bad Content-Length {value:?}"))?;
                if length.is_some_and(|l| l != n) {
                    return Err("conflicting Content-Length fields".to_owned());
                }
                length = Some(n);
            } else if name.eq_ignore_ascii_case("transfer-encoding") {
                coded = true;
                chunked = value
                    .rsplit(',')
                    .next()
                    .is_some_and(|c| c.trim().eq_ignore_ascii_case("chunked"));
            } else if name.eq_ignore_ascii_case("connection") {
                self.close |= value
                    .split(',')
                    .any(|t| t.trim().eq_ignore_ascii_case("close"));
            }
        }
        if coded && length.is_some() {
            return Err("both Transfer-Encoding and Content-Length".to_owned());
        }
        self.body = if (100..200).contains(&status) {
            Body::Head
        } else if status == 204 || status == 304 {
            Body::Done
        } else if chunked {
            Body::Chunked(Chunk::Size {
                value: 0,
                digits: 0,
                ext: false,
            })
        } else if coded {
            Body::UntilClose
        } else {
            match length {
                Some(0) => Body::Done,
                Some(n) => Body::Length(n),
                None => Body::UntilClose,
            }
        };
        Ok(status)
    }

    fn feed_chunked<'a>(&mut self, chunk: Chunk, data: &'a [u8]) -> Result<&'a [u8], String> {
        if let Chunk::Data(remaining) = chunk {
            let n = take(remaining, data.len());
            self.count(&data[..n]);
            self.body = Body::Chunked(if remaining == as_u64(n) {
                Chunk::DataCr
            } else {
                Chunk::Data(remaining - as_u64(n))
            });
            return Ok(&data[n..]);
        }
        let byte = data[0];
        let bad = |what: &str| Err(format!("malformed chunked body: {what}"));
        let next = match chunk {
            Chunk::Size { value, digits, ext } => match byte {
                b'\r' if digits > 0 => Chunk::SizeLf { value },
                b';' if digits > 0 => Chunk::Size {
                    value,
                    digits,
                    ext: true,
                },
                _ if ext => chunk,
                _ => {
                    let Some(d) = char::from(byte).to_digit(16) else {
                        return bad("a size is not hexadecimal");
                    };
                    if digits >= 15 {
                        return bad("a size is too large");
                    }
                    Chunk::Size {
                        value: value * 16 + u64::from(d),
                        digits: digits + 1,
                        ext,
                    }
                }
            },
            Chunk::SizeLf { value } if byte == b'\n' => {
                if value == 0 {
                    Chunk::Trailer { empty: true }
                } else {
                    Chunk::Data(value)
                }
            }
            Chunk::DataCr if byte == b'\r' => Chunk::DataLf,
            Chunk::DataLf if byte == b'\n' => Chunk::Size {
                value: 0,
                digits: 0,
                ext: false,
            },
            Chunk::Trailer { empty } => match byte {
                b'\r' => Chunk::TrailerLf { empty },
                _ => Chunk::Trailer { empty: false },
            },
            Chunk::TrailerLf { empty } if byte == b'\n' => {
                if empty {
                    self.body = Body::Done;
                    return Ok(&data[1..]);
                }
                Chunk::Trailer { empty: true }
            }
            _ => return bad("a line ending is missing"),
        };
        self.body = Body::Chunked(next);
        Ok(&data[1..])
    }
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

/// The bytes to take of `available`, with `remaining` left in the body.
fn take(remaining: u64, available: usize) -> usize {
    usize::try_from(remaining).map_or(available, |r| r.min(available))
}

fn as_u64(n: usize) -> u64 {
    u64::try_from(n).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(chunks: &[&[u8]]) -> Result<Response, String> {
        let mut r = Response::default();
        for c in chunks {
            r.feed(c)?;
        }
        Ok(r)
    }

    #[test]
    fn lengths_chunks_and_closes_delimit_bodies() {
        let r = parse(&[
            b"HTTP/1.1 200 OK\r\nContent-Le",
            b"ngth: 5\r\n\r\nhel",
            b"lo",
        ])
        .unwrap();
        assert_eq!((r.status, r.body_bytes, r.complete()), (Some(200), 5, true));
        assert_eq!(r.sample, b"hello");

        let chunked = b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n4;x=y\r\nwiki\r\n5\r\npedia\r\n0\r\nT: 1\r\n\r\n";
        for split in 1..chunked.len() {
            let r = parse(&[&chunked[..split], &chunked[split..]]).unwrap();
            assert_eq!((r.body_bytes, r.complete()), (9, true), "split at {split}");
            assert_eq!(r.sample, b"wikipedia");
        }

        let mut r =
            parse(&[b"HTTP/1.1 103 Early Hints\r\nLink: </a>\r\n\r\nHTTP/1.1 200 OK\r\n\r\nabc"])
                .unwrap();
        assert_eq!((r.status, r.complete()), (Some(200), false));
        r.end().unwrap();
        assert_eq!((r.body_bytes, r.complete()), (3, true));

        let r = parse(&[b"HTTP/1.1 204 No Content\r\nConnection: close\r\n\r\n"]).unwrap();
        assert!(r.complete() && r.close);
    }

    #[test]
    fn ambiguous_framing_is_rejected() {
        for bad in [
            &b"HTTP/1.1 200 OK\r\nContent-Length: 1\r\nContent-Length: 2\r\n\r\n"[..],
            b"HTTP/1.1 200 OK\r\nContent-Length: 1\r\nTransfer-Encoding: chunked\r\n\r\n",
            b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\nzz\r\n",
            b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n1\r\nab",
            b"HTTP/2 200\r\n\r\n",
            b"HTTP/1.1 200 OK\r\nBad Name: x\r\n\r\n",
        ] {
            assert!(parse(&[bad]).is_err(), "{}", String::from_utf8_lossy(bad));
        }
        let mut r = parse(&[b"HTTP/1.1 200 OK\r\nContent-Length: 9\r\n\r\nabc"]).unwrap();
        assert!(r.end().is_err());
    }
}
