//! Rust bindings to the [ESC/POS printer database](https://github.com/receipt-print-hq/escpos-printer-db).

#![deny(missing_docs)]

use std::borrow::Cow;

#[rustfmt::skip]
mod gen;
mod int_map;

pub use crate::gen::*;
pub use crate::int_map::*;

impl Encoding {
    /// This encoding's 7-bit codepage.
    pub fn data(&self) -> Option<&'static [char; 128]> {
        gen::encoding_data(*self)
    }
}

/// A profile with capability information for an ESC/POS printer.
#[derive(Debug)]
#[non_exhaustive]
pub struct Profile<'a> {
    /// The name of this printer.
    pub name: Cow<'a, str>,
    /// The vendor or manufacturer of this printer.
    pub vendor: Cow<'a, str>,
    /// Specific ESC/POS features supported by this printer.
    pub features: Features,
    /// Supported code pages.
    pub code_pages: Cow<'a, IntMap<Encoding>>,
    /// The ink colors supported by this printer.
    pub colors: Cow<'a, IntMap<Color>>,
    /// Information about the character fonts supported by this printer.
    pub fonts: Cow<'a, IntMap<FontInfo>>,
    /// Print media information for this printer.
    pub media: Media,
}

impl<'a> Profile<'a> {
    /// Create a new profile.
    ///
    /// All fields other than `name` and `vendor` are set to their default values.
    pub const fn new(name: Cow<'a, str>, vendor: Cow<'a, str>) -> Self {
        Self {
            name,
            vendor,
            features: Features::new(),
            code_pages: Cow::Borrowed(IntMap::empty()),
            colors: Cow::Borrowed(IntMap::empty()),
            fonts: Cow::Borrowed(IntMap::empty()),
            media: Media::new(None, None),
        }
    }

    /// Set [`Self::features`] to `features`
    pub const fn with_features(mut self, features: Features) -> Self {
        self.features = features;
        self
    }

    /// Set [`Self::code_pages`] to `code_pages`
    // TODO: make const once that's possible
    pub fn with_code_pages(mut self, code_pages: Cow<'a, IntMap<Encoding>>) -> Self {
        self.code_pages = code_pages;
        self
    }

    /// Set [`Self::colors`] to `colors`
    // TODO: make const once that's possible
    pub fn with_colors(mut self, colors: Cow<'a, IntMap<Color>>) -> Self {
        self.colors = colors;
        self
    }

    /// Set [`Self::fonts`] to `fonts`
    // TODO: make const once that's possible
    pub fn with_fonts(mut self, fonts: Cow<'a, IntMap<FontInfo>>) -> Self {
        self.fonts = fonts;
        self
    }

    /// Set [`Self::media`] to `media`
    pub const fn with_media(mut self, media: Media) -> Self {
        self.media = media;
        self
    }
}

/// An ink color supported by a printer profile.
#[derive(Copy, Clone, Debug)]
#[allow(missing_docs)]
pub enum Color {
    Black,
    Red,
    Alternate,
}

/// Information for a supported ESC/POS font.
#[derive(Debug)]
#[non_exhaustive]
pub struct FontInfo {
    /// The maximum number of characters that can fit on a line, using this font.
    pub columns: u8,
}

/// The specific ESC/POS features that are supported by a printer profile.
#[derive(Copy, Clone, Debug, Default)]
pub struct Features(FeaturesInner);

impl Features {
    const fn _with(mut self, flag: FeaturesInner, on: bool) -> Self {
        self.0 = if on {
            self.0.union(flag)
        } else {
            self.0.difference(flag)
        };
        self
    }
}

/// Print media information for a printer profile.
#[derive(Debug, Default)]
#[non_exhaustive]
pub struct Media {
    /// The pixel density of this printer in dots per inch.
    pub dpi: Option<u16>,
    /// The print width of this printer.
    pub width: Option<Width>,
}

impl Media {
    /// Create a new `Media` with the given dpi and width.
    pub const fn new(dpi: Option<u16>, width: Option<Width>) -> Self {
        Self { dpi, width }
    }
}

/// The supported print width for a printer profile.
#[derive(Debug)]
#[non_exhaustive]
pub struct Width {
    /// The print width in millimeters.
    pub mm: f32,
    /// The print width in pixels.
    pub px: u16,
}

impl Width {
    /// Create a new `Width` with the given millimeter and pixel width values.
    pub fn new(mm: f32, px: u16) -> Self {
        Self { mm, px }
    }
}
