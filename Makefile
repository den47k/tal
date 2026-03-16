LIB_DIR = target/release
LIB_NAME = tal

.PHONY: build clean run

build:
	cargo build --release

test_alloc: build tests/test_alloc.c
	gcc -o tests/test_alloc tests/test_alloc.c -L$(LIB_DIR) -l$(LIB_NAME)

test_alloc_dbg: tests/test_alloc.c
	cargo build
	gcc -g -o tests/test_alloc_dbg tests/test_alloc.c -Ltarget/debug -l$(LIB_NAME)

run: test_alloc
	LD_LIBRARY_PATH=$(LIB_DIR) ./tests/test_alloc

clean:
	cargo clean
	rm -f tests/test_alloc tests/test_alloc_dbg
