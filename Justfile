set shell := ["bash", "-eu", "-o", "pipefail", "-c"]

default:
    just --list

test-output-jbig2dec:
    CARGO_HOME=./.cargo cargo test -p jbig2-workbench --test jbig2dec_output_compat

# JBIG2 cross-implementation conformance matrix
mod conformance-test
