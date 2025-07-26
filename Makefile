.PHONY: build test-rust test-python benchmark run

# Constrói o projeto Rust em modo de desenvolvimento
build:
	cargo build

# Executa os testes do core Rust
test-rust:
	cargo test --verbose

# Executa os testes da API Python
test-python:
	python -m pytest

# Executa os benchmarks do core Rust
benchmark:
	cargo bench --verbose

# Executa o projeto (assume que o binário se chama 'space')
run:
	cargo run