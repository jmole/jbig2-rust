set shell := ["bash", "-eu", "-o", "pipefail", "-c"]

default:
    just --list

test-output-jbig2dec:
    CARGO_HOME=./.cargo cargo test --test jbig2dec_output_compat --features image

# JBIG2 cross-implementation conformance matrix
mod conformance-test
