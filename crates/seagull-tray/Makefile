INSTALL_DIR = /usr/bin
AUTOSTART_DIR = /etc/xdg/autostart

.PHONY: build install uninstall clean

build:
	cargo build --release -p seagull-tray

install:
	install -Dm755 ../../target/release/seagull-tray $(DESTDIR)$(INSTALL_DIR)/seagull-tray
	install -Dm644 seagull-tray.desktop $(DESTDIR)$(AUTOSTART_DIR)/seagull-tray.desktop

uninstall:
	rm -f $(DESTDIR)$(INSTALL_DIR)/seagull-tray
	rm -f $(DESTDIR)$(AUTOSTART_DIR)/seagull-tray.desktop

clean:
	cargo clean -p seagull-tray
