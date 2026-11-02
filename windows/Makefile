# RKFlash Windows 原生核心：用 MinGW-w64 静态编译 rkdeveloptool.exe（内置 libusb）。
#
# Windows 上在 MSYS2 UCRT64/MINGW64 环境执行：make core
# macOS/Linux 交叉编译：make core CROSS=x86_64-w64-mingw32-

.DEFAULT_GOAL := core

ROOT := $(abspath $(dir $(lastword $(MAKEFILE_LIST))))
NATIVE := $(abspath $(ROOT)/../native)
BUILD_DIR := $(ROOT)/build

CROSS ?=
CC := $(CROSS)gcc
CXX := $(CROSS)g++
AR := $(CROSS)ar
STRIP := $(CROSS)strip

LIBUSB_ROOT := $(NATIVE)/libusb
LIBUSB_SOURCE_ROOT := $(LIBUSB_ROOT)/libusb
LIBUSB_CONFIG := $(NATIVE)/libusb-config/windows
LIBUSB_SOURCES := \
	$(LIBUSB_SOURCE_ROOT)/core.c \
	$(LIBUSB_SOURCE_ROOT)/descriptor.c \
	$(LIBUSB_SOURCE_ROOT)/hotplug.c \
	$(LIBUSB_SOURCE_ROOT)/io.c \
	$(LIBUSB_SOURCE_ROOT)/strerror.c \
	$(LIBUSB_SOURCE_ROOT)/sync.c \
	$(LIBUSB_SOURCE_ROOT)/os/events_windows.c \
	$(LIBUSB_SOURCE_ROOT)/os/threads_windows.c \
	$(LIBUSB_SOURCE_ROOT)/os/windows_common.c \
	$(LIBUSB_SOURCE_ROOT)/os/windows_usbdk.c \
	$(LIBUSB_SOURCE_ROOT)/os/windows_winusb.c
LIBUSB_OBJECTS := $(patsubst $(LIBUSB_SOURCE_ROOT)/%.c,$(BUILD_DIR)/libusb/%.o,$(LIBUSB_SOURCES))
LIBUSB_ARCHIVE := $(BUILD_DIR)/lib/libusb-1.0.a

RK_ROOT := $(NATIVE)/rkdeveloptool
RK_SOURCES := $(wildcard $(RK_ROOT)/*.cpp)
RK_OBJECTS := $(patsubst $(RK_ROOT)/%.cpp,$(BUILD_DIR)/rkdeveloptool/%.o,$(RK_SOURCES))
RK_TOOL := $(BUILD_DIR)/bin/rkdeveloptool.exe

LIBUSB_CFLAGS := -std=gnu11 -O2 -DNDEBUG -I$(LIBUSB_CONFIG) -I$(LIBUSB_SOURCE_ROOT)
RK_CXXFLAGS := -std=gnu++17 -O2 -Wall -Wextra -Wno-deprecated-declarations \
	-fno-strict-aliasing -D_FILE_OFFSET_BITS=64 -D_LARGE_FILE \
	-I$(NATIVE)/core -I$(RK_ROOT) -I$(LIBUSB_SOURCE_ROOT)
# 全静态链接：发布的 exe 不依赖 MinGW 运行库 DLL
LDFLAGS := -static -static-libgcc -static-libstdc++

.PHONY: core clean

core: $(RK_TOOL)

$(BUILD_DIR)/libusb/%.o: $(LIBUSB_SOURCE_ROOT)/%.c
	@mkdir -p $(dir $@)
	$(CC) $(LIBUSB_CFLAGS) -c $< -o $@

$(LIBUSB_ARCHIVE): $(LIBUSB_OBJECTS)
	@mkdir -p $(dir $@)
	$(AR) rcs $@ $^

$(BUILD_DIR)/rkdeveloptool/%.o: $(RK_ROOT)/%.cpp $(NATIVE)/core/config.h
	@mkdir -p $(dir $@)
	$(CXX) $(RK_CXXFLAGS) -c $< -o $@

$(RK_TOOL): $(RK_OBJECTS) $(LIBUSB_ARCHIVE)
	@mkdir -p $(dir $@)
	$(CXX) $^ $(LDFLAGS) -o $@
	$(STRIP) $@

clean:
	rm -rf $(BUILD_DIR)
