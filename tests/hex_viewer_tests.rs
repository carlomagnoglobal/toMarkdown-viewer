use std::path::PathBuf;
use to_markdown_gui::viewers::{FileViewer, HexViewer};

#[test]
fn test_hex_viewer_render() {
    let path = PathBuf::from("test.bin");
    let bytes = vec![
        0x48, 0x65, 0x6C, 0x6C, 0x6F, 0x20, 0x57, 0x6F, 0x72, 0x6C, 0x64, 0x21, 0x00, 0x00,
        0x00, 0x00,
    ];
    let total_size = bytes.len() as u64;

    let viewer = HexViewer::new_from_bytes(path, bytes.clone(), total_size)
        .expect("Failed to create HexViewer");

    let html = viewer.render().expect("Failed to render");

    // Verify HTML structure (wrapped in div for styling)
    assert!(html.starts_with("<div"), "HTML should start with <div>");
    assert!(html.ends_with("</div>"), "HTML should end with </div>");

    // Verify content contains expected hex values
    assert!(html.contains("48"), "Should contain hex byte 48 (H)");
    assert!(html.contains("65"), "Should contain hex byte 65 (e)");
    assert!(html.contains("6C"), "Should contain hex byte 6C (l)");

    // Verify ASCII representation
    assert!(html.contains("Hello World!"), "Should contain ASCII representation");
}

#[test]
fn test_hex_viewer_16_bytes_per_line() {
    let path = PathBuf::from("test.bin");
    // Create 32 bytes of test data (should create exactly 2 rows)
    let bytes: Vec<u8> = (0..32).map(|i| i as u8).collect();
    let total_size = bytes.len() as u64;

    let viewer = HexViewer::new_from_bytes(path, bytes, total_size)
        .expect("Failed to create HexViewer");

    let html = viewer.render().expect("Failed to render");

    // Verify we have exactly 2 rows for 32 bytes (16 bytes per row)
    let row_count = html.matches("<tr>").count();
    assert_eq!(row_count, 2, "32 bytes should produce exactly 2 rows (16 bytes per row)");

    // Verify first offset is 00000000
    assert!(html.contains("00000000"), "First row should have offset 00000000");

    // Verify second offset is 00000010 (16 in hex)
    assert!(html.contains("00000010"), "Second row should have offset 00000010");
}
