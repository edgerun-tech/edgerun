#include "EdgerunEfiBt.h"

#define HCI_OP_LE_SET_EVENT_MASK       HCI_OPCODE(HCI_OGF_LE, 0x0001)
#define HCI_OP_LE_SET_ADV_PARAMS       HCI_OPCODE(HCI_OGF_LE, 0x0006)
#define HCI_OP_LE_SET_ADV_DATA         HCI_OPCODE(HCI_OGF_LE, 0x0008)
#define HCI_OP_LE_SET_SCAN_RSP_DATA    HCI_OPCODE(HCI_OGF_LE, 0x0009)
#define HCI_OP_LE_SET_ADV_ENABLE       HCI_OPCODE(HCI_OGF_LE, 0x000A)

STATIC
UINTN
AsciiLenBounded(
  IN CONST CHAR8 *Text,
  IN UINTN MaxLen
  )
{
  UINTN Len;

  if (Text == NULL) {
    return 0;
  }

  for (Len = 0; Len < MaxLen; Len++) {
    if (Text[Len] == '\0') {
      break;
    }
  }

  return Len;
}

EFI_STATUS
EdgerunBleStartAdvertising(
  IN OUT EDGERUN_BT_USB *Device,
  IN CONST CHAR8 *Name
  )
{
  EFI_STATUS Status;
  UINT8 Event[260];
  UINTN EventLen;
  UINT8 LeEventMask[8] = { 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x1f };
  UINT8 AdvParams[15] = {
    0xa0, 0x00, // min interval: 100 ms
    0xa0, 0x00, // max interval: 100 ms
    0x00,       // ADV_IND connectable undirected
    0x00,       // own address type: public
    0x00,       // direct address type: public
    0, 0, 0, 0, 0, 0,
    0x07,       // channels 37, 38, 39
    0x00        // allow all
  };
  UINT8 AdvData[32];
  UINT8 ScanRsp[32];
  UINTN NameLen;
  UINTN Pos;
  UINTN Index;

  if (Device == NULL || Device->UsbIo == NULL || Name == NULL) {
    return EFI_INVALID_PARAMETER;
  }

  NameLen = AsciiLenBounded(Name, 24);
  if (NameLen == 0 || NameLen > 24) {
    return EFI_INVALID_PARAMETER;
  }

  EventLen = sizeof(Event);
  Status = EdgerunHciCommand(Device, HCI_OP_LE_SET_EVENT_MASK, LeEventMask, sizeof(LeEventMask), Event, &EventLen);
  if (EFI_ERROR(Status)) {
    return Status;
  }

  EventLen = sizeof(Event);
  Status = EdgerunHciCommand(Device, HCI_OP_LE_SET_ADV_PARAMS, AdvParams, sizeof(AdvParams), Event, &EventLen);
  if (EFI_ERROR(Status)) {
    return Status;
  }

  ZeroMem(AdvData, sizeof(AdvData));
  Pos = 1;

  // Flags: LE General Discoverable Mode + BR/EDR Not Supported.
  AdvData[Pos++] = 0x02;
  AdvData[Pos++] = 0x01;
  AdvData[Pos++] = 0x06;

  // Complete local name.
  AdvData[Pos++] = (UINT8)(1 + NameLen);
  AdvData[Pos++] = 0x09;
  for (Index = 0; Index < NameLen; Index++) {
    AdvData[Pos++] = (UINT8)Name[Index];
  }

  AdvData[0] = (UINT8)(Pos - 1);

  EventLen = sizeof(Event);
  Status = EdgerunHciCommand(Device, HCI_OP_LE_SET_ADV_DATA, AdvData, sizeof(AdvData), Event, &EventLen);
  if (EFI_ERROR(Status)) {
    return Status;
  }

  ZeroMem(ScanRsp, sizeof(ScanRsp));
  EventLen = sizeof(Event);
  Status = EdgerunHciCommand(Device, HCI_OP_LE_SET_SCAN_RSP_DATA, ScanRsp, sizeof(ScanRsp), Event, &EventLen);
  if (EFI_ERROR(Status)) {
    return Status;
  }

  EventLen = sizeof(Event);
  Status = EdgerunHciCommand(Device, HCI_OP_LE_SET_ADV_ENABLE, (CONST UINT8 *)"\x01", 1, Event, &EventLen);
  if (EFI_ERROR(Status)) {
    return Status;
  }

  return EFI_SUCCESS;
}
