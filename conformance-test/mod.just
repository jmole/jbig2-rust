set working-directory := '..'

mod encode
mod decode

# Run every available phase x implementation.
default:
    CARGO_HOME=./.cargo cargo run --release --features image,conformance-tools --bin conformance-matrix -- --phase both --target all

known-issues-doc:
    CARGO_HOME=./.cargo cargo run --release --features image,conformance-tools --bin conformance-matrix -- --write-known-issues-doc docs/known-conformance-issues.md
