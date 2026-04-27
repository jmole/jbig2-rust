#![no_main]
#![forbid(unsafe_code)]

use std::io::Cursor;

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(mut decoder) = jbig2::Jbig2Decoder::new(Cursor::new(data)) else {
        return;
    };

    for page in 1..=decoder.num_pages() {
        let _ = decoder.decode_page(page);
    }
});
