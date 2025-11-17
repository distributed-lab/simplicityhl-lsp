use std::collections::HashMap;

pub(crate) static TYPE_CASTS: std::sync::LazyLock<HashMap<&'static str, &'static str>> =
    std::sync::LazyLock::new(|| {
        HashMap::from([
            ("u1", "bool"),
            ("u2", "(u1, u1)"),
            ("u4", "(u2, u2)"),
            ("u8", "(u4, u4)"),
            ("u16", "(u8, u8)"),
            ("u32", "(u16, u16)"),
            ("u64", "(u32, u32)"),
            ("u128", "(u64, u64)"),
            ("u256", "(u128, u128)"),
            ("bool", "u1"),
            ("(u1, u1)", "u2"),
            ("(u2, u2)", "u4"),
            ("(u4, u4)", "u8"),
            ("(u8, u8)", "u16"),
            ("(u16, u16)", "u32"),
            ("(u32, u32)", "u64"),
            ("(u64, u64)", "u128"),
            ("(u128, u128)", "u256"),
        ])
    });
