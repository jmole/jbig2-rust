set working-directory := '..'

default:
    CARGO_HOME=./.cargo cargo run --release --features image,conformance-tools --bin conformance-matrix -- --phase encode --target all

system-binary:
    CARGO_HOME=./.cargo cargo run --release --features image,conformance-tools --bin conformance-matrix -- --phase encode --target system-binary

jbig2enc:
    CARGO_HOME=./.cargo cargo run --release --features image,conformance-tools --bin conformance-matrix -- --phase encode --target jbig2enc

itu-t88:
    CARGO_HOME=./.cargo cargo run --release --features image,conformance-tools --bin conformance-matrix -- --phase encode --target itu-t88

rust:
    CARGO_HOME=./.cargo cargo run --release --features image,conformance-tools --bin conformance-matrix -- --phase encode --target rust
