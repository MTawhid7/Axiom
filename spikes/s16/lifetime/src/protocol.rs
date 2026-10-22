//! The messages between the browser stand-in and its helpers, with their
//! payloads in little-endian bytes. In Axiom these would come from the IDL
//! (IP8); here, as in S10, they are written by hand.

/// Message IDs.
pub(crate) mod id {
    /// Helper → browser: its role, with a send right to its port.
    pub(crate) const HELLO: i32 = 1;
    /// Browser → helper: what to do, with any send rights it needs.
    pub(crate) const INTRODUCE: i32 = 2;
    /// Renderer → compositor: one image, its pixels as a memory entry.
    pub(crate) const RESOURCE: i32 = 3;
    /// Renderer → browser: every image is sent.
    pub(crate) const SENT: i32 = 4;
    /// GPU process → browser: every image is registered and drawn once.
    pub(crate) const READY: i32 = 5;
    /// Browser → GPU process: report this task's VM statistics.
    pub(crate) const SAMPLE: i32 = 6;
    /// GPU process → browser: its VM statistics.
    pub(crate) const STATE: i32 = 7;
    /// Browser → GPU process: release every image or tile.
    pub(crate) const RELEASE: i32 = 8;
    /// GPU process → browser: released.
    pub(crate) const RELEASED: i32 = 9;
    /// Browser → helper: exit.
    pub(crate) const QUIT: i32 = 10;
    /// GPU process → browser: its tiles, a send right each.
    pub(crate) const SURFACES: i32 = 11;
    /// Sender → browser: one message carrying one right (S16-Q2).
    pub(crate) const RIGHT: i32 = 12;
    /// Browser → sender: report the no-senders notifications received;
    /// and the sender's answer.
    pub(crate) const COUNT: i32 = 13;
    /// Browser → compositor: read every mapped page from the CPU; and its
    /// answer, with a checksum.
    pub(crate) const TOUCH: i32 = 14;
    pub(crate) const TOUCHED: i32 = 15;
}

/// A helper's role, in `HELLO`.
pub(crate) const ROLE_RENDERER: u8 = 1;
pub(crate) const ROLE_GPU: u8 = 2;
pub(crate) const ROLE_SENDER: u8 = 3;

/// Builds a payload.
#[derive(Debug, Default)]
pub(crate) struct Writer(pub(crate) Vec<u8>);

impl Writer {
    pub(crate) fn u8(&mut self, value: u8) -> &mut Self {
        self.0.push(value);
        self
    }

    pub(crate) fn u32(&mut self, value: u32) -> &mut Self {
        self.0.extend_from_slice(&value.to_le_bytes());
        self
    }

    pub(crate) fn u64(&mut self, value: u64) -> &mut Self {
        self.0.extend_from_slice(&value.to_le_bytes());
        self
    }
}

/// Reads a payload; `None` past its end.
#[derive(Debug)]
pub(crate) struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    pub(crate) fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, at: 0 }
    }

    fn take<const N: usize>(&mut self) -> Option<[u8; N]> {
        let slice = self.bytes.get(self.at..self.at.checked_add(N)?)?;
        self.at += N;
        let mut out = [0; N];
        out.copy_from_slice(slice);
        Some(out)
    }

    pub(crate) fn u8(&mut self) -> Option<u8> {
        Some(self.take::<1>()?[0])
    }

    pub(crate) fn u32(&mut self) -> Option<u32> {
        Some(u32::from_le_bytes(self.take()?))
    }

    pub(crate) fn u64(&mut self) -> Option<u64> {
        Some(u64::from_le_bytes(self.take()?))
    }
}
