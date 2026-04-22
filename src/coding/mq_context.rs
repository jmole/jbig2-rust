//! Context-ID constants matching `Jb2Common.h`.
//!
//! JBIG2 lays the MQ context pool out as a single array of
//! `Number_CX = 0x12000` entries. The low 0x10000 are used by the
//! generic-region pixel contexts (1024-entry template 0/1 space, 512-entry
//! template 2, 256-entry template 3, and the extended-template 4096-entry
//! space all fit comfortably). The high 0x2000 are sliced up into 512-entry
//! blocks, one per MQ integer decoder family.

/// Total number of MQ context slots JBIG2 allocates.
pub const NUMBER_CX: usize = 0x12000;

/// Base index of the `IAAI` integer coder family.
pub const IAAI: usize = 0x1_0000;
/// Base index of the `IADH` integer coder family.
pub const IADH: usize = 0x1_0200;
/// Base index of the `IADS` integer coder family.
pub const IADS: usize = 0x1_0400;
/// Base index of the `IADT` integer coder family.
pub const IADT: usize = 0x1_0600;
/// Base index of the `IADW` integer coder family.
pub const IADW: usize = 0x1_0800;
/// Base index of the `IAEX` integer coder family.
pub const IAEX: usize = 0x1_0A00;
/// Base index of the `IAFS` integer coder family.
pub const IAFS: usize = 0x1_0C00;
/// Base index of the `IAID` (symbol id) coder family.
pub const IAID: usize = 0x1_0E00;
/// Base index of the `IAIT` integer coder family.
pub const IAIT: usize = 0x1_1000;
/// Base index of the `IARDH` integer coder family.
pub const IARDH: usize = 0x1_1200;
/// Base index of the `IARDW` integer coder family.
pub const IARDW: usize = 0x1_1400;
/// Base index of the `IARDX` integer coder family.
pub const IARDX: usize = 0x1_1600;
/// Base index of the `IARDY` integer coder family.
pub const IARDY: usize = 0x1_1800;
/// Base index of the `IARI` integer coder family.
pub const IARI: usize = 0x1_1A00;

/// Size in slots of each `IA*` integer context family (except `IAID`).
pub const IA_FAMILY_SIZE: usize = 0x200;

/// Named family base addresses, useful for loops.
pub const IA_FAMILIES: &[(&str, usize)] = &[
    ("IAAI", IAAI),
    ("IADH", IADH),
    ("IADS", IADS),
    ("IADT", IADT),
    ("IADW", IADW),
    ("IAEX", IAEX),
    ("IAFS", IAFS),
    ("IAIT", IAIT),
    ("IARDH", IARDH),
    ("IARDW", IARDW),
    ("IARDX", IARDX),
    ("IARDY", IARDY),
    ("IARI", IARI),
];
