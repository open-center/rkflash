# Third-party source provenance

This repository vendors the source needed to produce a standalone Apple Silicon build.

| Component | Upstream | Revision | License |
| --- | --- | --- | --- |
| rkdeveloptool | https://github.com/rockchip-linux/rkdeveloptool | `304f073752fd25c854e1bcf05d8e7f925b1f4e14` | GPL-2.0-or-later |
| libusb | https://github.com/libusb/libusb | `v1.0.30` / `87a55632db62c9bdc58cd31d3ccfa673f1bb017f` | LGPL-2.1-or-later |

The rkdeveloptool portability patch changes two fixed-size sector buffers from a runtime
integer bound to a compile-time constant so current Apple Clang accepts the C++ source.
