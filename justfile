# Gates named in CONTRIBUTING.md.

lint:
    cargo fmt --check
    cargo clippy --all-targets -- -D warnings

test:
    cargo test
