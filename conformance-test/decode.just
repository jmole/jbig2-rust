set working-directory := '..'

default:
    CARGO_HOME=./.cargo cargo run --release --features image,conformance-tools --bin conformance-matrix -- --phase decode --target all

system-binary:
    CARGO_HOME=./.cargo cargo run --release --features image,conformance-tools --bin conformance-matrix -- --phase decode --target system-binary

jbig2dec:
    CARGO_HOME=./.cargo cargo run --release --features image,conformance-tools --bin conformance-matrix -- --phase decode --target jbig2dec

itu-t88:
    CARGO_HOME=./.cargo cargo run --release --features image,conformance-tools --bin conformance-matrix -- --phase decode --target itu-t88

java:
    CARGO_HOME=./.cargo cargo run --release --features image,conformance-tools --bin conformance-matrix -- --phase decode --target java

rust:
    CARGO_HOME=./.cargo cargo run --release --features image,conformance-tools --bin conformance-matrix -- --phase decode --target rust
