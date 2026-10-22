//! The messages between the browser, the GPU process and the renderer, with
//! their payloads in little-endian bytes. In Axiom these would come from the
//! IDL (IP8); here they are written by hand.

/// Message IDs.
pub(crate) mod id {
    /// Helper → browser: its role, with a send right to its port.
    pub(crate) const HELLO: i32 = 1;
    /// Browser → renderer: the run's settings, and a send right to the GPU
    /// process.
    pub(crate) const INTRODUCE: i32 = 2;
    /// Browser → GPU process: the run's settings.
    pub(crate) const CONFIG: i32 = 3;
    /// Renderer → GPU process: a resource, its pixels as a memory entry or an
    /// IOSurface.
    pub(crate) const RESOURCE: i32 = 4;
    /// GPU process → browser: every resource is registered, and drawn once.
    pub(crate) const RESOURCES_READY: i32 = 5;
    /// Browser → renderer: a frame offer (EL21).
    pub(crate) const OFFER: i32 = 6;
    /// Renderer → GPU process: a display-list update, out of line.
    pub(crate) const UPDATE: i32 = 7;
    /// GPU process → browser: the layer commit, with new surfaces' rights.
    pub(crate) const COMMIT: i32 = 8;
    /// Browser → helper: finish, and report.
    pub(crate) const QUIT: i32 = 9;
    /// Helper → browser: its statistics.
    pub(crate) const STATS: i32 = 10;
    /// Renderer → GPU process: the whole page's display list, out of line,
    /// once; the GPU process keeps it, as it would to raster tiles that
    /// scroll into view.
    pub(crate) const PAGE: i32 = 11;
}

/// The helper's role, in `HELLO`.
pub(crate) const ROLE_GPU: u8 = 1;
pub(crate) const ROLE_RENDERER: u8 = 2;

/// How the GPU process reaches a renderer's pixels (S10-Q2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Path {
    /// A buffer over the mapped memory entry, without a copy.
    NoCopy,
    /// One blit from the mapped memory entry into a private texture.
    Blit,
    /// The renderer's own IOSurface, which RV1-09 removes.
    Surface,
}

impl Path {
    pub(crate) fn code(self) -> u8 {
        match self {
            Self::NoCopy => 0,
            Self::Blit => 1,
            Self::Surface => 2,
        }
    }

    pub(crate) fn from_code(code: u8) -> Option<Self> {
        match code {
            0 => Some(Self::NoCopy),
            1 => Some(Self::Blit),
            2 => Some(Self::Surface),
            _ => None,
        }
    }

    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::NoCopy => "no-copy",
            Self::Blit => "blit",
            Self::Surface => "iosurface",
        }
    }
}

/// The run's settings, sent to both helpers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Config {
    /// 0: the hop (S10-Q1); 1: resources (S10-Q2).
    pub(crate) mode: u8,
    pub(crate) path: Path,
    /// Tile side and grid, in pixels and tiles.
    pub(crate) side: u32,
    pub(crate) columns: u32,
    pub(crate) rows: u32,
    /// Items on the page, for the hop.
    pub(crate) items: u32,
    /// Images, and their side, for resources.
    pub(crate) images: u32,
    pub(crate) image_side: u32,
}

impl Config {
    pub(crate) fn encode(&self) -> Vec<u8> {
        let mut w = Writer::default();
        w.u8(self.mode).u8(self.path.code());
        for value in [
            self.side,
            self.columns,
            self.rows,
            self.items,
            self.images,
            self.image_side,
        ] {
            w.u32(value);
        }
        w.0
    }

    pub(crate) fn decode(bytes: &[u8]) -> Option<Self> {
        let mut r = Reader::new(bytes);
        Some(Self {
            mode: r.u8()?,
            path: Path::from_code(r.u8()?)?,
            side: r.u32()?,
            columns: r.u32()?,
            rows: r.u32()?,
            items: r.u32()?,
            images: r.u32()?,
            image_side: r.u32()?,
        })
    }
}

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

    pub(crate) fn f64(&mut self, value: f64) -> &mut Self {
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

    pub(crate) fn f64(&mut self) -> Option<f64> {
        Some(f64::from_le_bytes(self.take()?))
    }
}
