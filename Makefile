default: build
hard: test

CRATE = succinct
REPO  = succinct-rs

build:
	clear
	cargo build
	make doc

clippy:
	rustup run nightly cargo build --features=clippy

doc:
	cargo doc # --no-deps -p $(CRATE)
	echo "<meta http-equiv='refresh' content='0;url=$(CRATE)/'>" > target/doc/index.html

test:
	clear
	cargo test

upload-doc:
	make doc
	ghp-import -n target/doc
	git push -f https://github.com/tov/$(REPO).git gh-pages

release:
	bin/prepare_release.sh $(VERSION)
	make upload-doc
	cargo publish

clean:
	cargo clean
	$(RM) src/raw.rs
