#include "EdgerunEfiBt.h"

STATIC
BOOLEAN
IsBluetoothInterface(
  IN CONST EFI_USB_DEVICE_DESCRIPTOR *DeviceDescriptor,
  IN CONST EFI_USB_INTERFACE_DESCRIPTOR *InterfaceDescriptor
  )
{
  return DeviceDescriptor->IdVendor == EDGERUN_RTL_VENDOR_ID &&
         InterfaceDescriptor->InterfaceClass == EDGERUN_USB_CLASS_WIRELESS &&
         InterfaceDescriptor->InterfaceSubClass == EDGERUN_USB_SUBCLASS_RF &&
         InterfaceDescriptor->InterfaceProtocol == EDGERUN_USB_PROTOCOL_BT;
}

STATIC
EFI_STATUS
DiscoverEndpoints(
  IN EFI_USB_IO_PROTOCOL *UsbIo,
  IN UINT8 EndpointCount,
  OUT UINT8 *EventEndpoint,
  OUT UINT8 *AclOutEndpoint,
  OUT UINT8 *AclInEndpoint
  )
{
  EFI_STATUS Status;
  EFI_USB_ENDPOINT_DESCRIPTOR Endpoint;
  UINT8 Index;
  UINT8 TransferType;
  BOOLEAN IsIn;

  *EventEndpoint = 0;
  *AclOutEndpoint = 0;
  *AclInEndpoint = 0;

  for (Index = 0; Index < EndpointCount; Index++) {
    ZeroMem(&Endpoint, sizeof(Endpoint));
    Status = UsbIo->UsbGetEndpointDescriptor(UsbIo, Index, &Endpoint);
    if (EFI_ERROR(Status)) {
      return Status;
    }

    TransferType = Endpoint.Attributes & USB_ENDPOINT_TYPE_MASK;
    IsIn = (Endpoint.EndpointAddress & 0x80) != 0;

    if (TransferType == USB_ENDPOINT_INTERRUPT && IsIn) {
      *EventEndpoint = Endpoint.EndpointAddress;
    } else if (TransferType == USB_ENDPOINT_BULK && IsIn) {
      *AclInEndpoint = Endpoint.EndpointAddress;
    } else if (TransferType == USB_ENDPOINT_BULK && !IsIn) {
      *AclOutEndpoint = Endpoint.EndpointAddress;
    }
  }

  if (*EventEndpoint == 0 || *AclOutEndpoint == 0 || *AclInEndpoint == 0) {
    return EFI_DEVICE_ERROR;
  }

  return EFI_SUCCESS;
}

EFI_STATUS
EdgerunFindRtl8922UsbBluetooth(
  OUT EDGERUN_BT_USB *Device
  )
{
  EFI_STATUS Status;
  EFI_HANDLE *Handles;
  UINTN HandleCount;
  UINTN Index;

  if (Device == NULL) {
    return EFI_INVALID_PARAMETER;
  }

  ZeroMem(Device, sizeof(*Device));

  Handles = NULL;
  HandleCount = 0;
  Status = gBS->LocateHandleBuffer(
                  ByProtocol,
                  &gEfiUsbIoProtocolGuid,
                  NULL,
                  &HandleCount,
                  &Handles
                  );
  if (EFI_ERROR(Status)) {
    return Status;
  }

  Print(L"USB handles with UsbIo: %u\r\n", (UINT32)HandleCount);

  for (Index = 0; Index < HandleCount; Index++) {
    EFI_USB_IO_PROTOCOL *UsbIo;
    EFI_USB_DEVICE_DESCRIPTOR DeviceDescriptor;
    EFI_USB_INTERFACE_DESCRIPTOR InterfaceDescriptor;

    UsbIo = NULL;
    Status = gBS->OpenProtocol(
                    Handles[Index],
                    &gEfiUsbIoProtocolGuid,
                    (VOID **)&UsbIo,
                    gImageHandle,
                    NULL,
                    EFI_OPEN_PROTOCOL_BY_HANDLE_PROTOCOL
                    );
    if (EFI_ERROR(Status) || UsbIo == NULL) {
      continue;
    }

    ZeroMem(&DeviceDescriptor, sizeof(DeviceDescriptor));
    Status = UsbIo->UsbGetDeviceDescriptor(UsbIo, &DeviceDescriptor);
    if (EFI_ERROR(Status)) {
      continue;
    }

    ZeroMem(&InterfaceDescriptor, sizeof(InterfaceDescriptor));
    Status = UsbIo->UsbGetInterfaceDescriptor(UsbIo, &InterfaceDescriptor);
    if (EFI_ERROR(Status)) {
      continue;
    }

    if (!IsBluetoothInterface(&DeviceDescriptor, &InterfaceDescriptor)) {
      continue;
    }

    Print(
      L"candidate Realtek BT USB vid=0x%04x pid=0x%04x class=0x%02x/0x%02x/0x%02x\r\n",
      DeviceDescriptor.IdVendor,
      DeviceDescriptor.IdProduct,
      InterfaceDescriptor.InterfaceClass,
      InterfaceDescriptor.InterfaceSubClass,
      InterfaceDescriptor.InterfaceProtocol
      );

    Device->UsbIo = UsbIo;
    Device->VendorId = DeviceDescriptor.IdVendor;
    Device->ProductId = DeviceDescriptor.IdProduct;
    Device->InterfaceNumber = InterfaceDescriptor.InterfaceNumber;

    Status = DiscoverEndpoints(
               UsbIo,
               InterfaceDescriptor.NumEndpoints,
               &Device->EventEndpoint,
               &Device->AclOutEndpoint,
               &Device->AclInEndpoint
               );
    if (EFI_ERROR(Status)) {
      ZeroMem(Device, sizeof(*Device));
      continue;
    }

    if (Handles != NULL) {
      FreePool(Handles);
    }
    return EFI_SUCCESS;
  }

  if (Handles != NULL) {
    FreePool(Handles);
  }
  return EFI_NOT_FOUND;
}
