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

stable beta nightly:
	mkdir -p target.$@
	rm -f target
	ln -s target.$@ target

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
	cargo publish
	make upload-doc

clean:
	cargo clean
	$(RM) src/raw.rs
