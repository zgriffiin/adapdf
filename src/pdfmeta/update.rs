//! The appended part of an incremental update (ISO 32000-1, 7.5.6): the new
//! and changed objects, a cross-reference section, and a trailer whose `/Prev`
//! points to the cross-reference section the file had before.
//!
//! Only the appended bytes are made here. The caller writes the original file
//! first and these bytes after it, so the original bytes are always a prefix
//! of the output.

use std::collections::BTreeMap;

use lopdf::{Dictionary, Object, ObjectId, Stream, Writer};

/// Where the file's last cross-reference section starts, and what kind it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LastXref {
    pub offset: usize,
    /// `true` for a cross-reference stream, `false` for a table.
    pub is_stream: bool,
}

/// Finds the last `startxref` in the file and says what is at its offset.
pub fn last_xref(original: &[u8]) -> Option<LastXref> {
    const KEY: &[u8] = b"startxref";
    let at = original.windows(KEY.len()).rposition(|w| w == KEY)? + KEY.len();
    let digits: Vec<u8> = original[at..]
        .iter()
        .copied()
        .skip_while(|b| b.is_ascii_whitespace())
        .take_while(u8::is_ascii_digit)
        .collect();
    let offset: usize = std::str::from_utf8(&digits).ok()?.parse().ok()?;
    let target = original.get(offset..)?;
    let target = &target[target
        .iter()
        .take_while(|b| b.is_ascii_whitespace())
        .count()..];
    Some(LastXref {
        offset,
        is_stream: !target.starts_with(b"xref"),
    })
}

pub struct Update<'a> {
    original: &'a [u8],
    last: LastXref,
    /// The keys of the old trailer that the new trailer repeats.
    trailer: Dictionary,
    next_id: u32,
    objects: BTreeMap<ObjectId, Object>,
}

impl<'a> Update<'a> {
    /// `old_trailer` is the newest trailer of the file. `next_id` is the first
    /// object number that no object of the file uses.
    pub fn new(original: &'a [u8], last: LastXref, old_trailer: &Dictionary, next_id: u32) -> Self {
        let mut trailer = Dictionary::new();
        for key in [&b"Root"[..], b"Info", b"ID"] {
            if let Ok(value) = old_trailer.get(key) {
                trailer.set(key, value.clone());
            }
        }
        Update {
            original,
            last,
            trailer,
            next_id,
            objects: BTreeMap::new(),
        }
    }

    /// Writes a new version of an object that the file already has.
    pub fn replace(&mut self, id: ObjectId, object: Object) {
        self.objects.insert(id, object);
    }

    /// Adds an object with a new number.
    pub fn add(&mut self, object: Object) -> ObjectId {
        let id = (self.next_id, 0);
        self.next_id += 1;
        self.objects.insert(id, object);
        id
    }

    /// Points the trailer's `/Info` at an object.
    pub fn set_info(&mut self, id: ObjectId) {
        self.trailer.set("Info", Object::Reference(id));
    }

    /// The numbers of the objects that the update writes.
    pub fn object_ids(&self) -> Vec<ObjectId> {
        self.objects.keys().copied().collect()
    }

    pub fn is_empty(&self) -> bool {
        self.objects.is_empty()
    }

    /// The bytes to append to the original file.
    pub fn finish(mut self) -> Result<Vec<u8>, String> {
        let mut out: Vec<u8> = Vec::new();
        // The original may end without a line break. The new objects start on a
        // line of their own.
        if self.original.last().is_some_and(|b| *b != b'\n') {
            out.push(b'\n');
        }
        let base = self.original.len();
        let mut offsets: BTreeMap<u32, (u16, usize)> = BTreeMap::new();
        for (&(number, generation), object) in &self.objects {
            offsets.insert(number, (generation, base + out.len()));
            out.extend_from_slice(format!("{number} {generation} obj\n").as_bytes());
            Writer::write_object(&mut out, object).map_err(|e| e.to_string())?;
            out.extend_from_slice(b"\nendobj\n");
        }

        self.trailer.set("Prev", self.last.offset as i64);
        let xref_offset = base + out.len();
        if self.last.is_stream {
            // The cross-reference stream is an object too, and so it has an
            // entry of its own.
            let stream_number = self.next_id;
            offsets.insert(stream_number, (0, xref_offset));
            let mut data = Vec::new();
            let mut index = Vec::new();
            for (start, run) in runs(&offsets) {
                index.push(Object::Integer(i64::from(start)));
                index.push(Object::Integer(run.len() as i64));
                for (generation, offset) in run {
                    let offset = u32::try_from(offset).map_err(|_| {
                        "the file is too large for an incremental update".to_string()
                    })?;
                    data.push(1);
                    data.extend_from_slice(&offset.to_be_bytes());
                    data.extend_from_slice(&generation.to_be_bytes());
                }
            }
            let mut dict = self.trailer.clone();
            dict.set("Type", Object::Name(b"XRef".to_vec()));
            dict.set("Size", i64::from(stream_number) + 1);
            dict.set(
                "W",
                Object::Array(vec![
                    Object::Integer(1),
                    Object::Integer(4),
                    Object::Integer(2),
                ]),
            );
            dict.set("Index", Object::Array(index));
            out.extend_from_slice(format!("{stream_number} 0 obj\n").as_bytes());
            Writer::write_object(&mut out, &Object::Stream(Stream::new(dict, data)))
                .map_err(|e| e.to_string())?;
            out.extend_from_slice(b"\nendobj\n");
        } else {
            out.extend_from_slice(b"xref\n");
            for (start, run) in runs(&offsets) {
                out.extend_from_slice(format!("{start} {}\n", run.len()).as_bytes());
                for (generation, offset) in run {
                    out.extend_from_slice(format!("{offset:010} {generation:05} n \n").as_bytes());
                }
            }
            self.trailer.set("Size", i64::from(self.next_id));
            out.extend_from_slice(b"trailer\n");
            Writer::write_object(&mut out, &Object::Dictionary(self.trailer.clone()))
                .map_err(|e| e.to_string())?;
            out.push(b'\n');
        }
        out.extend_from_slice(format!("startxref\n{xref_offset}\n%%EOF\n").as_bytes());
        Ok(out)
    }
}

/// The entries grouped into runs of consecutive object numbers, each with the
/// number it starts at.
fn runs(offsets: &BTreeMap<u32, (u16, usize)>) -> Vec<(u32, Vec<(u16, usize)>)> {
    let mut result: Vec<(u32, Vec<(u16, usize)>)> = Vec::new();
    for (&number, &entry) in offsets {
        match result.last_mut() {
            Some((start, run)) if *start + run.len() as u32 == number => run.push(entry),
            _ => result.push((number, vec![entry])),
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_the_last_startxref_and_its_kind() {
        let table = b"%PDF-1.4\n1 0 obj\n<<>>\nendobj\nxref\n0 1\n0000000000 65535 f \ntrailer\n<<>>\nstartxref\n29\n%%EOF\n";
        let found = last_xref(table).unwrap();
        assert_eq!(found.offset, 29);
        assert!(!found.is_stream);
        let stream = b"%PDF-1.5\n2 0 obj\n<<>>\nendobj\nstartxref\n9\n%%EOF";
        assert!(last_xref(stream).unwrap().is_stream);
        assert_eq!(last_xref(b"%PDF-1.4\n"), None);
        assert_eq!(last_xref(b"startxref\n99999\n%%EOF"), None);
    }

    #[test]
    fn runs_group_consecutive_numbers() {
        let mut offsets = BTreeMap::new();
        for n in [3, 4, 7, 9, 10] {
            offsets.insert(n, (0u16, n as usize));
        }
        let starts: Vec<(u32, usize)> = runs(&offsets)
            .into_iter()
            .map(|(s, r)| (s, r.len()))
            .collect();
        assert_eq!(starts, vec![(3, 2), (7, 1), (9, 2)]);
    }
}
