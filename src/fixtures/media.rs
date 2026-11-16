//! The image part of the fixtures.
//!
//! A picture in a fixture points to a real PNG file in the .docx (`a:blip
//! r:embed` and `v:imagedata r:id`), so Word can load it. Every picture in one
//! document points to the same part. The PNG is 8 x 8 pixels in grey and is
//! written by code, so the bytes are the same on every run.

/// The relationship id of the image part in `word/_rels/document.xml.rels`.
/// The picture builders write it in `r:embed` and `r:id`.
pub(super) const IMAGE_REL_ID: &str = "rIdAdapdfImage1";

/// The name of the image part in the .docx.
pub(super) const IMAGE_PART: &str = "word/media/image1.png";

/// True when the XML points to the image part, so the .docx needs it.
pub(super) fn uses_image(xml: &str) -> bool {
    xml.contains(&format!(r#""{IMAGE_REL_ID}""#))
}

const SIZE: usize = 8;

/// An 8 x 8 PNG: 8-bit grey, no interlace, with a diagonal gradient.
pub(super) fn png() -> Vec<u8> {
    let mut header = Vec::new();
    header.extend_from_slice(&u32::try_from(SIZE).expect("small").to_be_bytes()); // width
    header.extend_from_slice(&u32::try_from(SIZE).expect("small").to_be_bytes()); // height
    header.extend_from_slice(&[8, 0, 0, 0, 0]); // depth 8, grey, deflate, no filter, no interlace

    // Each row starts with its filter type (0, none).
    let mut rows = Vec::new();
    for y in 0..SIZE {
        rows.push(0);
        for x in 0..SIZE {
            rows.push(u8::try_from((x + y) * 255 / (2 * (SIZE - 1))).expect("0 to 255"));
        }
    }

    let mut png = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    chunk(&mut png, b"IHDR", &header);
    chunk(&mut png, b"IDAT", &zlib_stored(&rows));
    chunk(&mut png, b"IEND", &[]);
    png
}

/// Appends a PNG chunk: length, type, data and the CRC of the type and data.
fn chunk(png: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    png.extend_from_slice(&u32::try_from(data.len()).expect("small").to_be_bytes());
    let start = png.len();
    png.extend_from_slice(kind);
    png.extend_from_slice(data);
    let crc = crc32(&png[start..]);
    png.extend_from_slice(&crc.to_be_bytes());
}

/// A zlib stream that holds the data in one stored (not compressed) block.
fn zlib_stored(data: &[u8]) -> Vec<u8> {
    let len = u16::try_from(data.len()).expect("one stored block holds up to 65535 bytes");
    let mut out = vec![0x78, 0x01]; // deflate, 32 KiB window, no dictionary, fastest
    out.push(1); // final block, stored
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(&(!len).to_le_bytes());
    out.extend_from_slice(data);
    out.extend_from_slice(&adler32(data).to_be_bytes());
    out
}

/// The CRC-32 that PNG uses (the same as zip's).
fn crc32(data: &[u8]) -> u32 {
    let mut crc = u32::MAX;
    for &byte in data {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            crc = if crc & 1 == 1 {
                (crc >> 1) ^ 0xEDB8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

fn adler32(data: &[u8]) -> u32 {
    let (mut a, mut b) = (1_u32, 0_u32);
    for &byte in data {
        a = (a + u32::from(byte)) % 65_521;
        b = (b + a) % 65_521;
    }
    (b << 16) | a
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checksums_match_known_values() {
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
        assert_eq!(adler32(b"Wikipedia"), 0x11E6_0398);
    }

    #[test]
    fn the_png_has_the_right_chunks_and_checksums() {
        let png = png();
        assert_eq!(&png[..8], &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]);
        let mut at = 8;
        let mut kinds = Vec::new();
        let mut idat = Vec::new();
        while at < png.len() {
            let len = u32::from_be_bytes(png[at..at + 4].try_into().unwrap()) as usize;
            let body = &png[at + 4..at + 8 + len];
            let crc = u32::from_be_bytes(png[at + 8 + len..at + 12 + len].try_into().unwrap());
            assert_eq!(crc, crc32(body), "CRC of {:?}", &body[..4]);
            kinds.push(String::from_utf8_lossy(&body[..4]).into_owned());
            if &body[..4] == b"IDAT" {
                idat.extend_from_slice(&body[4..]);
            } else if &body[..4] == b"IHDR" {
                assert_eq!(&body[4..12], &[0, 0, 0, 8, 0, 0, 0, 8], "8 x 8 pixels");
            }
            at += 12 + len;
        }
        assert_eq!(kinds, ["IHDR", "IDAT", "IEND"]);

        // The zlib stream: header, one stored block, and the Adler-32 of 8 rows of 9 bytes.
        let rows = &idat[7..idat.len() - 4];
        assert_eq!(rows.len(), SIZE * (SIZE + 1));
        assert_eq!(&idat[..3], &[0x78, 0x01, 1]);
        assert_eq!(
            u16::from_le_bytes([idat[3], idat[4]]) as usize,
            rows.len(),
            "stored length"
        );
        assert_eq!(
            u16::from_le_bytes([idat[5], idat[6]]),
            !u16::from_le_bytes([idat[3], idat[4]]),
            "length complement"
        );
        assert_eq!(
            idat[idat.len() - 4..],
            adler32(rows).to_be_bytes(),
            "Adler-32"
        );
    }

    #[test]
    fn the_picture_builder_uses_this_relationship_id() {
        let xml = crate::fixtures::Image::picture().xml();
        assert!(uses_image(&xml), "{xml}");
        assert!(!uses_image("<a:blip/>"));
    }
}
