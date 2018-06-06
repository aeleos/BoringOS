TARGET_FILES += $(TARGET_DIR)/bin/test
BUILD_DIRS += test/target
INITRAMFS_FILES += /bin/test
FMT_DIRS += test

$(TARGET_DIR)/bin/test: target/$(BUILD_TARGET)/$(BUILD_TYPE)/test
	@mkdir -p $(shell dirname $@)
	cp $< $@

target/$(BUILD_TARGET)/$(BUILD_TYPE)/test: target/$(BUILD_TARGET)/$(BUILD_TYPE)/libtest.a
	$(LINKER) $(LINKER_FLAGS) $< -o $@

target/$(BUILD_TARGET)/$(BUILD_TYPE)/libtest.a: $(shell find test/src -name "*.rs") test/Cargo.toml $(STD_FILES)
	cd test && $(RUST_COMPILER) build $(RUST_COMPILER_FLAGS)
