use crate::Result;
use image::codecs::jpeg::JpegEncoder;
use image::DynamicImage;
use std::io::Write;
use std::path::Path;

const PT_PER_PX: f32 = 0.75;

pub fn write_images(images: &[DynamicImage], out: &Path) -> Result<()> {
    let mut buf: Vec<u8> = Vec::new();
    let mut offsets: Vec<usize> = Vec::new();
    buf.extend_from_slice(b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n");

    let page_obj = |i: usize| 3 + i * 3;
    let kids: Vec<String> = (0..images.len()).map(|i| format!("{} 0 R", page_obj(i))).collect();

    let obj = |buf: &mut Vec<u8>, offsets: &mut Vec<usize>, body: &[u8]| {
        offsets.push(buf.len());
        buf.extend_from_slice(format!("{} 0 obj\n", offsets.len()).as_bytes());
        buf.extend_from_slice(body);
        buf.extend_from_slice(b"\nendobj\n");
    };

    obj(&mut buf, &mut offsets, b"<< /Type /Catalog /Pages 2 0 R >>");
    obj(
        &mut buf,
        &mut offsets,
        format!("<< /Type /Pages /Kids [{}] /Count {} >>", kids.join(" "), images.len()).as_bytes(),
    );

    for (i, img) in images.iter().enumerate() {
        let rgb = DynamicImage::ImageRgb8(crate::image_ops::flatten_on_white(img));
        let mut jpeg = Vec::new();
        JpegEncoder::new_with_quality(&mut jpeg, 92).encode_image(&rgb)?;
        let (w, h) = (rgb.width(), rgb.height());
        let (pw, ph) = (w as f32 * PT_PER_PX, h as f32 * PT_PER_PX);
        let n = page_obj(i);

        obj(
            &mut buf,
            &mut offsets,
            format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {pw:.2} {ph:.2}] \
                 /Resources << /XObject << /Im0 {} 0 R >> >> /Contents {} 0 R >>",
                n + 2,
                n + 1
            )
            .as_bytes(),
        );
        let content = format!("q {pw:.2} 0 0 {ph:.2} 0 0 cm /Im0 Do Q");
        obj(
            &mut buf,
            &mut offsets,
            format!("<< /Length {} >>\nstream\n{content}\nendstream", content.len()).as_bytes(),
        );
        let mut image_obj = format!(
            "<< /Type /XObject /Subtype /Image /Width {w} /Height {h} /ColorSpace /DeviceRGB \
             /BitsPerComponent 8 /Filter /DCTDecode /Length {} >>\nstream\n",
            jpeg.len()
        )
        .into_bytes();
        image_obj.extend_from_slice(&jpeg);
        image_obj.extend_from_slice(b"\nendstream");
        obj(&mut buf, &mut offsets, &image_obj);
    }

    let xref = buf.len();
    write!(buf, "xref\n0 {}\n0000000000 65535 f \n", offsets.len() + 1)?;
    for off in &offsets {
        write!(buf, "{off:010} 00000 n \n")?;
    }
    write!(buf, "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n", offsets.len() + 1)?;
    std::fs::write(out, buf)?;
    Ok(())
}
