//! Build utilities for FineType — DuckDB extension metadata appending.

use std::fs;
use std::io::{self, Write};
use std::path::Path;

/// Pad an ASCII string to exactly 32 bytes with null bytes.
///
/// Panics if the input is longer than 32 bytes.
pub fn padded_field(s: &str) -> [u8; 32] {
    let bytes = s.as_bytes();
    assert!(
        bytes.len() <= 32,
        "metadata field too long ({} bytes, max 32): {:?}",
        bytes.len(),
        s
    );
    let mut buf = [0u8; 32];
    buf[..bytes.len()].copy_from_slice(bytes);
    buf
}

/// Build the DuckDB extension metadata trailer.
///
/// The format is a WebAssembly custom section containing:
/// - Section header (22 bytes): type + LEB128 lengths + "duckdb_signature"
/// - 8 × 32-byte null-padded ASCII fields (256 bytes)
/// - 256 zero bytes reserved for signature
///
/// Total: 534 bytes appended to the shared library.
pub fn build_metadata(
    platform: &str,
    duckdb_version: &str,
    extension_version: &str,
    abi_type: &str,
) -> Vec<u8> {
    let mut buf = Vec::with_capacity(534);

    // WebAssembly custom section header
    buf.push(0x00); // custom section type
    // LEB128-encoded payload length: 531 = 0x213
    // 0x93 (1_0010011 = continuation + 0x13), 0x04 (0_0000100 = final + 0x04)
    buf.push(0x93);
    buf.push(0x04);
    // Name length: 16
    buf.push(0x10);
    // Section name
    buf.extend_from_slice(b"duckdb_signature");
    // LEB128-encoded content length: 512 = 0x200
    // 0x80 (1_0000000 = continuation + 0x00), 0x04 (0_0000100 = final + 0x04)
    buf.push(0x80);
    buf.push(0x04);

    // 8 metadata fields (32 bytes each), written FIELD8 → FIELD1
    buf.extend_from_slice(&padded_field("")); // FIELD8 (unused)
    buf.extend_from_slice(&padded_field("")); // FIELD7 (unused)
    buf.extend_from_slice(&padded_field("")); // FIELD6 (unused)
    buf.extend_from_slice(&padded_field(abi_type)); // FIELD5
    buf.extend_from_slice(&padded_field(extension_version)); // FIELD4
    buf.extend_from_slice(&padded_field(duckdb_version)); // FIELD3
    buf.extend_from_slice(&padded_field(platform)); // FIELD2
    buf.extend_from_slice(&padded_field("4")); // FIELD1 (magic)

    // 256 zero bytes for signature space
    buf.extend_from_slice(&[0u8; 256]);

    debug_assert_eq!(buf.len(), 534);
    buf
}

/// Append DuckDB extension metadata to a shared library file.
///
/// Copies `input` to `output` (via a `.tmp` intermediate), then appends the
/// metadata trailer. This matches the format produced by DuckDB's official
/// `append_extension_metadata.py` script.
pub fn append_metadata(
    input: &Path,
    output: &Path,
    platform: &str,
    duckdb_version: &str,
    extension_version: &str,
    abi_type: &str,
) -> io::Result<()> {
    let tmp = output.with_extension("duckdb_extension.tmp");

    // Copy the raw shared library to a temp file
    fs::copy(input, &tmp)?;

    // Append metadata
    let metadata = build_metadata(platform, duckdb_version, extension_version, abi_type);
    let mut file = fs::OpenOptions::new().append(true).open(&tmp)?;
    file.write_all(&metadata)?;
    file.flush()?;
    drop(file);

    // Atomic rename
    fs::rename(&tmp, output)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn padded_field_basic() {
        let f = padded_field("hello");
        assert_eq!(f.len(), 32);
        assert_eq!(&f[..5], b"hello");
        assert!(f[5..].iter().all(|&b| b == 0));
    }

    #[test]
    fn padded_field_empty() {
        let f = padded_field("");
        assert_eq!(f, [0u8; 32]);
    }

    #[test]
    fn padded_field_max_length() {
        let s = "a".repeat(32);
        let f = padded_field(&s);
        assert!(f.iter().all(|&b| b == b'a'));
    }

    #[test]
    #[should_panic(expected = "metadata field too long")]
    fn padded_field_too_long() {
        padded_field(&"x".repeat(33));
    }

    #[test]
    fn metadata_size() {
        let m = build_metadata("linux_amd64", "v1.2.0", "0.5.1", "C_STRUCT");
        assert_eq!(m.len(), 534);
    }

    #[test]
    fn metadata_header() {
        let m = build_metadata("linux_amd64", "v1.2.0", "0.5.1", "C_STRUCT");
        // Custom section type
        assert_eq!(m[0], 0x00);
        // LEB128 payload length = 531
        assert_eq!(m[1], 0x93);
        assert_eq!(m[2], 0x04);
        // Name length = 16
        assert_eq!(m[3], 0x10);
        // Section name
        assert_eq!(&m[4..20], b"duckdb_signature");
        // LEB128 content length = 512
        assert_eq!(m[20], 0x80);
        assert_eq!(m[21], 0x04);
    }

    #[test]
    fn metadata_fields() {
        let m = build_metadata("linux_amd64", "v1.2.0", "0.5.1", "C_STRUCT");
        let fields_start = 22;

        // FIELD8-6: empty
        for i in 0..3 {
            let offset = fields_start + i * 32;
            assert!(m[offset..offset + 32].iter().all(|&b| b == 0));
        }

        // FIELD5: abi_type = "C_STRUCT"
        let f5 = &m[fields_start + 3 * 32..fields_start + 4 * 32];
        assert_eq!(&f5[..8], b"C_STRUCT");
        assert!(f5[8..].iter().all(|&b| b == 0));

        // FIELD4: extension_version = "0.5.1"
        let f4 = &m[fields_start + 4 * 32..fields_start + 5 * 32];
        assert_eq!(&f4[..5], b"0.5.1");

        // FIELD3: duckdb_version = "v1.2.0"
        let f3 = &m[fields_start + 5 * 32..fields_start + 6 * 32];
        assert_eq!(&f3[..6], b"v1.2.0");

        // FIELD2: platform = "linux_amd64"
        let f2 = &m[fields_start + 6 * 32..fields_start + 7 * 32];
        assert_eq!(&f2[..11], b"linux_amd64");

        // FIELD1: magic = "4"
        let f1 = &m[fields_start + 7 * 32..fields_start + 8 * 32];
        assert_eq!(f1[0], b'4');
        assert!(f1[1..].iter().all(|&b| b == 0));
    }

    #[test]
    fn metadata_signature_space() {
        let m = build_metadata("linux_amd64", "v1.2.0", "0.5.1", "C_STRUCT");
        // Last 256 bytes should all be zero
        assert!(m[278..].iter().all(|&b| b == 0));
        assert_eq!(m.len() - 278, 256);
    }

    #[test]
    fn round_trip_append() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("test.so");
        let output = dir.path().join("test.duckdb_extension");

        // Create a dummy .so file
        let dummy_content = b"ELF dummy shared library content";
        fs::write(&input, dummy_content).unwrap();

        append_metadata(&input, &output, "linux_amd64", "v1.2.0", "0.5.1", "C_STRUCT").unwrap();

        let result = fs::read(&output).unwrap();

        // Original content preserved
        assert_eq!(&result[..dummy_content.len()], dummy_content);

        // Metadata appended
        assert_eq!(result.len(), dummy_content.len() + 534);

        // Verify we can read back fields from the tail
        let metadata = &result[dummy_content.len()..];
        assert_eq!(&metadata[4..20], b"duckdb_signature");
    }
}
