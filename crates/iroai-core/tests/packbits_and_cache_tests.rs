use iroai_core::packbits::PackBits;
use iroai_core::tile_cache::SparseTileBuffer;
use iroai_core::Color;

#[test]
fn test_packbits_roundtrip() {
    let mut data = Vec::new();
    // 10 identical bytes
    data.extend_from_slice(&[42u8; 10]);
    // 5 arbitrary bytes
    data.extend_from_slice(&[1, 2, 3, 4, 5]);
    // 30 identical zeros
    data.extend_from_slice(&[0u8; 30]);

    let compressed = PackBits::encode(&data);
    assert!(compressed.len() < data.len(), "PackBits should compress repetitive runs");

    let decompressed = PackBits::decode(&compressed, data.len()).expect("Failed to decode PackBits");
    assert_eq!(data, decompressed);
}

#[test]
fn test_sparse_tile_buffer_memory_savings() {
    // 4000 x 3000 キャンバス (通常なら 4000*3000*4 = 48MB)
    let mut sparse = SparseTileBuffer::new(4000, 3000, Color::TRANSPARENT);
    
    // 初期状態ではメモリ消費は 0 バイト
    assert_eq!(sparse.allocated_bytes(), 0);
    assert_eq!(sparse.get_pixel(100, 100), Color::TRANSPARENT);

    // 1つのピクセルだけ打つ -> 64x64タイル1個分(16KB)のみオンデマンド確保
    sparse.set_pixel(100, 100, Color::rgb(255, 0, 0));
    assert_eq!(sparse.allocated_bytes(), 64 * 64 * 4); // 16,384 bytes
    assert_eq!(sparse.get_pixel(100, 100), Color::rgb(255, 0, 0));
    assert_eq!(sparse.get_pixel(2000, 2000), Color::TRANSPARENT);

    // 通常のフラットPixelBufferへの展開検証
    let flat = sparse.to_pixel_buffer();
    assert_eq!(flat.width, 4000);
    assert_eq!(flat.height, 3000);
    assert_eq!(flat.get_pixel(100, 100).unwrap(), Color::rgb(255, 0, 0));
}
