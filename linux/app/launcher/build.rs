//! Windows 构建：把统一图标和版本信息嵌入 rkflash.exe；其他平台不做任何事。

use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=RKFLASH_WINDRES");
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    if env::var("CARGO_CFG_TARGET_ENV").as_deref() != Ok("gnu") {
        println!("cargo:warning=非 GNU 目标不嵌入图标资源，发布版请使用 x86_64-pc-windows-gnu");
        return;
    }

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("cargo sets it"));
    let icon = manifest_dir.join("../../../windows/rkflash.ico");
    println!("cargo:rerun-if-changed={}", icon.display());
    let version = env::var("CARGO_PKG_VERSION").expect("cargo sets it");
    let numeric = version
        .split(['.', '-'])
        .take(3)
        .map(|part| part.parse::<u16>().unwrap_or(0).to_string())
        .collect::<Vec<_>>()
        .join(",");

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("cargo sets it"));
    let rc = out_dir.join("rkflash.rc");
    let object = out_dir.join("rkflash-resource.o");
    std::fs::write(
        &rc,
        format!(
            r#"1 ICON "{icon}"
1 VERSIONINFO
FILEVERSION {numeric},0
PRODUCTVERSION {numeric},0
FILEOS 0x40004
FILETYPE 0x1
BEGIN
  BLOCK "StringFileInfo"
  BEGIN
    BLOCK "040904b0"
    BEGIN
      VALUE "CompanyName", "OpenCenter"
      VALUE "FileDescription", "RKFlash - Rockchip firmware flasher"
      VALUE "FileVersion", "{version}"
      VALUE "InternalName", "rkflash"
      VALUE "OriginalFilename", "rkflash.exe"
      VALUE "ProductName", "RKFlash"
      VALUE "ProductVersion", "{version}"
    END
  END
  BLOCK "VarFileInfo"
  BEGIN
    VALUE "Translation", 0x409, 1200
  END
END
"#,
            icon = icon.display().to_string().replace('\\', "/"),
        ),
    )
    .expect("write resource script");

    // 交叉编译默认使用 x86_64-w64-mingw32-windres，在 Windows 上使用 PATH 中的 windres。
    let windres = env::var("RKFLASH_WINDRES").unwrap_or_else(|_| {
        if cfg!(windows) {
            "windres".to_owned()
        } else {
            "x86_64-w64-mingw32-windres".to_owned()
        }
    });
    let status = Command::new(&windres)
        .arg(&rc)
        .args(["-O", "coff", "-o"])
        .arg(&object)
        .status()
        .unwrap_or_else(|error| panic!("无法运行 {windres}：{error}"));
    assert!(status.success(), "{windres} 编译资源失败");
    println!("cargo:rustc-link-arg-bins={}", object.display());
}
