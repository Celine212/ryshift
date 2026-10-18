//! Reading PDFs through Pdfium (the PDF engine inside Chrome). The library is
//! a separate download (`scripts/fetch-pdfium.sh`) bundled next to the app.

use crate::image_ops::{self, Format};
use crate::{naming, Error, Result};
use pdfium_render::prelude::*;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

fn pdf_err(e: PdfiumError) -> Error {
    Error::Pdf(e.to_string())
}

/// Folders to look in for the Pdfium library, most specific first.
fn search_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(p) = std::env::var_os("SHIFTWHEEL_PDFIUM") {
        dirs.push(PathBuf::from(p));
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            dirs.push(dir.to_path_buf());
            dirs.push(dir.join("lib"));
            dirs.push(dir.join("../lib"));
            dirs.push(dir.join("../Resources/lib")); // inside a macOS .app
            dirs.push(dir.join("../lib/shiftwheel")); // Linux packages
        }
    }
    dirs.push(PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../lib"))); // dev checkout
    dirs
}

fn pdfium() -> Result<&'static Pdfium> {
    static PDFIUM: OnceLock<std::result::Result<Pdfium, String>> = OnceLock::new();
    PDFIUM
        .get_or_init(|| {
            for dir in search_dirs() {
                let lib = Pdfium::pdfium_platform_library_name_at_path(&dir);
                if lib.exists() {
                    if let Ok(bindings) = Pdfium::bind_to_library(&lib) {
                        return Ok(Pdfium::new(bindings));
                    }
                }
            }
            Pdfium::bind_to_system_library()
                .map(Pdfium::new)
                .map_err(|_| "Pdfium library not found. Run scripts/fetch-pdfium.sh".to_string())
        })
        .as_ref()
        .map_err(|e| Error::Pdf(e.clone()))
}

/// One image per page: `Doc.pdf` -> `Doc.jpg`, or `Doc p1.jpg`, `Doc p2.jpg`, …
pub fn to_images(path: &Path, format: Format) -> Result<Vec<PathBuf>> {
    let doc = pdfium()?.load_pdf_from_file(path, None).map_err(pdf_err)?;
    let pages = doc.pages();
    let count = pages.len();
    let config = PdfRenderConfig::new().scale_page_by_factor(2.0);
    let mut out = Vec::new();
    for (i, page) in pages.iter().enumerate() {
        let img = page.render_with_config(&config).map_err(pdf_err)?.as_image().map_err(pdf_err)?;
        let suffix = (count > 1).then(|| format!("p{}", i + 1));
        let dest = naming::sibling(path, suffix.as_deref(), format.ext());
        image_ops::save(&img, &dest, format, 90)?;
        out.push(dest);
    }
    Ok(out)
}

pub fn to_text(path: &Path) -> Result<PathBuf> {
    let doc = pdfium()?.load_pdf_from_file(path, None).map_err(pdf_err)?;
    let mut text = String::new();
    for page in doc.pages().iter() {
        text.push_str(&page.text().map_err(pdf_err)?.all());
        text.push_str("\n\n");
    }
    let dest = naming::sibling(path, None, "txt");
    std::fs::write(&dest, text.trim_end())?;
    Ok(dest)
}
