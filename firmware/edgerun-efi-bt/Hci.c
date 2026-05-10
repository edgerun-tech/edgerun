#include "EdgerunEfiBt.h"

STATIC
EFI_STATUS
SendHciCommandPacket(
  IN OUT EDGERUN_BT_USB *Device,
  IN OUT UINT8 *Packet,
  IN UINTN PacketLen
  )
{
  EFI_USB_DEVICE_REQUEST Request;
  UINT32 UsbStatus;

  if (Device == NULL || Device->UsbIo == NULL || Packet == NULL || PacketLen < 3 || PacketLen > 255 + 3) {
    return EFI_INVALID_PARAMETER;
  }

  ZeroMem(&Request, sizeof(Request));
  Request.RequestType = USB_DEV_H2D | USB_REQ_TYPE_CLASS | USB_TARGET_DEVICE;
  Request.Request = 0;
  Request.Value = 0;
  Request.Index = 0;
  Request.Length = (UINT16)PacketLen;

  UsbStatus = 0;
  return Device->UsbIo->UsbControlTransfer(
                          Device->UsbIo,
                          &Request,
                          EfiUsbDataOut,
                          EDGERUN_USB_TIMEOUT_MS,
                          Packet,
                          PacketLen,
                          &UsbStatus
                          );
}

STATIC
EFI_STATUS
ReadHciEvent(
  IN OUT EDGERUN_BT_USB *Device,
  OUT UINT8 *Event,
  IN OUT UINTN *EventLen
  )
{
  EFI_STATUS Status;
  UINT32 UsbStatus;

  if (Device == NULL || Device->UsbIo == NULL || Event == NULL || EventLen == NULL || *EventLen == 0) {
    return EFI_INVALID_PARAMETER;
  }
  if (Device->EventEndpoint == 0) {
    return EFI_DEVICE_ERROR;
  }

  UsbStatus = 0;
  Status = Device->UsbIo->UsbSyncInterruptTransfer(
                           Device->UsbIo,
                           Device->EventEndpoint,
                           Event,
                           EventLen,
                           EDGERUN_HCI_EVENT_TIMEOUT_MS,
                           &UsbStatus
                           );
  return Status;
}

STATIC
EFI_STATUS
WaitCommandComplete(
  IN OUT EDGERUN_BT_USB *Device,
  IN UINT16 ExpectedOpcode,
  OUT UINT8 *Event,
  IN OUT UINTN *EventLen
  )
{
  EFI_STATUS Status;
  UINTN Poll;

  if (Event == NULL || EventLen == NULL || *EventLen < 6) {
    return EFI_INVALID_PARAMETER;
  }

  for (Poll = 0; Poll < EDGERUN_HCI_MAX_POLLS; Poll++) {
    UINTN Len;

    Len = *EventLen;
    ZeroMem(Event, Len);
    Status = ReadHciEvent(Device, Event, &Len);
    if (EFI_ERROR(Status)) {
      if (Status == EFI_TIMEOUT || Status == EFI_NOT_READY) {
        continue;
      }
      return Status;
    }

    if (Len < 3) {
      continue;
    }

    if (Event[0] == HCI_EVT_COMMAND_COMPLETE) {
      UINT8 ParamLen;
      UINT16 Opcode;
      UINT8 HciStatus;

      ParamLen = Event[1];
      if (ParamLen < 4 || Len < 6) {
        return EFI_DEVICE_ERROR;
      }

      Opcode = (UINT16)(Event[3] | (Event[4] << 8));
      HciStatus = Event[5];
      if (Opcode != ExpectedOpcode) {
        continue;
      }

      *EventLen = Len;
      if (HciStatus != 0) {
        Print(L"HCI command 0x%04x failed status=0x%02x\r\n", Opcode, HciStatus);
        return EFI_DEVICE_ERROR;
      }
      return EFI_SUCCESS;
    }

    if (Event[0] == HCI_EVT_COMMAND_STATUS) {
      UINT8 ParamLen;
      UINT8 HciStatus;
      UINT16 Opcode;

      ParamLen = Event[1];
      if (ParamLen < 4 || Len < 6) {
        return EFI_DEVICE_ERROR;
      }

      HciStatus = Event[2];
      Opcode = (UINT16)(Event[4] | (Event[5] << 8));
      if (Opcode == ExpectedOpcode && HciStatus != 0) {
        Print(L"HCI command-status 0x%04x failed status=0x%02x\r\n", Opcode, HciStatus);
        return EFI_DEVICE_ERROR;
      }
    }
  }

  return EFI_TIMEOUT;
}

EFI_STATUS
EdgerunHciCommand(
  IN OUT EDGERUN_BT_USB *Device,
  IN UINT16 Opcode,
  IN CONST UINT8 *Params,
  IN UINTN ParamsLen,
  OUT UINT8 *Event,
  IN OUT UINTN *EventLen
  )
{
  EFI_STATUS Status;
  UINT8 Packet[258];
  UINTN Index;

  if (Device == NULL || ParamsLen > 255 || Event == NULL || EventLen == NULL) {
    return EFI_INVALID_PARAMETER;
  }
  if (ParamsLen > 0 && Params == NULL) {
    return EFI_INVALID_PARAMETER;
  }

  ZeroMem(Packet, sizeof(Packet));
  Packet[0] = (UINT8)(Opcode & 0xFF);
  Packet[1] = (UINT8)(Opcode >> 8);
  Packet[2] = (UINT8)ParamsLen;
  for (Index = 0; Index < ParamsLen; Index++) {
    Packet[3 + Index] = Params[Index];
  }

  Status = SendHciCommandPacket(Device, Packet, ParamsLen + 3);
  if (EFI_ERROR(Status)) {
    return Status;
  }

  return WaitCommandComplete(Device, Opcode, Event, EventLen);
}

EFI_STATUS
EdgerunHciReset(
  IN OUT EDGERUN_BT_USB *Device
  )
{
  UINT8 Event[260];
  UINTN EventLen;

  EventLen = sizeof(Event);
  return EdgerunHciCommand(Device, HCI_OP_RESET, NULL, 0, Event, &EventLen);
}

EFI_STATUS
EdgerunHciReadLocalVersion(
  IN OUT EDGERUN_BT_USB *Device,
  OUT EDGERUN_HCI_LOCAL_VERSION *Version
  )
{
  EFI_STATUS Status;
  UINT8 Event[260];
  UINTN EventLen;

  if (Version == NULL) {
    return EFI_INVALID_PARAMETER;
  }

  EventLen = sizeof(Event);
  Status = EdgerunHciCommand(Device, HCI_OP_READ_LOCAL_VERSION, NULL, 0, Event, &EventLen);
  if (EFI_ERROR(Status)) {
    return Status;
  }
  if (EventLen < 14) {
    return EFI_DEVICE_ERROR;
  }

  Version->HciVersion = Event[6];
  Version->HciRevision = (UINT16)(Event[7] | (Event[8] << 8));
  Version->LmpPalVersion = Event[9];
  Version->Manufacturer = (UINT16)(Event[10] | (Event[11] << 8));
  Version->LmpPalSubversion = (UINT16)(Event[12] | (Event[13] << 8));
  return EFI_SUCCESS;
}

EFI_STATUS
EdgerunHciReadBdAddr(
  IN OUT EDGERUN_BT_USB *Device,
  OUT UINT8 BdAddr[6]
  )
{
  EFI_STATUS Status;
  UINT8 Event[260];
  UINTN EventLen;

  if (BdAddr == NULL) {
    return EFI_INVALID_PARAMETER;
  }

  EventLen = sizeof(Event);
  Status = EdgerunHciCommand(Device, HCI_OP_READ_BD_ADDR, NULL, 0, Event, &EventLen);
  if (EFI_ERROR(Status)) {
    return Status;
  }
  if (EventLen < 12) {
    return EFI_DEVICE_ERROR;
  }

  CopyMem(BdAddr, &Event[6], 6);
  return EFI_SUCCESS;
}
