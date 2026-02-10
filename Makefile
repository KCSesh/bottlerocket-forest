.PHONY: fmt
fmt:
	cargo fmt --quiet --all

.PHONY: check-fmt
check-fmt:
	cargo fmt --check --quiet --all

.PHONY: clippy
clippy:
	python3 scripts/clippy_wrapper.py

.PHONY: test
test:
	cargo test --workspace --release --locked --quiet --lib --bins

.PHONY: install
install:
	cargo install --path crates/sembly-cli --locked
	cargo install --path crates/forester --locked

.PHONY: deny
deny:
	cargo deny --no-default-features check licenses bans sources

.PHONY: lint-loc
lint-loc:
	python3 scripts/lint_loc.py

.PHONY: check
check: check-fmt clippy deny lint-loc test

.PHONY: integ
integ: check
	# Integration tests require --release on ARM64 Linux.
	# The candle ML framework depends on gemm, which uses f16 SIMD instructions.
	# In debug mode, gemm-f16 emits fullfp16 instructions that aren't available
	# on all ARM64 CPUs (e.g., Graviton). Release mode optimizes these away.
	# See: https://github.com/sarah-quinones/gemm/issues/31
	cargo test --workspace --release --locked --quiet -- --ignored

.PHONY: build
build:
	cargo build --workspace --locked --quiet

.PHONY: release-build
release-build:
	cargo build --workspace --release --locked --quiet

.PHONY: lint-style
lint-style:
	cargo run --quiet --release --package syn-lint

# Documentation
.PHONY: book book-serve
book:
	mdbook build crates/crumbly-cli/book

book-serve:
	mdbook serve crates/crumbly-cli/book
