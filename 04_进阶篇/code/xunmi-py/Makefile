# 如果你的BUILD_DIR不同, 可以make BUILD_DIR=<your-dir>
BUILD_DIR := C:\Users\28304\Desktop\rust-basics\04_进阶篇\code\xunmi-py\lib

SRCS := ${wildcard src/*.rs} Cargo.toml
NAME = xunmi
TARGET = lib${NAME}
BUILD_FILE = ${BUILD_DIR}/${TARGET}.dylib
BUILD_FILE1 = ${BUILD_DIR}/${TARGET}.so
TARGET_FILE = ${NAME}/${NAME}.so

all: ${TARGET_FILE}

test: ${TARGET_FILE}
	python3 -m pytest

$(TARGET_FILE): $(BUILD_FILE1)
	@cp &(BUILD_FILE1) $(TARGET_FILE)

$(BUILD_FILE1): $(SRCS)
	@cargo build --release
	@mv $(BUILD_FILE) $(BUILD_FILE1) || true

PHONY: test all