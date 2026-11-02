use crate::domain::StorageType;
use rkflash_config::ImageEntry;

struct PartitionPreset {
    enabled: bool,
    address: &'static str,
    name: &'static str,
}

const DEFAULT_PARTITIONS: &[PartitionPreset] = &[
    PartitionPreset {
        enabled: false,
        address: "0x00000000",
        name: "parameter",
    },
    PartitionPreset {
        enabled: false,
        address: "0x00004000",
        name: "uboot",
    },
    PartitionPreset {
        enabled: false,
        address: "0x00006000",
        name: "trust",
    },
    PartitionPreset {
        enabled: false,
        address: "0x00008000",
        name: "misc",
    },
    PartitionPreset {
        enabled: false,
        address: "0x0000a000",
        name: "resource",
    },
    PartitionPreset {
        enabled: false,
        address: "0x00010000",
        name: "kernel",
    },
    PartitionPreset {
        enabled: false,
        address: "0x00020000",
        name: "boot",
    },
    PartitionPreset {
        enabled: false,
        address: "0x00040000",
        name: "recovery",
    },
    PartitionPreset {
        enabled: false,
        address: "0x00080000",
        name: "system",
    },
];

const PARTITION_FLAGS: &[(&str, &str)] = &[
    ("parameter", "-p"),
    ("uboot", "-u"),
    ("trust", "-t"),
    ("misc", "-m"),
    ("resource", "-re"),
    ("kernel", "-k"),
    ("boot", "-b"),
    ("recovery", "-r"),
    ("system", "-s"),
];

#[derive(Default)]
pub(super) struct RockchipProfile;

impl RockchipProfile {
    pub(super) fn default_images(&self) -> Vec<ImageEntry> {
        DEFAULT_PARTITIONS
            .iter()
            .map(|preset| ImageEntry::new(preset.enabled, preset.address, preset.name))
            .collect()
    }

    pub(super) fn partition_flag(&self, name: &str) -> Option<&'static str> {
        PARTITION_FLAGS
            .iter()
            .find_map(|(partition, flag)| (*partition == name).then_some(*flag))
    }

    pub(super) fn storage_token(&self, storage: StorageType) -> Option<&'static str> {
        match storage {
            StorageType::Auto => None,
            StorageType::Emmc => Some("EMMC"),
            StorageType::SpiNor => Some("SPINOR"),
            StorageType::SpiNand => Some("SPINAND"),
        }
    }
}
