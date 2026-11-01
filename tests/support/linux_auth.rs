use super::*;

fn fake_token() -> String {
    format!("{}.{}.{}", "A".repeat(26), "B".repeat(6), "C".repeat(38))
}

#[test]
fn reads_current_compressed_records_without_touching_source() {
    let source = tempfile::tempdir().unwrap();
    let key = b"_https://discord.com\0\x01token";
    let fake = fake_token();
    let value = format!("\x01\"{fake}\"");
    let mut db = DB::open(
        source.path(),
        Options {
            compressor: 1,
            ..Options::default()
        },
    )
    .unwrap();
    db.put(key, value.as_bytes()).unwrap();
    db.flush().unwrap();
    db.compact_range(b"", b"\xff").unwrap();
    // Keep the source locked: discovery must only open the copy.
    assert_eq!(read_tokens(source.path()).unwrap()[0].as_str(), fake);
    assert_eq!(db.get(key).unwrap().as_ref(), value.as_bytes());
    db.delete(key).unwrap();
    db.flush().unwrap();
    assert!(read_tokens(source.path()).unwrap().is_empty());
}

#[test]
fn restricts_origin_key_and_token_format() {
    let value = format!("\x01\"{}\"", fake_token());
    assert!(token_record(b"_https://evil.example\0\x01token", value.as_bytes()).is_none());
    assert!(token_record(b"_https://discord.com\0\x01other", value.as_bytes()).is_none());
    assert!(token_record(b"_https://discord.com\0\x01token", b"\x01\"garbage\"").is_none());
    let utf16 = std::iter::once(0)
        .chain(
            format!("\"{}\"", fake_token())
                .encode_utf16()
                .flat_map(u16::to_le_bytes),
        )
        .collect::<Vec<_>>();
    assert_eq!(
        token_record(b"_https://discord.com\0\x01token", &utf16)
            .unwrap()
            .as_str(),
        fake_token()
    );
    assert!(decode_string(b"\0x").is_none());
}
