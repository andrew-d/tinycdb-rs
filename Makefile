TINYCDB_VERSION := 0.78
TINYCDB_PATH    := deps/tinycdb-$(TINYCDB_VERSION)

all: build

# Note: not depending on c_library, since that gets called by Cargo
.PHONY: build
build:
	@cargo build

.PHONY: c_library
c_library: $(TINYCDB_PATH)/libcdb_pic.a
	@cp $< $(OUT_DIR)/libcdb.a

$(TINYCDB_PATH)/libcdb_pic.a:
	$(MAKE) -C $(TINYCDB_PATH) piclib

.PHONY: doc
doc:
	@rustdoc src/lib.rs

.PHONY: test
test:
	@cargo test
