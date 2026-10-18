mod image_ops;
mod kind;
mod naming;
mod pdf_ops;
mod pdf_write;

use serde::Serialize;
use std::path::{Path, PathBuf};

pub use kind::Kind;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0} can't be used on this file")]
    Unsupported(String),
    #[error("couldn't read or write a file: {0}")]
    Io(#[from] std::io::Error),
    #[error("image error: {0}")]
    Image(#[from] image::ImageError),
    #[error("PDF error: {0}")]
    Pdf(String),
    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Wheel {
    Convert,
    Tools,
}

#[derive(Debug, Clone, Serialize)]
pub struct ActionInfo {
    pub id: String,
    pub title: String,
    pub wheel: Wheel,
    pub panel: Option<String>,
}

fn info(id: &str, title: &str, wheel: Wheel) -> ActionInfo {
    ActionInfo { id: id.into(), title: title.into(), wheel, panel: None }
}

fn panel(name: &str, title: &str) -> ActionInfo {
    ActionInfo { id: format!("tool.{name}"), title: title.into(), wheel: Wheel::Tools, panel: Some(name.into()) }
}

const PANELS: &[(&str, &str)] = &[
    ("crop", "Crop"),
    ("edit", "Edit"),
    ("annotate", "Annotate"),
    ("addbg", "Add BG"),
    ("redact", "Redact"),
];

const IMAGE_TARGETS: &[(&str, &str)] = &[
    ("png", "PNG"),
    ("jpg", "JPG"),
    ("webp", "WEBP"),
    ("pdf", "PDF"),
    ("heic", "HEIC"),
    ("tiff", "TIFF"),
    ("bmp", "BMP"),
    ("gif", "GIF"),
];

const IMAGE_TOOLS: &[(&str, &str)] = &[("tool.compress", "Compress"), ("tool.strip", "Metadata")];

const IMAGE_EXTRAS: &[(&str, &str)] = &[
    ("tool.half", "Half size"),
    ("tool.rotate", "Rotate"),
    ("tool.square", "Square 1:1"),
    ("tool.story", "Story 9:16"),
];

const PDF_TARGETS: &[(&str, &str)] = &[("jpg", "JPG"), ("png", "PNG"), ("txt", "TXT")];

fn actions_for_kind(kind: Kind) -> Vec<ActionInfo> {
    let mut out = Vec::new();
    match kind {
        Kind::Image(src) => {
            for (ext, title) in IMAGE_TARGETS {
                if *ext == "heic" && !image_ops::heic_supported() {
                    continue;
                }
                if *ext != src.ext() {
                    out.push(info(&format!("convert.{ext}"), title, Wheel::Convert));
                }
            }
            out.push(panel(PANELS[0].0, PANELS[0].1));
            for (id, title) in IMAGE_TOOLS {
                out.push(info(id, title, Wheel::Tools));
            }
            for (name, title) in &PANELS[1..] {
                out.push(panel(name, title));
            }
        }
        Kind::Pdf => {
            for (ext, title) in PDF_TARGETS {
                out.push(info(&format!("convert.{ext}"), title, Wheel::Convert));
            }
            for (name, title) in PANELS {
                out.push(panel(name, title));
            }
        }
        Kind::Other => {}
    }
    out
}

pub fn actions_for(paths: &[PathBuf]) -> Vec<ActionInfo> {
    let mut iter = paths.iter().map(|p| actions_for_kind(Kind::of(p)));
    let Some(first) = iter.next() else { return Vec::new() };
    let shared = iter.fold(first, |acc, next| {
        acc.into_iter().filter(|a| next.iter().any(|b| b.id == a.id)).collect::<Vec<_>>()
    });
    if paths.len() > 1 {
        return shared.into_iter().filter(|a| a.panel.is_none()).collect();
    }
    shared
}

pub fn all_actions_for(paths: &[PathBuf]) -> Vec<ActionInfo> {
    let mut all: Vec<ActionInfo> = actions_for(paths).into_iter().filter(|a| a.panel.is_none()).collect();
    if !paths.is_empty() && paths.iter().all(|p| matches!(Kind::of(p), Kind::Image(_))) {
        all.extend(IMAGE_EXTRAS.iter().map(|(id, title)| info(id, title, Wheel::Tools)));
    }
    all
}

pub struct EditSource {
    pub png: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub out_ext: &'static str,
}

pub fn load_for_edit(path: &Path) -> Result<EditSource> {
    let (img, out_ext) = match Kind::of(path) {
        Kind::Image(f) => {
            let ext = if matches!(f, image_ops::Format::Jpg | image_ops::Format::Heic) { "jpg" } else { "png" };
            (image_ops::load(path)?, ext)
        }
        Kind::Pdf => (pdf_ops::render_page(path, 0)?, "jpg"),
        Kind::Other => return Err(Error::Unsupported("editing".into())),
    };
    Ok(EditSource { png: image_ops::png_bytes(&img)?, width: img.width(), height: img.height(), out_ext })
}

pub fn save_edit(original: &Path, suffix: &str, ext: &str, bytes: &[u8]) -> Result<PathBuf> {
    if !matches!(ext, "png" | "jpg") || suffix.is_empty() || suffix.contains(['/', '\\']) {
        return Err(Error::Other(format!("can't save as {suffix:?}.{ext}")));
    }
    let out = naming::sibling(original, Some(suffix), ext);
    std::fs::write(&out, bytes)?;
    Ok(out)
}

pub fn run(action_id: &str, paths: &[PathBuf]) -> Result<Vec<PathBuf>> {
    let mut created = Vec::new();
    for path in paths {
        created.extend(run_one(action_id, path)?);
    }
    Ok(created)
}

fn run_one(action_id: &str, path: &Path) -> Result<Vec<PathBuf>> {
    let unsupported = || Error::Unsupported(action_id.to_string());
    if let Some(name) = action_id.strip_prefix("tool.").filter(|n| PANELS.iter().any(|(p, _)| p == n)) {
        return Err(Error::Other(format!("{name} needs its editor: open it from the wheel")));
    }
    match (Kind::of(path), action_id.split_once('.')) {
        (Kind::Image(_), Some(("convert", "pdf"))) => {
            let img = image_ops::load(path)?;
            let out = naming::sibling(path, None, "pdf");
            pdf_write::write_images(&[img], &out)?;
            Ok(vec![out])
        }
        (Kind::Image(_), Some(("convert", ext))) => {
            let target = image_ops::Format::from_ext(ext).ok_or_else(unsupported)?;
            let img = image_ops::load(path)?;
            let out = naming::sibling(path, None, target.ext());
            image_ops::save(&img, &out, target, 90)?;
            Ok(vec![out])
        }
        (Kind::Image(src), Some(("tool", tool))) => {
            let out = image_ops::tool(path, src, tool)?;
            Ok(vec![out])
        }
        (Kind::Pdf, Some(("convert", "txt"))) => Ok(vec![pdf_ops::to_text(path)?]),
        (Kind::Pdf, Some(("convert", ext))) => {
            let target = image_ops::Format::from_ext(ext).ok_or_else(unsupported)?;
            pdf_ops::to_images(path, target)
        }
        _ => Err(unsupported()),
    }
}

pub fn total_size(paths: &[PathBuf]) -> u64 {
    paths.iter().filter_map(|p| std::fs::metadata(p).ok()).map(|m| m.len()).sum()
}
