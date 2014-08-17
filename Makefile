TINYCDB_VERSION := 0.78
TINYCDB_PATH    := deps/tinycdb-$(TINYCDB_VERSION)
CFLAGS          := -fPIC

all: $(TINYCDB_PATH)/libcdb_pic.a
	@cp $< $(OUT_DIR)/libcdb.a

$(TINYCDB_PATH)/libcdb_pic.a:
	$(MAKE) -C $(TINYCDB_PATH) piclib
