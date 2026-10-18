//! Ryshift engine: every action a wheel slice can run.
//!
//! The engine knows nothing about windows or drags. Give it file paths and an
//! action id, and it writes new files next to the originals and returns them.

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

/// One slice on a wheel.
#[derive(Debug, Clone, Serialize)]
pub struct ActionInfo {
    pub id: String,
    pub title: String,
    pub wheel: Wheel,
}

fn info(id: &str, title: &str, wheel: Wheel) -> ActionInfo {
    ActionInfo { id: id.into(), title: title.into(), wheel }
}

/// Image formats the Convert wheel can write, in slice order.
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

const IMAGE_TOOLS: &[(&str, &str)] = &[
    ("tool.compress", "Compress"),
    ("tool.half", "Half size"),
    ("tool.rotate", "Rotate"),
    ("tool.strip", "Strip metadata"),
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
            for (id, title) in IMAGE_TOOLS {
                out.push(info(id, title, Wheel::Tools));
            }
        }
        Kind::Pdf => {
            for (ext, title) in PDF_TARGETS {
                out.push(info(&format!("convert.{ext}"), title, Wheel::Convert));
            }
        }
        Kind::Other => {}
    }
    out
}

/// The slices to show for these files: only actions that work on every one of them.
pub fn actions_for(paths: &[PathBuf]) -> Vec<ActionInfo> {
    let mut iter = paths.iter().map(|p| actions_for_kind(Kind::of(p)));
    let Some(first) = iter.next() else { return Vec::new() };
    iter.fold(first, |acc, next| {
        acc.into_iter().filter(|a| next.iter().any(|b| b.id == a.id)).collect()
    })
}

/// Run one action on every file. Returns the files it created.
pub fn run(action_id: &str, paths: &[PathBuf]) -> Result<Vec<PathBuf>> {
    let mut created = Vec::new();
    for path in paths {
        created.extend(run_one(action_id, path)?);
    }
    Ok(created)
}

fn run_one(action_id: &str, path: &Path) -> Result<Vec<PathBuf>> {
    let unsupported = || Error::Unsupported(action_id.to_string());
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

/// Total size of the files in bytes, for the wheel's centre.
pub fn total_size(paths: &[PathBuf]) -> u64 {
    paths.iter().filter_map(|p| std::fs::metadata(p).ok()).map(|m| m.len()).sum()
}
