.PHONY: test build test-node test-all bench clean

test:
	cargo test

build:
	wasm-pack build --target web

test-node: build
	node --test js/loader.test.mjs

test-all: test test-node

bench: build
	node js/bench.mjs

clean:
	cargo clean
	rm -rf pkg/
