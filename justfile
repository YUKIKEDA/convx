# Gates named in CONTRIBUTING.md.

lint:
    cargo fmt --check
    cargo clippy --all-targets -- -D warnings

test:
    cargo test
    # The allocation contract holds for release builds (tests/alloc.rs, #521).
    cargo test --release --test alloc
