# RKFlash Windows 版构建：全部组件用 MinGW-w64 编译，可在 Linux/macOS 上交叉生成。
#
#   make core      rkdeveloptool.exe（内置 libusb）
#   make driver    WinUSB 驱动安装工具 wdi-simple.exe（libwdi）
#   make app       rkflash.exe（Rust，x86_64-pc-windows-gnu）
#   make package   免安装 zip 与 NSIS 安装程序，输出到 build/dist/
#
# 交叉编译：make package CROSS=x86_64-w64-mingw32-
# 依赖：MinGW-w64、Rust（rustup target add x86_64-pc-windows-gnu）、NSIS（makensis）、7-Zip、zip

.DEFAULT_GOAL := package

ROOT := $(abspath $(dir $(lastword $(MAKEFILE_LIST))))
THIRD_PARTY := $(abspath $(ROOT)/../../../third_party)
BUILD_DIR := $(ROOT)/build
REPO := $(abspath $(ROOT)/../../..)
VERSION := $(shell tr -d '[:space:]' < $(REPO)/VERSION)
PACKAGE_NAME := rkflash-$(VERSION)-windows-x64
STAGE := $(BUILD_DIR)/stage/$(PACKAGE_NAME)
DIST_DIR := $(BUILD_DIR)/dist
RUST_TARGET := x86_64-pc-windows-gnu
CARGO_TARGET_DIR ?= $(REPO)/output/target
APP_EXE := $(CARGO_TARGET_DIR)/$(RUST_TARGET)/release/rkflash.exe
DRIVER_DIR := $(BUILD_DIR)/driver/bin

CROSS ?=
CC := $(CROSS)gcc
CXX := $(CROSS)g++
AR := $(CROSS)ar
STRIP := $(CROSS)strip

LIBUSB_ROOT := $(THIRD_PARTY)/libusb
LIBUSB_SOURCE_ROOT := $(LIBUSB_ROOT)/libusb
LIBUSB_CONFIG := $(ROOT)/libusb-config
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

RK_ROOT := $(THIRD_PARTY)/rkdeveloptool
RK_SOURCES := $(wildcard $(RK_ROOT)/*.cpp)
RK_OBJECTS := $(patsubst $(RK_ROOT)/%.cpp,$(BUILD_DIR)/rkdeveloptool/%.o,$(RK_SOURCES))
RK_TOOL := $(BUILD_DIR)/bin/rkdeveloptool.exe

LIBUSB_CFLAGS := -std=gnu11 -O2 -DNDEBUG -I$(LIBUSB_CONFIG) -I$(LIBUSB_SOURCE_ROOT)
RK_CXXFLAGS := -std=gnu++17 -O2 -Wall -Wextra -Wno-deprecated-declarations \
	-fno-strict-aliasing -D_FILE_OFFSET_BITS=64 -D_LARGE_FILE \
	-I$(THIRD_PARTY)/rkdeveloptool/port -I$(RK_ROOT) -I$(LIBUSB_SOURCE_ROOT)
# 全静态链接：发布的 exe 不依赖 MinGW 运行库 DLL
LDFLAGS := -static -static-libgcc -static-libstdc++

.PHONY: core driver app stage zip setup package clean

core: $(RK_TOOL)

driver:
	CROSS=$(CROSS) $(ROOT)/driver/build.sh $(DRIVER_DIR)

# 交叉编译时用 MinGW 的 gcc 链接；在 Windows（MSYS2）上由 rustc 自行查找。
app:
	cd $(REPO) && \
		CARGO_TARGET_DIR=$(CARGO_TARGET_DIR) \
		$(if $(CROSS),CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER=$(CROSS)gcc RKFLASH_WINDRES=$(CROSS)windres) \
		cargo build --locked --release --target $(RUST_TARGET)

stage: core driver app
	rm -rf $(BUILD_DIR)/stage
	mkdir -p $(STAGE)/driver $(STAGE)/licenses
	cp $(APP_EXE) $(RK_TOOL) $(REPO)/assets/icons/rkflash.ico $(STAGE)/
	cp $(DRIVER_DIR)/wdi-simple.exe $(ROOT)/driver/install-driver.ps1 $(ROOT)/driver/rockchip-pids.txt $(STAGE)/driver/
	cp $(ROOT)/package/README.txt $(ROOT)/package/install-driver.cmd $(STAGE)/
	cp $(ROOT)/package/NOTICE.txt $(STAGE)/licenses/NOTICE.txt
	cp $(REPO)/crates/LICENSE $(STAGE)/licenses/RKFlash-MIT.txt
	cp $(THIRD_PARTY)/rkdeveloptool/license.txt $(STAGE)/licenses/rkdeveloptool-GPL-2.0.txt
	cp $(LIBUSB_ROOT)/COPYING $(STAGE)/licenses/libusb-LGPL-2.1.txt
	cp $(DRIVER_DIR)/libwdi-COPYING.txt $(STAGE)/licenses/libwdi-LGPL-3.0.txt
	cp $(DRIVER_DIR)/WDK-Redist-License.rtf $(STAGE)/licenses/

zip: stage
	mkdir -p $(DIST_DIR)
	rm -f $(DIST_DIR)/$(PACKAGE_NAME).zip
	cd $(BUILD_DIR)/stage && zip -qr -X $(DIST_DIR)/$(PACKAGE_NAME).zip $(PACKAGE_NAME)

# makensis 依赖 UTF-8 区域设置才能读取中文脚本。
setup: stage
	mkdir -p $(DIST_DIR)
	LC_ALL=C.UTF-8 makensis -V2 -INPUTCHARSET UTF8 -DVERSION=$(VERSION) -DSTAGE=$(STAGE) \
		-DOUTFILE=$(DIST_DIR)/$(PACKAGE_NAME)-setup.exe $(ROOT)/installer.nsi

package: zip setup
	@ls -l $(DIST_DIR)

$(BUILD_DIR)/libusb/%.o: $(LIBUSB_SOURCE_ROOT)/%.c
	@mkdir -p $(dir $@)
	$(CC) $(LIBUSB_CFLAGS) -c $< -o $@

$(LIBUSB_ARCHIVE): $(LIBUSB_OBJECTS)
	@mkdir -p $(dir $@)
	$(AR) rcs $@ $^

$(BUILD_DIR)/rkdeveloptool/%.o: $(RK_ROOT)/%.cpp $(THIRD_PARTY)/rkdeveloptool/port/config.h
	@mkdir -p $(dir $@)
	$(CXX) $(RK_CXXFLAGS) -c $< -o $@

$(RK_TOOL): $(RK_OBJECTS) $(LIBUSB_ARCHIVE)
	@mkdir -p $(dir $@)
	$(CXX) $^ $(LDFLAGS) -o $@
	$(STRIP) $@

clean:
	rm -rf $(BUILD_DIR)
