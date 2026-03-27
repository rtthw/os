//! # PCI Device Classification

// https://uefi.org/sites/default/files/resources/PCI_Code-ID_r_1_11__v24_Jan_2019.pdf

const CLASS_UNCLASSIFIED: u8 = 0x00;
const CLASS_MASS_STORAGE_CONTROLLER: u8 = 0x01;
const CLASS_NETWORK_CONTROLLER: u8 = 0x02;
const CLASS_DISPLAY_CONTROLLER: u8 = 0x03;
const CLASS_MULTIMEDIA_DEVICE: u8 = 0x04;
const CLASS_MEMORY_CONTROLLER: u8 = 0x05;
const CLASS_BRIDGE_DEVICE: u8 = 0x06;
const CLASS_SIMPLE_COMMUNICATION: u8 = 0x07;
const CLASS_BASE_SYSTEM_PERIPHERAL: u8 = 0x08;
const CLASS_INPUT_DEVICE_CONTROLLER: u8 = 0x09;
const CLASS_DOCKING_STATION: u8 = 0x0A;
const CLASS_PROCESSOR: u8 = 0x0B;
const CLASS_SERIAL_BUS: u8 = 0x0C;
const CLASS_WIRELESS_CONTROLLER: u8 = 0x0D;
const CLASS_INTELLIGENT_IO_CONTROLLER: u8 = 0x0E;
const CLASS_SATELLITE_COMM_CONTROLLER: u8 = 0x0F;
const CLASS_ENCRYPTION_CONTROLLER: u8 = 0x10;
const CLASS_SIGNAL_PROCESSING_CONTROLLER: u8 = 0x11;
const CLASS_PROCESSING_ACCELERATOR: u8 = 0x12;
const CLASS_NON_ESSENTIAL_INSTRUMENTATION: u8 = 0x13;
const CLASS_COPROCESSOR: u8 = 0x40;
const CLASS_VENDOR_SPECIFIC: u8 = 0xFF;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[repr(u8)]
pub enum Class {
    Unclassified = CLASS_UNCLASSIFIED,
    MassStorageController = CLASS_MASS_STORAGE_CONTROLLER,
    NetworkController = CLASS_NETWORK_CONTROLLER,
    DisplayController = CLASS_DISPLAY_CONTROLLER,
    MultimediaDevice = CLASS_MULTIMEDIA_DEVICE,
    MemoryController = CLASS_MEMORY_CONTROLLER,
    BridgeDevice = CLASS_BRIDGE_DEVICE,
    SimpleCommunicationController = CLASS_SIMPLE_COMMUNICATION,
    BaseSystemPeripheral = CLASS_BASE_SYSTEM_PERIPHERAL,
    InputDevice = CLASS_INPUT_DEVICE_CONTROLLER,
    DockingStation = CLASS_DOCKING_STATION,
    Processor = CLASS_PROCESSOR,
    SerialBus = CLASS_SERIAL_BUS,
    WirelessController = CLASS_WIRELESS_CONTROLLER,
    IntelligentIoController = CLASS_INTELLIGENT_IO_CONTROLLER,
    SatelliteCommunicationController = CLASS_SATELLITE_COMM_CONTROLLER,
    EncryptionController = CLASS_ENCRYPTION_CONTROLLER,
    SignalProcessingController = CLASS_SIGNAL_PROCESSING_CONTROLLER,
    ProcessingAccelerator = CLASS_PROCESSING_ACCELERATOR,
    NonEssentialInstrumentation = CLASS_NON_ESSENTIAL_INSTRUMENTATION,
    CoProcessor = CLASS_COPROCESSOR,
    VendorSpecific = CLASS_VENDOR_SPECIFIC,
}

impl Class {
    pub const fn from_raw(raw: u8) -> Option<Class> {
        match raw {
            CLASS_UNCLASSIFIED => Some(Class::Unclassified),
            CLASS_MASS_STORAGE_CONTROLLER => Some(Class::MassStorageController),
            CLASS_NETWORK_CONTROLLER => Some(Class::NetworkController),
            CLASS_DISPLAY_CONTROLLER => Some(Class::DisplayController),
            CLASS_MULTIMEDIA_DEVICE => Some(Class::MultimediaDevice),
            CLASS_MEMORY_CONTROLLER => Some(Class::MemoryController),
            CLASS_BRIDGE_DEVICE => Some(Class::BridgeDevice),
            CLASS_SIMPLE_COMMUNICATION => Some(Class::SimpleCommunicationController),
            CLASS_BASE_SYSTEM_PERIPHERAL => Some(Class::BaseSystemPeripheral),
            CLASS_INPUT_DEVICE_CONTROLLER => Some(Class::InputDevice),
            CLASS_DOCKING_STATION => Some(Class::DockingStation),
            CLASS_PROCESSOR => Some(Class::Processor),
            CLASS_SERIAL_BUS => Some(Class::SerialBus),
            CLASS_WIRELESS_CONTROLLER => Some(Class::WirelessController),
            CLASS_INTELLIGENT_IO_CONTROLLER => Some(Class::IntelligentIoController),
            CLASS_SATELLITE_COMM_CONTROLLER => Some(Class::SatelliteCommunicationController),
            CLASS_ENCRYPTION_CONTROLLER => Some(Class::EncryptionController),
            CLASS_SIGNAL_PROCESSING_CONTROLLER => Some(Class::SignalProcessingController),
            CLASS_PROCESSING_ACCELERATOR => Some(Class::ProcessingAccelerator),
            CLASS_NON_ESSENTIAL_INSTRUMENTATION => Some(Class::NonEssentialInstrumentation),
            CLASS_COPROCESSOR => Some(Class::CoProcessor),
            CLASS_VENDOR_SPECIFIC => Some(Class::VendorSpecific),

            _ => None,
        }
    }

    #[inline]
    pub const fn into_raw(self) -> u8 {
        self as u8
    }
}
