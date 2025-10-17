
.PHONY: build install uninstall

TARGET_DIR := target/release
BINARY_NAME := illef-niri
SERVICE_FILE := illef-niri.service

build:
	cargo build --release

install: build
	sudo cp $(TARGET_DIR)/$(BINARY_NAME) /usr/local/bin/
	sudo cp $(SERVICE_FILE) /usr/lib/systemd/user/

uninstall:
	sudo rm -f /usr/local/bin/$(BINARY_NAME)
	sudo rm -f /usr/lib/systemd/user/$(SERVICE_FILE)
