# image-bilevel

Packed 1-bit-per-pixel bilevel image type for the image-rs ecosystem.

The `image` crate currently has no native 1-bpp image buffer type. Format crates that handle bilevel image data either expand to 8-bpp grayscale at every boundary or invent their own packed bitmap. This crate provides a small shared representation while leaving room for eventual upstreaming into `image` itself.

## Usage

```rust
let gray = image::GrayImage::from_raw(3, 1, vec![0, 127, 255]).unwrap();
let bilevel = image_bilevel::BilevelImage::from_luma8(&gray, 128);
assert_eq!(bilevel.data(), &[0b1100_0000]);

let expanded = bilevel.to_luma8(0, 255);
assert_eq!(expanded.into_raw(), vec![0, 0, 255]);
```

## Pixel Convention

`Bilevel(false)` is paper and `Bilevel(true)` is ink. Packed rows store the leftmost pixel in bit 7 of the first byte, matching PBM and JBIG2's `0 = paper`, `1 = ink` convention.

PNG, BMP, and TIFF commonly use the inverse interpretation for 1-bit grayscale samples (`0 = black`). Conversion methods document the mapping explicitly: `from_luma8` maps samples below the threshold to ink, and `to_luma8(ink, paper)` lets callers choose the expanded sample values.

## Toward Upstream

The natural long-term home for this representation is the `image` crate itself. This crate is intended as a working reference for a future RFC that could add:

- `image::ColorType::L1`
- `image::DynamicImage::ImageL1(...)`
- A documented packed-bit row convention for 1-bpp decoders and encoders

That RFC is intentionally out of scope for this crate's initial release.

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
