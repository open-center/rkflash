.DEFAULT_GOAL := app

SHELL := /bin/zsh

ROOT := $(abspath $(dir $(lastword $(MAKEFILE_LIST))))
BUILD_DIR := $(ROOT)/build
APP_NAME := RKFlashMac
XCODE_PRODUCT_DIR := $(strip $(TARGET_BUILD_DIR))
APP_OUTPUT_DIR := $(if $(XCODE_PRODUCT_DIR),$(XCODE_PRODUCT_DIR),$(BUILD_DIR))
APP_BUNDLE := $(APP_OUTPUT_DIR)/$(APP_NAME).app
APP_CONTENTS := $(APP_BUNDLE)/Contents
APP_MACOS := $(APP_CONTENTS)/MacOS
APP_RESOURCES := $(APP_CONTENTS)/Resources
APP_TOOL := $(APP_MACOS)/rkdeveloptool
APP_ICON := $(ROOT)/Resources/AppIcon.icns
INFO_PLIST := $(ROOT)/Resources/Info.plist
PRIVACY_MANIFEST := $(ROOT)/Resources/PrivacyInfo.xcprivacy
APP_ENTITLEMENTS := $(ROOT)/Resources/RKFlashMac.entitlements
DEBUG_ENTITLEMENTS := $(ROOT)/Resources/RKFlashMac.debug.entitlements
TOOL_ENTITLEMENTS := $(ROOT)/Resources/rkdeveloptool.entitlements
THIRD_PARTY_NOTICE := $(ROOT)/THIRD_PARTY.md
GPL_LICENSE := $(ROOT)/LICENSE
LGPL_LICENSE := $(ROOT)/Vendor/libusb/COPYING
APP_VERSION := $(shell /usr/libexec/PlistBuddy -c 'Print :CFBundleShortVersionString' $(INFO_PLIST))
DIST_DIR := $(BUILD_DIR)/dist
DMG_NAME = $(APP_NAME)-$(APP_VERSION)-$(ARCH).dmg
DMG_PATH = $(DIST_DIR)/$(DMG_NAME)
INSTALL_DIR ?= /Applications
INSTALLED_APP := $(INSTALL_DIR)/$(APP_NAME).app
UNIVERSAL_DIR := $(ROOT)/build/universal
UNIVERSAL_APP := $(UNIVERSAL_DIR)/$(APP_NAME).app

SDKROOT := $(shell xcrun --sdk macosx --show-sdk-path)
HOST_ARCH := $(shell uname -m)
ARCH ?= $(if $(filter arm64 x86_64,$(HOST_ARCH)),$(HOST_ARCH),arm64)
DEPLOYMENT_TARGET ?= 13.0
TARGET := $(ARCH)-apple-macos$(DEPLOYMENT_TARGET)
SANDBOX ?= 0

CC := xcrun --sdk macosx clang
CXX := xcrun --sdk macosx clang++
AR := xcrun --sdk macosx ar
SWIFTC := xcrun --sdk macosx swiftc

ifeq ($(CONFIGURATION),Debug)
SWIFT_OPT_FLAGS := -Onone -g
else
SWIFT_OPT_FLAGS := -O
endif

LIBUSB_ROOT := $(ROOT)/Vendor/libusb
LIBUSB_SOURCE_ROOT := $(LIBUSB_ROOT)/libusb
LIBUSB_SOURCES := \
	$(LIBUSB_SOURCE_ROOT)/core.c \
	$(LIBUSB_SOURCE_ROOT)/descriptor.c \
	$(LIBUSB_SOURCE_ROOT)/hotplug.c \
	$(LIBUSB_SOURCE_ROOT)/io.c \
	$(LIBUSB_SOURCE_ROOT)/strerror.c \
	$(LIBUSB_SOURCE_ROOT)/sync.c \
	$(LIBUSB_SOURCE_ROOT)/os/events_posix.c \
	$(LIBUSB_SOURCE_ROOT)/os/threads_posix.c \
	$(LIBUSB_SOURCE_ROOT)/os/darwin_usb.c
LIBUSB_OBJECTS := $(patsubst $(LIBUSB_SOURCE_ROOT)/%.c,$(BUILD_DIR)/libusb/%.o,$(LIBUSB_SOURCES))
LIBUSB_ARCHIVE := $(BUILD_DIR)/lib/libusb-1.0.a

RK_ROOT := $(ROOT)/Vendor/rkdeveloptool
RK_SOURCES := $(wildcard $(RK_ROOT)/*.cpp)
RK_OBJECTS := $(patsubst $(RK_ROOT)/%.cpp,$(BUILD_DIR)/rkdeveloptool/%.o,$(RK_SOURCES))
RK_TOOL := $(BUILD_DIR)/bin/rkdeveloptool

SWIFT_SOURCES := $(wildcard $(ROOT)/App/*.swift)
SWIFT_BINARY := $(BUILD_DIR)/app/$(APP_NAME)
MODEL_TEST := $(BUILD_DIR)/tests/RKModelsTests
FILE_FORMAT_TEST := $(BUILD_DIR)/tests/RKFileFormatTests

COMMON_TARGET_FLAGS := -arch $(ARCH) -mmacosx-version-min=$(DEPLOYMENT_TARGET) -isysroot $(SDKROOT)
LIBUSB_CFLAGS := -std=gnu11 -O2 -DNDEBUG -fvisibility=hidden -pthread \
	$(COMMON_TARGET_FLAGS) -I$(LIBUSB_ROOT)/Xcode -I$(LIBUSB_SOURCE_ROOT)
RK_CXXFLAGS := -std=gnu++17 -O2 -Wall -Wextra -Werror -Wno-deprecated-declarations \
	-fno-strict-aliasing -D_FILE_OFFSET_BITS=64 -D_LARGE_FILE $(COMMON_TARGET_FLAGS) \
	-I$(ROOT)/Core -I$(RK_ROOT) -I$(LIBUSB_ROOT)/Xcode -I$(LIBUSB_SOURCE_ROOT)

.PHONY: all app core check sandbox-check universal-app universal-check run install dmg clean

all: app

core: $(RK_TOOL)

app: $(APP_BUNDLE)

$(BUILD_DIR):
	@mkdir -p $@
	@xattr -w com.apple.xcode.CreatedByBuildSystem true $@ 2>/dev/null || true

$(BUILD_DIR)/libusb/%.o: $(LIBUSB_SOURCE_ROOT)/%.c | $(BUILD_DIR)
	@mkdir -p $(dir $@)
	$(CC) $(LIBUSB_CFLAGS) -c $< -o $@

$(LIBUSB_ARCHIVE): $(LIBUSB_OBJECTS) | $(BUILD_DIR)
	@mkdir -p $(dir $@)
	$(AR) rcs $@ $^

$(BUILD_DIR)/rkdeveloptool/%.o: $(RK_ROOT)/%.cpp $(ROOT)/Core/config.h | $(BUILD_DIR)
	@mkdir -p $(dir $@)
	$(CXX) $(RK_CXXFLAGS) -c $< -o $@

$(RK_TOOL): $(RK_OBJECTS) $(LIBUSB_ARCHIVE) | $(BUILD_DIR)
	@mkdir -p $(dir $@)
	$(CXX) $(COMMON_TARGET_FLAGS) $^ \
		-framework IOKit -framework CoreFoundation -framework Security -o $@

$(SWIFT_BINARY): $(SWIFT_SOURCES) | $(BUILD_DIR)
	@mkdir -p $(dir $@)
	$(SWIFTC) -swift-version 5 -parse-as-library $(SWIFT_OPT_FLAGS) -target $(TARGET) -sdk $(SDKROOT) \
		-framework SwiftUI -framework AppKit -framework UniformTypeIdentifiers \
		$(SWIFT_SOURCES) -o $@

$(APP_BUNDLE): $(SWIFT_BINARY) $(RK_TOOL) $(INFO_PLIST) $(APP_ICON) $(PRIVACY_MANIFEST) \
	$(APP_ENTITLEMENTS) $(DEBUG_ENTITLEMENTS) $(TOOL_ENTITLEMENTS) $(THIRD_PARTY_NOTICE) $(GPL_LICENSE) $(LGPL_LICENSE)
	@mkdir -p $(APP_MACOS) $(APP_RESOURCES)
	cp $(SWIFT_BINARY) $(APP_MACOS)/$(APP_NAME)
	cp $(RK_TOOL) $(APP_TOOL)
	@[[ ! -e $(APP_RESOURCES)/rkdeveloptool ]] || rm -f $(APP_RESOURCES)/rkdeveloptool
	cp $(APP_ICON) $(APP_RESOURCES)/AppIcon.icns
	cp $(PRIVACY_MANIFEST) $(APP_RESOURCES)/PrivacyInfo.xcprivacy
	cp $(THIRD_PARTY_NOTICE) $(APP_RESOURCES)/THIRD_PARTY.md
	cp $(GPL_LICENSE) $(APP_RESOURCES)/GPL-2.0.txt
	cp $(LGPL_LICENSE) $(APP_RESOURCES)/LGPL-2.1.txt
	cp $(INFO_PLIST) $(APP_CONTENTS)/Info.plist
	chmod 755 $(APP_MACOS)/$(APP_NAME) $(APP_TOOL)
	@if [[ "$(SANDBOX)" == "1" ]]; then \
		codesign --force --sign - --timestamp=none --options runtime \
			--identifier cn.roboticscenter.RKFlashMac.rkdeveloptool \
			--entitlements $(TOOL_ENTITLEMENTS) $(APP_TOOL); \
		codesign --force --sign - --timestamp=none --options runtime \
			--entitlements $(APP_ENTITLEMENTS) $(APP_BUNDLE); \
	else \
		codesign --force --sign - --timestamp=none $(APP_TOOL); \
		if [[ "$(CONFIGURATION)" == "Debug" ]]; then \
			codesign --force --sign - --timestamp=none \
				--entitlements $(DEBUG_ENTITLEMENTS) $(APP_BUNDLE); \
		else \
			codesign --force --sign - --timestamp=none $(APP_BUNDLE); \
		fi; \
	fi
	@if [[ "$(APP_OUTPUT_DIR)" == "$(BUILD_DIR)" ]]; then \
		ln -sfn $(APP_NAME).app $(BUILD_DIR)/$(APP_NAME); \
	fi
	@touch $(APP_BUNDLE)

$(MODEL_TEST): $(ROOT)/App/RKModels.swift $(ROOT)/App/RKFirmwareImage.swift $(ROOT)/Tests/RKModelsTests.swift | $(BUILD_DIR)
	@mkdir -p $(dir $@)
	$(SWIFTC) -swift-version 5 -parse-as-library -target $(TARGET) -sdk $(SDKROOT) $^ -o $@

$(FILE_FORMAT_TEST): $(ROOT)/Core/file_format.h $(ROOT)/Tests/RKFileFormatTests.cpp | $(BUILD_DIR)
	@mkdir -p $(dir $@)
	$(CXX) -std=gnu++17 $(COMMON_TARGET_FLAGS) -I$(ROOT)/Core $(ROOT)/Tests/RKFileFormatTests.cpp -o $@

check: $(APP_BUNDLE) $(MODEL_TEST) $(FILE_FORMAT_TEST)
	plutil -lint $(APP_CONTENTS)/Info.plist
	plutil -lint $(APP_RESOURCES)/PrivacyInfo.xcprivacy
	test -f $(APP_RESOURCES)/AppIcon.icns
	test -f $(APP_RESOURCES)/THIRD_PARTY.md
	test -f $(APP_RESOURCES)/GPL-2.0.txt
	test -f $(APP_RESOURCES)/LGPL-2.1.txt
	file $(APP_MACOS)/$(APP_NAME) $(APP_TOOL)
	$(MODEL_TEST)
	$(FILE_FORMAT_TEST)
	$(APP_TOOL) --version
	$(APP_TOOL) --help >/dev/null
	@if otool -L $(APP_TOOL) | grep -q '/opt/homebrew\|/usr/local'; then \
		echo 'ERROR: rkdeveloptool has an unexpected Homebrew dependency'; exit 1; \
	fi
	codesign --verify --deep --strict $(APP_BUNDLE)

sandbox-check:
	$(MAKE) BUILD_DIR=$(ROOT)/build/sandbox SANDBOX=1 app
	codesign --verify --deep --strict $(ROOT)/build/sandbox/$(APP_NAME).app
	codesign -d --entitlements :- $(ROOT)/build/sandbox/$(APP_NAME).app 2>/dev/null | \
		grep -q 'com.apple.security.device.usb'
	codesign -d --entitlements :- $(ROOT)/build/sandbox/$(APP_NAME).app/Contents/MacOS/rkdeveloptool 2>/dev/null | \
		grep -q 'com.apple.security.inherit'

universal-app:
	$(MAKE) BUILD_DIR=$(UNIVERSAL_DIR)/arm64 ARCH=arm64 SANDBOX=1 app
	$(MAKE) BUILD_DIR=$(UNIVERSAL_DIR)/x86_64 ARCH=x86_64 SANDBOX=1 app
	@mkdir -p $(UNIVERSAL_DIR)
	/usr/bin/ditto $(UNIVERSAL_DIR)/arm64/$(APP_NAME).app $(UNIVERSAL_APP)
	lipo -create \
		$(UNIVERSAL_DIR)/arm64/$(APP_NAME).app/Contents/MacOS/$(APP_NAME) \
		$(UNIVERSAL_DIR)/x86_64/$(APP_NAME).app/Contents/MacOS/$(APP_NAME) \
		-output $(UNIVERSAL_APP)/Contents/MacOS/$(APP_NAME)
	lipo -create \
		$(UNIVERSAL_DIR)/arm64/$(APP_NAME).app/Contents/MacOS/rkdeveloptool \
		$(UNIVERSAL_DIR)/x86_64/$(APP_NAME).app/Contents/MacOS/rkdeveloptool \
		-output $(UNIVERSAL_APP)/Contents/MacOS/rkdeveloptool
	codesign --force --sign - --timestamp=none --options runtime \
		--identifier cn.roboticscenter.RKFlashMac.rkdeveloptool \
		--entitlements $(TOOL_ENTITLEMENTS) $(UNIVERSAL_APP)/Contents/MacOS/rkdeveloptool
	codesign --force --sign - --timestamp=none --options runtime \
		--entitlements $(APP_ENTITLEMENTS) $(UNIVERSAL_APP)

universal-check: universal-app
	lipo $(UNIVERSAL_APP)/Contents/MacOS/$(APP_NAME) -verify_arch arm64 x86_64
	lipo $(UNIVERSAL_APP)/Contents/MacOS/rkdeveloptool -verify_arch arm64 x86_64
	plutil -lint $(UNIVERSAL_APP)/Contents/Info.plist $(UNIVERSAL_APP)/Contents/Resources/PrivacyInfo.xcprivacy
	codesign --verify --deep --strict $(UNIVERSAL_APP)

run: $(APP_BUNDLE)
	open $(APP_BUNDLE)

install: $(APP_BUNDLE)
	@test -d "$(INSTALL_DIR)" || { echo "ERROR: 安装目录不存在：$(INSTALL_DIR)"; exit 1; }
	@staging=$$(mktemp -d "$(INSTALL_DIR)/.$(APP_NAME).install.XXXXXX") || exit 1; \
		backup_root=$$(mktemp -d "$${TMPDIR:-/tmp}/$(APP_NAME).previous.XXXXXX") || exit 1; \
		backup="$$backup_root/$(APP_NAME).app"; \
		/usr/bin/ditto "$(APP_BUNDLE)" "$$staging/$(APP_NAME).app" || exit 1; \
		if [[ -e "$(INSTALLED_APP)" ]]; then mv "$(INSTALLED_APP)" "$$backup" || exit 1; fi; \
		if mv "$$staging/$(APP_NAME).app" "$(INSTALLED_APP)" && \
			/usr/bin/codesign --verify --deep --strict "$(INSTALLED_APP)"; then \
			rm -rf "$$staging" "$$backup_root"; \
		else \
			rm -rf "$(INSTALLED_APP)"; \
			if [[ -e "$$backup" ]]; then mv "$$backup" "$(INSTALLED_APP)"; fi; \
			rm -rf "$$staging" "$$backup_root"; \
			echo "ERROR: 安装验证失败，已恢复旧版本"; exit 1; \
		fi
	@touch "$(INSTALLED_APP)"
	@echo "已安装：$(INSTALLED_APP)"

dmg: $(APP_BUNDLE)
	@mkdir -p "$(DIST_DIR)"
	@staging=$$(mktemp -d "$(BUILD_DIR)/dmg.XXXXXX"); \
		trap 'rm -rf "$$staging"' EXIT; \
		/usr/bin/ditto "$(APP_BUNDLE)" "$$staging/$(APP_NAME).app"; \
		ln -s /Applications "$$staging/Applications"; \
		hdiutil create -volname "$(APP_NAME)" -srcfolder "$$staging" \
			-ov -format UDZO "$(DMG_PATH)"
	@echo "安装镜像：$(DMG_PATH)"

clean:
	rm -rf $(BUILD_DIR)
