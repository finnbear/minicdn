fmt:
	cargo fmt
	cargo fmt --manifest-path minicdn_core/Cargo.toml
	cargo fmt --manifest-path minicdn_macros/Cargo.toml

test:
	cargo test
	# cargo test --no-default-features
	cargo run --example include --release --all-features
	cargo run --example include --release --features gzip,brotli,webp
	cargo run --example include --release --features brotli,walkdir --no-default-features
	cargo run --example include --release --features serde
	cargo run --example include --release
	cargo run --example include
	cargo run --example include_lite --no-default-features
	cargo run --example include_lite --no-default-features --features mime