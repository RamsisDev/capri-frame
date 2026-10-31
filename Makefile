.PHONY: test check

# Prueba gráfica manual: panel nativo + un proceso Capri de prueba.
test:
	cargo build -p capri-app --bin capri
	cargo run -p capri-ctl --features visual --bin capri-eyes -- --spawn

# Verificaciones automáticas, sin abrir ventanas.
check:
	cargo test --workspace --all-features
	cargo clippy --workspace --all-targets --all-features
