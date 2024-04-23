<!-- cargo-rdme start -->

The most complete serialization tree for [`serde`].

[`Save`] represents the entire [serde data model](https://serde.rs/data-model.html),
including [struct names](Save::Struct::name), [field names](Save::Struct::fields),
and [enum variant information](Variant).
This means that it can intercept structures when they are serialized, before
losslessly forwarding them.

[`Save`] can optionally [persist errors](save_errors) _in the serialization tree_,
instead of short-circuiting.
This is a zero-cost option - see documentation on [`Save::Error`] for more.
```rust
#[derive(Serialize)]
struct MyStruct {
    system_time: SystemTime,
    path_buf: PathBuf,
    normal_string: String,
}

// These will fail to serialize
let before_unix_epoch = SystemTime::UNIX_EPOCH - Duration::from_secs(1);
let non_utf8_path = PathBuf::from(OsString::from_vec(vec![u8::MAX]));

let my_struct = MyStruct {
    system_time: before_unix_epoch,
    path_buf: non_utf8_path,
    normal_string: String::from("this is a string"), // this is fine
};

// By default errors are short-circuiting
assert_eq!(
    save(&my_struct).unwrap_err().to_string(),
    "SystemTime must be later than UNIX_EPOCH"
);

// But you can persist and inspect them in-tree if you prefer.
assert_eq!(
    save_errors(&my_struct), // use this method instead
    Save::strukt(
        "MyStruct",
        [
            ("system_time",   Save::error("SystemTime must be later than UNIX_EPOCH")),
            ("path_buf",      Save::error("path contains invalid UTF-8 characters")),
            ("normal_string", Save::string("this is a string")),
        ]
    )
)
```

[`Serializer`] can also check for incorrect implementations of the serde protocol.

See the documentation on [`Save`]s variants to see which invariants are checked.
You can [configure this behaviour](Serializer::check_for_protocol_errors).

<!-- cargo-rdme end -->
