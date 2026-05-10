use crate::uefi::{
    self, EFI_DEVICE_ERROR, EFI_INVALID_PARAMETER, EFI_NOT_FOUND,
    EFI_OPEN_PROTOCOL_BY_HANDLE_PROTOCOL, EFI_SUCCESS, EfiGuid, EfiHandle, EfiLocateSearchType,
    EfiStatus,
};
use core::ffi::c_void;
use core::mem::MaybeUninit;
use core::ptr::null_mut;

pub const REALTEK_VENDOR_ID: u16 = 0x0bda;
pub const RTL8922_PRODUCT_ID: u16 = 0x8922;

const USB_CLASS_WIRELESS: u8 = 0xe0;
const USB_SUBCLASS_RF: u8 = 0x01;
const USB_PROTOCOL_BLUETOOTH: u8 = 0x01;

const USB_ENDPOINT_IN: u8 = 0x80;
const USB_ENDPOINT_TRANSFER_TYPE_MASK: u8 = 0x03;
const USB_ENDPOINT_BULK: u8 = 0x02;
const USB_ENDPOINT_INTERRUPT: u8 = 0x03;

pub const EFI_USB_IO_PROTOCOL_GUID: EfiGuid = EfiGuid {
    data1: 0x2b2f68d6,
    data2: 0x0cd2,
    data3: 0x44cf,
    data4: [0x8e, 0x8b, 0xbb, 0xa2, 0x0b, 0x1b, 0x5b, 0x75],
};

#[repr(C)]
#[derive(Clone, Copy)]
pub struct EfiUsbDeviceDescriptor {
    pub length: u8,
    pub descriptor_type: u8,
    pub bcd_usb: u16,
    pub device_class: u8,
    pub device_sub_class: u8,
    pub device_protocol: u8,
    pub max_packet_size0: u8,
    pub id_vendor: u16,
    pub id_product: u16,
    pub bcd_device: u16,
    pub manufacturer: u8,
    pub product: u8,
    pub serial_number: u8,
    pub num_configurations: u8,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct EfiUsbInterfaceDescriptor {
    pub length: u8,
    pub descriptor_type: u8,
    pub interface_number: u8,
    pub alternate_setting: u8,
    pub num_endpoints: u8,
    pub interface_class: u8,
    pub interface_sub_class: u8,
    pub interface_protocol: u8,
    pub interface: u8,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct EfiUsbEndpointDescriptor {
    pub length: u8,
    pub descriptor_type: u8,
    pub endpoint_address: u8,
    pub attributes: u8,
    pub max_packet_size: u16,
    pub interval: u8,
}

#[repr(C)]
pub struct EfiUsbDeviceRequest {
    pub request_type: u8,
    pub request: u8,
    pub value: u16,
    pub index: u16,
    pub length: u16,
}

#[repr(u32)]
#[derive(Clone, Copy)]
pub enum EfiUsbDataDirection {
    DataIn = 0,
    DataOut = 1,
    NoData = 2,
}

#[repr(C)]
pub struct EfiUsbIoProtocol {
    pub usb_control_transfer: extern "efiapi" fn(
        this: *mut EfiUsbIoProtocol,
        request: *mut EfiUsbDeviceRequest,
        direction: EfiUsbDataDirection,
        timeout: u32,
        data: *mut c_void,
        data_length: usize,
        status: *mut u32,
    ) -> EfiStatus,
    pub usb_bulk_transfer: extern "efiapi" fn(
        this: *mut EfiUsbIoProtocol,
        device_endpoint: u8,
        data: *mut c_void,
        data_length: *mut usize,
        timeout: usize,
        status: *mut u32,
    ) -> EfiStatus,
    pub usb_async_interrupt_transfer: usize,
    pub usb_sync_interrupt_transfer: extern "efiapi" fn(
        this: *mut EfiUsbIoProtocol,
        device_endpoint: u8,
        data: *mut c_void,
        data_length: *mut usize,
        timeout: usize,
        status: *mut u32,
    ) -> EfiStatus,
    pub usb_isochronous_transfer: usize,
    pub usb_async_isochronous_transfer: usize,
    pub usb_get_device_descriptor: extern "efiapi" fn(
        this: *mut EfiUsbIoProtocol,
        descriptor: *mut EfiUsbDeviceDescriptor,
    ) -> EfiStatus,
    pub usb_get_config_descriptor: usize,
    pub usb_get_interface_descriptor: extern "efiapi" fn(
        this: *mut EfiUsbIoProtocol,
        descriptor: *mut EfiUsbInterfaceDescriptor,
    ) -> EfiStatus,
    pub usb_get_endpoint_descriptor: extern "efiapi" fn(
        this: *mut EfiUsbIoProtocol,
        endpoint_index: u8,
        descriptor: *mut EfiUsbEndpointDescriptor,
    ) -> EfiStatus,
    pub usb_port_reset: extern "efiapi" fn(this: *mut EfiUsbIoProtocol) -> EfiStatus,
}

pub struct BtUsb {
    usb: *mut EfiUsbIoProtocol,
    pub vendor_id: u16,
    pub product_id: u16,
    pub interface_number: u8,
    pub event_ep: u8,
    pub acl_out_ep: u8,
    pub acl_in_ep: u8,
}

impl BtUsb {
    pub fn send_hci_command_packet(&mut self, packet: &mut [u8]) -> EfiStatus {
        if packet.len() > u16::MAX as usize {
            return EFI_INVALID_PARAMETER;
        }

        let mut request = EfiUsbDeviceRequest {
            request_type: 0x20,
            request: 0,
            value: 0,
            index: 0,
            length: packet.len() as u16,
        };
        let mut usb_status = 0u32;

        unsafe {
            ((*self.usb).usb_control_transfer)(
                self.usb,
                &mut request,
                EfiUsbDataDirection::DataOut,
                1_000,
                packet.as_mut_ptr().cast(),
                packet.len(),
                &mut usb_status,
            )
        }
    }

    pub fn read_hci_event(&mut self, out: &mut [u8], timeout_us: usize) -> Result<usize, EfiStatus> {
        if self.event_ep == 0 || out.is_empty() {
            return Err(EFI_INVALID_PARAMETER);
        }
        let mut len = out.len();
        let mut usb_status = 0u32;
        let status = unsafe {
            ((*self.usb).usb_sync_interrupt_transfer)(
                self.usb,
                self.event_ep,
                out.as_mut_ptr().cast(),
                &mut len,
                timeout_us,
                &mut usb_status,
            )
        };
        if status == EFI_SUCCESS {
            Ok(len)
        } else {
            Err(status)
        }
    }

    pub fn write_acl(&mut self, data: &mut [u8], timeout_us: usize) -> Result<usize, EfiStatus> {
        if self.acl_out_ep == 0 || data.is_empty() {
            return Err(EFI_INVALID_PARAMETER);
        }
        let mut len = data.len();
        let mut usb_status = 0u32;
        let status = unsafe {
            ((*self.usb).usb_bulk_transfer)(
                self.usb,
                self.acl_out_ep,
                data.as_mut_ptr().cast(),
                &mut len,
                timeout_us,
                &mut usb_status,
            )
        };
        if status == EFI_SUCCESS { Ok(len) } else { Err(status) }
    }

    pub fn read_acl(&mut self, out: &mut [u8], timeout_us: usize) -> Result<usize, EfiStatus> {
        if self.acl_in_ep == 0 || out.is_empty() {
            return Err(EFI_INVALID_PARAMETER);
        }
        let mut len = out.len();
        let mut usb_status = 0u32;
        let status = unsafe {
            ((*self.usb).usb_bulk_transfer)(
                self.usb,
                self.acl_in_ep,
                out.as_mut_ptr().cast(),
                &mut len,
                timeout_us,
                &mut usb_status,
            )
        };
        if status == EFI_SUCCESS { Ok(len) } else { Err(status) }
    }

    pub unsafe fn describe(&self) {
        unsafe {
            uefi::puts("USB Bluetooth: vid=0x");
            uefi::put_hex_u16(self.vendor_id);
            uefi::puts(" pid=0x");
            uefi::put_hex_u16(self.product_id);
            uefi::puts(" intf=0x");
            uefi::put_hex_u8(self.interface_number);
            uefi::puts(" event_ep=0x");
            uefi::put_hex_u8(self.event_ep);
            uefi::puts(" acl_out=0x");
            uefi::put_hex_u8(self.acl_out_ep);
            uefi::puts(" acl_in=0x");
            uefi::put_hex_u8(self.acl_in_ep);
            uefi::puts("\r\n");
        }
    }
}

pub unsafe fn find_rtl8922_controller() -> Result<BtUsb, EfiStatus> {
    let bs = unsafe { uefi::boot_services() };
    let mut count = 0usize;
    let mut handles: *mut EfiHandle = null_mut();

    let status = unsafe {
        ((*bs).locate_handle_buffer)(
            EfiLocateSearchType::ByProtocol,
            &EFI_USB_IO_PROTOCOL_GUID,
            null_mut(),
            &mut count,
            &mut handles,
        )
    };
    if status != EFI_SUCCESS {
        return Err(status);
    }

    unsafe {
        uefi::puts("USB handles with UsbIo: 0x");
        uefi::put_hex_usize(count);
        uefi::puts("\r\n");
    }

    let mut result = Err(EFI_NOT_FOUND);
    let mut i = 0usize;
    while i < count {
        let handle = unsafe { *handles.add(i) };
        if let Ok(device) = unsafe { probe_handle(handle) } {
            result = Ok(device);
            break;
        }
        i += 1;
    }

    unsafe { uefi::free_pool(handles.cast()) };
    result
}

unsafe fn probe_handle(handle: EfiHandle) -> Result<BtUsb, EfiStatus> {
    let bs = unsafe { uefi::boot_services() };
    let mut interface: *mut c_void = null_mut();

    let status = unsafe {
        ((*bs).open_protocol)(
            handle,
            &EFI_USB_IO_PROTOCOL_GUID,
            &mut interface,
            uefi::image_handle(),
            null_mut(),
            EFI_OPEN_PROTOCOL_BY_HANDLE_PROTOCOL,
        )
    };
    if status != EFI_SUCCESS || interface.is_null() {
        return Err(status);
    }

    let usb = interface.cast::<EfiUsbIoProtocol>();
    let mut dev = MaybeUninit::<EfiUsbDeviceDescriptor>::zeroed();
    let status = unsafe { ((*usb).usb_get_device_descriptor)(usb, dev.as_mut_ptr()) };
    if status != EFI_SUCCESS {
        return Err(status);
    }
    let dev = unsafe { dev.assume_init() };

    let mut intf = MaybeUninit::<EfiUsbInterfaceDescriptor>::zeroed();
    let status = unsafe { ((*usb).usb_get_interface_descriptor)(usb, intf.as_mut_ptr()) };
    if status != EFI_SUCCESS {
        return Err(status);
    }
    let intf = unsafe { intf.assume_init() };

    if !is_realtek_bluetooth(&dev, &intf) {
        return Err(EFI_NOT_FOUND);
    }

    unsafe {
        uefi::puts("candidate Realtek BT USB device vid=0x");
        uefi::put_hex_u16(dev.id_vendor);
        uefi::puts(" pid=0x");
        uefi::put_hex_u16(dev.id_product);
        if dev.id_product == RTL8922_PRODUCT_ID {
            uefi::puts(" RTL8922AE");
        }
        uefi::puts(" class=0x");
        uefi::put_hex_u8(intf.interface_class);
        uefi::puts("/0x");
        uefi::put_hex_u8(intf.interface_sub_class);
        uefi::puts("/0x");
        uefi::put_hex_u8(intf.interface_protocol);
        uefi::puts("\r\n");
    }

    let mut device = BtUsb {
        usb,
        vendor_id: dev.id_vendor,
        product_id: dev.id_product,
        interface_number: intf.interface_number,
        event_ep: 0,
        acl_out_ep: 0,
        acl_in_ep: 0,
    };

    unsafe { discover_endpoints(&mut device, intf.num_endpoints) }?;

    if device.event_ep == 0 || device.acl_out_ep == 0 || device.acl_in_ep == 0 {
        return Err(EFI_DEVICE_ERROR);
    }

    Ok(device)
}

fn is_realtek_bluetooth(dev: &EfiUsbDeviceDescriptor, intf: &EfiUsbInterfaceDescriptor) -> bool {
    dev.id_vendor == REALTEK_VENDOR_ID
        && intf.interface_class == USB_CLASS_WIRELESS
        && intf.interface_sub_class == USB_SUBCLASS_RF
        && intf.interface_protocol == USB_PROTOCOL_BLUETOOTH
}

unsafe fn discover_endpoints(device: &mut BtUsb, num_endpoints: u8) -> Result<(), EfiStatus> {
    let mut idx = 0u8;
    while idx < num_endpoints {
        let mut ep = MaybeUninit::<EfiUsbEndpointDescriptor>::zeroed();
        let status = unsafe {
            ((*device.usb).usb_get_endpoint_descriptor)(device.usb, idx, ep.as_mut_ptr())
        };
        if status != EFI_SUCCESS {
            return Err(status);
        }
        let ep = unsafe { ep.assume_init() };
        let transfer_type = ep.attributes & USB_ENDPOINT_TRANSFER_TYPE_MASK;
        let is_in = (ep.endpoint_address & USB_ENDPOINT_IN) != 0;

        if transfer_type == USB_ENDPOINT_INTERRUPT && is_in {
            device.event_ep = ep.endpoint_address;
        } else if transfer_type == USB_ENDPOINT_BULK && is_in {
            device.acl_in_ep = ep.endpoint_address;
        } else if transfer_type == USB_ENDPOINT_BULK && !is_in {
            device.acl_out_ep = ep.endpoint_address;
        }

        idx = idx.wrapping_add(1);
    }
    Ok(())
}
