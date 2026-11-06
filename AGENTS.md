# caffiend

See README.md. Run `cargo clippy --all-targets -- -D warnings` and `cargo test` before calling a change done; `nix-build default.nix -A packages.caffiend -A tests.eval` checks the Nix side. No unwrap/expect/panic outside tests (clippy denies them). Em and en dashes are not allowed in any text.
