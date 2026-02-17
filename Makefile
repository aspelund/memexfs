.PHONY: test build test-node test-pool test-all bench clean

test:
	cargo test

build:
	wasm-pack build --target web

test-node: build
	node --test js/loader.test.mjs

test-pool: build
	node --test js/pool.test.mjs

test-all: test test-node test-pool

bench: build
	node js/bench.mjs

clean:
	cargo clean
	rm -rf pkg/
