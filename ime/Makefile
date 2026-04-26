INSTALL_DIR = /usr/libexec
COMPONENT_DIR = /usr/share/ibus/component

.PHONY: build install uninstall clean

build:
	cargo build --release -p seagull-ime

install:
	install -Dm755 ../target/release/seagull-ime $(DESTDIR)$(INSTALL_DIR)/seagull-ime
	install -Dm644 seagull-ime.xml $(DESTDIR)$(COMPONENT_DIR)/seagull-ime.xml
	-ibus write-cache

uninstall:
	rm -f $(DESTDIR)$(INSTALL_DIR)/seagull-ime
	rm -f $(DESTDIR)$(COMPONENT_DIR)/seagull-ime.xml
	-ibus write-cache

clean:
	cargo clean -p seagull-ime
