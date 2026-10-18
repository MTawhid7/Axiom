//! Font data for the Rust path from CoreText's choices (TX6): the font's file,
//! mapped read-only, or, for a font whose file can't be read (from inside the
//! sandbox, for example), its tables copied through CoreText and assembled
//! into an sfnt.

use std::path::PathBuf;
use std::sync::Arc;

use read_fonts::FileRef;
use s5_rust_text::sfnt::{self, Table};
use skrifa::MetadataProvider;
use skrifa::string::StringId;

use crate::platform::coretext::Font;
use crate::platform::mapping::Mapping;

/// Where a font's data came from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Source {
    /// The font's file, mapped read-only.
    File(PathBuf),
    /// Its tables, copied with `CTFontCopyTable`.
    Tables,
}

/// A font's bytes for read-fonts, skrifa and HarfRust.
#[derive(Clone)]
pub(crate) struct FontData {
    pub(crate) blob: Arc<dyn AsRef<[u8]> + Send + Sync>,
    pub(crate) index: u32,
    pub(crate) source: Source,
}

impl FontData {
    pub(crate) fn bytes(&self) -> &[u8] {
        (*self.blob).as_ref()
    }

    pub(crate) fn font_ref(&self) -> Option<skrifa::FontRef<'_>> {
        skrifa::FontRef::from_index(self.bytes(), self.index).ok()
    }
}

/// The data behind `font`.
pub(crate) fn font_data(font: &Font) -> Result<FontData, String> {
    let name = font.postscript_name();
    if let Some(path) = font.file()
        && let Ok(mapping) = Mapping::open(&path)
    {
        let index = face_index(mapping.as_ref(), &name)
            .ok_or_else(|| format!("{name} is not in {}", path.display()))?;
        return Ok(FontData {
            blob: Arc::new(mapping),
            index,
            source: Source::File(path),
        });
    }
    let tables: Vec<Table> = font
        .tables()
        .into_iter()
        .map(|(tag, data)| Table { tag, data })
        .collect();
    if tables.is_empty() {
        return Err(format!("{name}: no readable file and no tables"));
    }
    let flavor = if tables.iter().any(|t| &t.tag == b"CFF ") {
        sfnt::CFF
    } else {
        sfnt::TRUETYPE
    };
    let bytes = sfnt::write(flavor, tables).map_err(|e| format!("{name}: {e}"))?;
    Ok(FontData {
        blob: Arc::new(bytes),
        index: 0,
        source: Source::Tables,
    })
}

/// The index of the face named `postscript_name` in a font file or
/// collection: by its own PostScript name, or by one of its named instances',
/// since CoreText reports a variable face's instance by the instance's name
/// (`PingFangSC-Regular` is an instance of `.PingFangUITextSC-Default`).
fn face_index(data: &[u8], postscript_name: &str) -> Option<u32> {
    let named = |font: &skrifa::FontRef<'_>, id: StringId| {
        font.localized_strings(id)
            .english_or_first()
            .is_some_and(|s| s.chars().eq(postscript_name.chars()))
    };
    let matches = |font: &skrifa::FontRef<'_>| {
        named(font, StringId::POSTSCRIPT_NAME)
            || font
                .named_instances()
                .iter()
                .filter_map(|instance| instance.postscript_name_id())
                .any(|id| named(font, id))
    };
    match FileRef::new(data).ok()? {
        FileRef::Font(_) => Some(0),
        FileRef::Collection(collection) => {
            (0..collection.len()).find(|&i| collection.get(i).is_ok_and(|font| matches(&font)))
        }
    }
}
