DESTDIR ?= $(HOME)/.local/bin

install:
	cargo build --release
	mkdir -p $(dir $(DESTDIR))
	rm -f $(DESTDIR)/dedupe
	cp target/release/dedupe $(DESTDIR)/dedupe
