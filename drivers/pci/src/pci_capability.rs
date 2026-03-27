//! # PCI Device Capabilities

// https://uefi.org/sites/default/files/resources/PCI_Code-ID_r_1_11__v24_Jan_2019.pdf

const CAP_NULL: u8 = 0x0;
const CAP_POWER_MANAGEMENT_INTERFACE: u8 = 0x01;
const CAP_ACCELERATED_GRAPHICS_PORT: u8 = 0x02;
const CAP_VITAL_PRODUCT_DATA: u8 = 0x03;
const CAP_SLOT_IDENTIFICATION: u8 = 0x04;
const CAP_MESSAGE_SIGNALED_INTERRUPTS: u8 = 0x05;
const CAP_COMPACT_PCI_HOT_SWAP: u8 = 0x06;
const CAP_PCI_X: u8 = 0x07;
const CAP_HYPER_TRANSPORT: u8 = 0x08;
const CAP_VENDOR_SPECIFIC: u8 = 0x09;
const CAP_DEBUG_PORT: u8 = 0x0A;
const CAP_COMPACT_PCI_CENTRAL_RESOURCE_CONTROL: u8 = 0x0B;
const CAP_PCI_HOT_PLUG: u8 = 0x0C;
const CAP_PCI_BRIDGE_SUBSYSTEM_VENDOR_ID: u8 = 0x0D;
const CAP_AGP_8X: u8 = 0x0E;
const CAP_SECURE_DEVICE: u8 = 0x0F;
const CAP_PCI_EXPRESS: u8 = 0x10;
const CAP_MSI_X: u8 = 0x11;
const CAP_SERIAL_ATA: u8 = 0x12;
const CAP_ADVANCED_FEATURES: u8 = 0x13;
const CAP_ENHANCED_ALLOCATION: u8 = 0x14;
const CAP_FLATTENING_PORTAL_BRIDGE: u8 = 0x15;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[repr(u8)]
pub enum Capability {
    Null = CAP_NULL,
    Pmi = CAP_POWER_MANAGEMENT_INTERFACE,
    Agp = CAP_ACCELERATED_GRAPHICS_PORT,
    Vpd = CAP_VITAL_PRODUCT_DATA,
    SlotIdentification = CAP_SLOT_IDENTIFICATION,
    Msi = CAP_MESSAGE_SIGNALED_INTERRUPTS,
    CompactPciHotSwap = CAP_COMPACT_PCI_HOT_SWAP,
    PciX = CAP_PCI_X,
    HyperTransport = CAP_HYPER_TRANSPORT,
    VendorSpecific = CAP_VENDOR_SPECIFIC,
    DebugPort = CAP_DEBUG_PORT,
    CompactPciCentralResourceControl = CAP_COMPACT_PCI_CENTRAL_RESOURCE_CONTROL,
    HotPlug = CAP_PCI_HOT_PLUG,
    BridgeSubsystemVendorId = CAP_PCI_BRIDGE_SUBSYSTEM_VENDOR_ID,
    Agp8x = CAP_AGP_8X,
    SecureDevice = CAP_SECURE_DEVICE,
    PciExpress = CAP_PCI_EXPRESS,
    MsiX = CAP_MSI_X,
    SerialAta = CAP_SERIAL_ATA,
    AdvancedFeatures = CAP_ADVANCED_FEATURES,
    EnhancedAllocation = CAP_ENHANCED_ALLOCATION,
    FlatteningPortalBridge = CAP_FLATTENING_PORTAL_BRIDGE,
}

impl Capability {
    pub const fn from_raw(raw: u8) -> Option<Self> {
        match raw {
            CAP_NULL => Some(Self::Null),
            CAP_POWER_MANAGEMENT_INTERFACE => Some(Self::Pmi),
            CAP_ACCELERATED_GRAPHICS_PORT => Some(Self::Agp),
            CAP_VITAL_PRODUCT_DATA => Some(Self::Vpd),
            CAP_SLOT_IDENTIFICATION => Some(Self::SlotIdentification),
            CAP_MESSAGE_SIGNALED_INTERRUPTS => Some(Self::Msi),
            CAP_COMPACT_PCI_HOT_SWAP => Some(Self::CompactPciHotSwap),
            CAP_PCI_X => Some(Self::PciX),
            CAP_HYPER_TRANSPORT => Some(Self::HyperTransport),
            CAP_VENDOR_SPECIFIC => Some(Self::VendorSpecific),
            CAP_DEBUG_PORT => Some(Self::DebugPort),
            CAP_COMPACT_PCI_CENTRAL_RESOURCE_CONTROL => {
                Some(Self::CompactPciCentralResourceControl)
            }
            CAP_PCI_HOT_PLUG => Some(Self::HotPlug),
            CAP_PCI_BRIDGE_SUBSYSTEM_VENDOR_ID => Some(Self::BridgeSubsystemVendorId),
            CAP_AGP_8X => Some(Self::Agp8x),
            CAP_SECURE_DEVICE => Some(Self::SecureDevice),
            CAP_PCI_EXPRESS => Some(Self::PciExpress),
            CAP_MSI_X => Some(Self::MsiX),
            CAP_SERIAL_ATA => Some(Self::SerialAta),
            CAP_ADVANCED_FEATURES => Some(Self::AdvancedFeatures),
            CAP_ENHANCED_ALLOCATION => Some(Self::EnhancedAllocation),
            CAP_FLATTENING_PORTAL_BRIDGE => Some(Self::FlatteningPortalBridge),

            _ => None,
        }
    }

    #[inline]
    pub const fn into_raw(self) -> u8 {
        self as u8
    }
}
