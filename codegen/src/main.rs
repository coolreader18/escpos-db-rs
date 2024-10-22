use std::collections::BTreeMap;
use std::env;
use std::fs::File;
use std::io::{self, Write};
use std::path::Path;

use serde::Deserialize;

#[derive(Deserialize)]
struct Db {
    encodings: BTreeMap<String, Encoding>,
    profiles: BTreeMap<String, Profile>,
}

#[derive(Deserialize)]
struct Encoding {
    name: String,
    // iconv: Option<String>,
    // python_encode: Option<String>,
    #[serde(deserialize_with = "deserialize_encoding_data", default)]
    data: Option<Box<[char; 128]>>,
    notes: Option<String>,
}

fn deserialize_encoding_data<'de, D>(deserializer: D) -> Result<Option<Box<[char; 128]>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let data: Option<[String; 8]> = serde::Deserialize::deserialize(deserializer)?;
    let Some(data) = data else { return Ok(None) };
    let mut vec = Vec::with_capacity(128);
    vec.extend(data.iter().flat_map(|s| s.chars()));
    let len = vec.len();
    vec.into_boxed_slice().try_into().map(Some).map_err(|_| {
        serde::de::Error::invalid_length(len, &"an array of 8 strings with 16 characters each")
    })
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Profile {
    code_pages: BTreeMap<u8, String>,
    colors: BTreeMap<u8, String>,
    features: BTreeMap<String, bool>,
    fonts: BTreeMap<u8, FontInfo>,
    media: Media,
    name: String,
    notes: String,
    vendor: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
enum Color {
    Black,
    Red,
    Alternate,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
#[allow(unused)]
struct FontInfo {
    columns: u8,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Media {
    dpi: Option<MaybeUnknown<u16>>,
    width: Width,
}

impl Media {
    fn get_width(&self) -> Option<(f32, u16)> {
        match (self.width.mm.opt(), flatten(self.width.pixels)) {
            (Some(mm), Some(px)) => Some((mm, px)),
            (None, None) => None,
            (Some(mm), None) => {
                let dpi = flatten(self.dpi).unwrap();
                let px = f32::from(dpi) * mm * 25.4;
                Some((mm, px as u16))
            }
            (None, Some(px)) => {
                let dpi = flatten(self.dpi).unwrap();
                let dpmm = f32::from(dpi) / 25.4;
                let mm = f32::from(px) / dpmm;
                Some((mm, px as u16))
            }
        }
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]

struct Width {
    mm: MaybeUnknown<f32>,
    pixels: Option<MaybeUnknown<u16>>,
}

#[derive(Deserialize, Copy, Clone, Debug)]
enum MaybeUnknown<T> {
    Unknown,
    #[serde(untagged)]
    Known(T),
}

impl<T> MaybeUnknown<T> {
    fn opt(self) -> Option<T> {
        match self {
            MaybeUnknown::Unknown => None,
            MaybeUnknown::Known(x) => Some(x),
        }
    }
}

fn flatten<T>(x: Option<MaybeUnknown<T>>) -> Option<T> {
    x.and_then(|x| x.opt())
}

fn write_map<T, W: io::Write>(
    file: &mut W,
    map: &BTreeMap<u8, T>,
    f: impl Fn(&mut W, &T) -> io::Result<()>,
) -> io::Result<()> {
    writeln!(file, "Cow::Borrowed(IntMap::from_entries(&[")?;
    for (k, v) in map {
        write!(file, "      ({k}, ")?;
        f(file, v)?;
        writeln!(file, "),")?;
    }
    write!(file, "    ]))")
}

fn main() -> io::Result<()> {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut file = io::BufWriter::new(File::create(manifest_dir.join("../src/gen.rs"))?);

    writeln!(file, "// GENERATED")?;
    writeln!(file, "// run `cargo run -p codegen` instead")?;
    writeln!(file)?;
    writeln!(file, "use crate::*;\n")?;

    let db = serde_json::from_slice::<Db>(&std::fs::read(
        manifest_dir.join("../escpos-printer-db/dist/capabilities.json"),
    )?)
    .unwrap();

    let encodings = db
        .encodings
        .into_iter()
        .map(|(k, v)| (heck::AsShoutySnakeCase(k), v))
        .collect::<Vec<_>>();

    writeln!(file, "#[derive(Debug, Copy, Clone)]")?;
    writeln!(file, "#[non_exhaustive]")?;
    writeln!(file, "#[allow(non_camel_case_types)]")?;
    writeln!(file, "/// A code page supported by ESC/POS printers.")?;
    writeln!(file, "pub enum Encoding {{")?;
    for (name, enc) in &encodings {
        let mut doc = enc.name.clone();
        if let Some(notes) = &enc.notes {
            doc.push_str("\n\n");
            doc.push_str(notes)
        }
        writeln!(file, "    #[doc = {doc:?}] {name},")?;
    }
    writeln!(file, "}}\n")?;

    writeln!(
        file,
        "pub(crate) fn encoding_data(enc: Encoding) -> Option<&'static [char; 128]> {{"
    )?;
    writeln!(file, "    match enc {{")?;
    for (name, enc) in &encodings {
        if let Some(data) = &enc.data {
            writeln!(file, "        Encoding::{name} => Some(&{data:?}),",)?;
        }
    }
    writeln!(file, "        _ => None,")?;
    writeln!(file, "    }}")?;
    writeln!(file, "}}")?;

    writeln!(
        file,
        "bitflags::bitflags! {{ #[derive(Copy, Clone, Debug, Default)] pub(crate) struct FeaturesInner: u32 {{"
    )?;
    let (_, first_profile) = db.profiles.first_key_value().unwrap();
    for (i, feature) in first_profile.features.keys().enumerate() {
        let feature = heck::AsShoutySnakeCase(feature);
        writeln!(file, "    const {feature} = {};", 1u32 << i)?;
    }
    writeln!(file, "}} }}")?;

    writeln!(file, "impl Features {{")?;
    for feature in first_profile.features.keys() {
        let fn_name = heck::AsSnakeCase(feature);
        let flag_name = heck::AsShoutySnakeCase(feature);
        let doc = format!("Check if the `{feature}` feature is supported.");
        writeln!(file, "    #[doc = {doc:?}]")?;
        writeln!(file, "    pub fn {fn_name}(&self) -> bool {{")?;
        writeln!(file, "        self.0.contains(FeaturesInner::{flag_name})")?;
        writeln!(file, "    }}")?;
    }
    writeln!(file, "}}")?;

    writeln!(file, "impl Features {{")?;
    writeln!(
        file,
        "\
    /// Create a new `Features` with no features enabled.
    pub const fn new() -> Self {{
        Self(FeaturesInner::empty())
    }}"
    )?;
    for feature in first_profile.features.keys() {
        let fn_name = heck::AsSnakeCase(feature);
        let flag_name = heck::AsShoutySnakeCase(feature);
        let doc = format!("Set the `{feature}` feature to be on or off.");
        writeln!(file, "    #[doc = {doc:?}]")?;
        writeln!(file, "    pub fn with_{fn_name}(self, on: bool) -> Self {{")?;
        writeln!(file, "        self._with(FeaturesInner::{flag_name}, on)")?;
        writeln!(file, "    }}")?;
    }
    writeln!(file, "}}")?;

    for (name, profile) in &db.profiles {
        let name = heck::AsShoutySnakeCase(name);
        let doc = [&profile.name, " profile\n\n", &profile.notes].concat();
        writeln!(file, "#[doc = {doc:?}]")?;
        writeln!(file, "pub static {name}: Profile = Profile {{")?;
        writeln!(file, "    name: Cow::Borrowed({:?}),", profile.name)?;
        writeln!(file, "    vendor: Cow::Borrowed({:?}),", profile.vendor)?;

        write!(file, "    features: Features(FeaturesInner::empty()")?;
        for (feature, on) in &profile.features {
            if *on {
                let flag_name = heck::AsShoutySnakeCase(feature);
                write!(file, ".union(FeaturesInner::{flag_name})")?;
            }
        }
        writeln!(file, "),")?;

        write!(file, "    code_pages: ")?;
        write_map(&mut file, &profile.code_pages, |file, page| {
            write!(file, "Encoding::{}", heck::AsShoutySnakeCase(page))
        })?;
        writeln!(file, ",")?;

        write!(file, "    colors: ")?;
        write_map(&mut file, &profile.colors, |file, color| {
            write!(file, "Color::{}", heck::AsPascalCase(color))
        })?;
        writeln!(file, ",")?;

        write!(file, "    fonts: ")?;
        write_map(&mut file, &profile.fonts, |file, font| {
            write!(file, "{font:?}")
        })?;
        writeln!(file, ",")?;

        #[derive(Debug)]
        #[allow(unused)] // used in Debug
        struct Media {
            dpi: Option<u16>,
            width: Option<Width>,
        }
        #[derive(Debug)]
        #[allow(unused)] // used in Debug
        struct Width {
            mm: f32,
            px: u16,
        }
        let media = Media {
            dpi: flatten(profile.media.dpi),
            width: profile.media.get_width().map(|(mm, px)| Width { mm, px }),
        };
        writeln!(file, "    media: {media:?},")?;

        writeln!(file, "}};\n")?;
    }

    let mut map = phf_codegen::Map::new();
    for profile_name in db.profiles.keys() {
        map.entry(
            profile_name,
            &format!("&{}", heck::AsShoutySnakeCase(profile_name)),
        );
    }
    writeln!(
        &mut file,
        "/// A map from the name of each profile in the database to its [`Profile`]."
    )?;
    writeln!(
        &mut file,
        "pub static ALL_PROFILES: phf::Map<&'static str, &'static Profile<'static>> = {};",
        map.build()
    )?;

    file.flush()
}
