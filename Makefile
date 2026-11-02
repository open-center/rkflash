# rkflash 统一入口：版本管理与两个平台的构建、检查、打包。
# 各平台的完整能力仍分别由 linux/build.sh 与 macos/Makefile 提供，这里只做汇总。
# 统一命名的安装包输出到 dist/，与 GitHub Release 中的文件名一致。

.DEFAULT_GOAL := help
SHELL := /bin/bash

ROOT := $(abspath $(dir $(lastword $(MAKEFILE_LIST))))
VERSION := $(shell tr -d '[:space:]' < $(ROOT)/VERSION)
DIST := $(ROOT)/dist
HOST_ARCH := $(shell uname -m)
MAC_ARCH ?= $(if $(filter arm64 x86_64,$(HOST_ARCH)),$(HOST_ARCH),arm64)

.PHONY: help version version-check set-version linux linux-check macos macos-check check clean

help:
	@echo "rkflash $(VERSION)"
	@echo
	@echo "  make version-check        检查 linux/ 与 macos/ 版本是否与 VERSION 一致"
	@echo "  make set-version V=X.Y.Z  修改统一版本号并同步到两个平台"
	@echo "  make linux                构建 .deb（需要 Docker 和 linux/tools/upgrade_tool）"
	@echo "  make linux-check          Linux 格式、架构、测试、Clippy"
	@echo "  make macos                构建当前架构的 .dmg（MAC_ARCH=arm64|x86_64）"
	@echo "  make macos-check          macOS 构建与单元测试"
	@echo "  make check                版本一致性 + 当前系统能跑的平台检查"
	@echo "  make clean                清理两个平台的构建产物和 dist/"

version:
	@echo $(VERSION)

version-check:
	@$(ROOT)/scripts/version.sh check

set-version:
	@test -n "$(V)" || { echo "用法：make set-version V=X.Y.Z"; exit 1; }
	@$(ROOT)/scripts/version.sh set $(V)

linux: version-check
	cd $(ROOT)/linux && ./build.sh build
	@mkdir -p $(DIST)
	cp $(ROOT)/linux/output/dist/rkflash_$(VERSION)_amd64.deb $(DIST)/rkflash-$(VERSION)-linux-amd64.deb
	@ls -l $(DIST)

linux-check:
	cd $(ROOT)/linux && ./build.sh check

macos: version-check
	$(MAKE) -C $(ROOT)/macos ARCH=$(MAC_ARCH) dmg
	@mkdir -p $(DIST)
	cp $(ROOT)/macos/build/dist/RKFlash-$(VERSION)-$(MAC_ARCH).dmg $(DIST)/rkflash-$(VERSION)-macos-$(MAC_ARCH).dmg
	@ls -l $(DIST)

macos-check:
	$(MAKE) -C $(ROOT)/macos ARCH=$(MAC_ARCH) check

check: version-check
ifeq ($(shell uname -s),Darwin)
	$(MAKE) macos-check
else
	$(MAKE) linux-check
endif

clean:
	rm -rf $(DIST)
	$(MAKE) -C $(ROOT)/macos clean 2>/dev/null || true
	cd $(ROOT)/linux && ./build.sh clean
