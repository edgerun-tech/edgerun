#include "EdgerunEfiBt.h"

STATIC
VOID
PrintLocalVersion(
  IN CONST EDGERUN_HCI_LOCAL_VERSION *Version
  )
{
  Print(
    L"HCI version=0x%02x hci_rev=0x%04x lmp=0x%02x manufacturer=0x%04x lmp_sub=0x%04x\r\n",
    Version->HciVersion,
    Version->HciRevision,
    Version->LmpPalVersion,
    Version->Manufacturer,
    Version->LmpPalSubversion
    );
}

STATIC
EFI_STATUS
ReadRealtekRomVersion(
  IN OUT EDGERUN_BT_USB *Device,
  OUT UINT8 *RomVersion
  )
{
  EFI_STATUS Status;
  UINT8 Event[260];
  UINTN EventLen;

  if (RomVersion == NULL) {
    return EFI_INVALID_PARAMETER;
  }

  EventLen = sizeof(Event);
  Status = EdgerunHciCommand(Device, HCI_OP_RTL_READ_ROM_VERSION, NULL, 0, Event, &EventLen);
  if (EFI_ERROR(Status)) {
    return Status;
  }

  // EdgerunHciCommand already validated the first return byte as command status.
  if (EventLen < 7) {
    return EFI_DEVICE_ERROR;
  }

  *RomVersion = Event[6];
  return EFI_SUCCESS;
}

STATIC
EFI_STATUS
ReadRealtekReg16(
  IN OUT EDGERUN_BT_USB *Device,
  IN CONST UINT8 RegCommand[5],
  OUT UINT16 *Value
  )
{
  EFI_STATUS Status;
  UINT8 Event[260];
  UINTN EventLen;

  if (Device == NULL || RegCommand == NULL || Value == NULL) {
    return EFI_INVALID_PARAMETER;
  }

  EventLen = sizeof(Event);
  Status = EdgerunHciCommand(Device, HCI_OP_RTL_READ_REG16, RegCommand, 5, Event, &EventLen);
  if (EFI_ERROR(Status)) {
    return Status;
  }

  // EdgerunHciCommand already validated the first return byte as command status,
  // so the two-byte register value starts immediately after it.
  if (EventLen < 8) {
    return EFI_DEVICE_ERROR;
  }

  *Value = (UINT16)(Event[6] | ((UINT16)Event[7] << 8));
  return EFI_SUCCESS;
}

STATIC
EFI_STATUS
ReadRealtekSecurityProjectKey(
  IN OUT EDGERUN_BT_USB *Device,
  OUT UINT8 *KeyId
  )
{
  EFI_STATUS Status;
  UINT16 Value;
  CONST UINT8 SecurityProjectRegister[5] = { 0x10, 0xA4, 0xAD, 0x00, 0xB0 };

  if (KeyId == NULL) {
    return EFI_INVALID_PARAMETER;
  }

  *KeyId = 0;
  Value = 0;
  Status = ReadRealtekReg16(Device, SecurityProjectRegister, &Value);
  if (EFI_ERROR(Status)) {
    return Status;
  }

  *KeyId = (UINT8)(Value & 0xFF);
  return EFI_SUCCESS;
}

STATIC
BOOLEAN
LooksLikeRtl8922A(
  IN CONST EDGERUN_HCI_LOCAL_VERSION *Version
  )
{
  return Version->Manufacturer == RTL_COMPANY_ID &&
         Version->LmpPalSubversion == RTL_ROM_LMP_8922A &&
         Version->HciRevision == RTL8922A_HCI_REVISION &&
         Version->HciVersion == RTL8922A_HCI_VERSION;
}

EFI_STATUS
EdgerunRealtekInitRtl8922A(
  IN OUT EDGERUN_BT_USB *Device,
  IN EFI_HANDLE ImageHandle
  )
{
  EFI_STATUS Status;
  EDGERUN_HCI_LOCAL_VERSION Version;
  EDGERUN_RTL_FIRMWARE_FILES Files;
  EDGERUN_RTL_PATCH_IMAGE Patch;
  UINT8 RomVersion;
  UINT8 KeyId;

  if (Device == NULL || Device->UsbIo == NULL) {
    return EFI_INVALID_PARAMETER;
  }

  ZeroMem(&Files, sizeof(Files));
  ZeroMem(&Patch, sizeof(Patch));

  Print(L"HCI reset...\r\n");
  Status = EdgerunHciReset(Device);
  if (EFI_ERROR(Status)) {
    return Status;
  }

  ZeroMem(&Version, sizeof(Version));
  Status = EdgerunHciReadLocalVersion(Device, &Version);
  if (EFI_ERROR(Status)) {
    return Status;
  }
  PrintLocalVersion(&Version);

  if (!LooksLikeRtl8922A(&Version)) {
    Print(L"warning: controller does not exactly match RTL8922A USB tuple\r\n");
  }

  RomVersion = 0;
  Status = ReadRealtekRomVersion(Device, &RomVersion);
  if (EFI_ERROR(Status)) {
    Print(L"Realtek ROM version read failed: ");
    EdgerunPrintStatus(Status);
    Print(L"\r\n");
    return Status;
  }
  Print(L"Realtek ROM version=0x%02x\r\n", RomVersion);

  KeyId = 0;
  Status = ReadRealtekSecurityProjectKey(Device, &KeyId);
  if (EFI_ERROR(Status)) {
    Print(L"Realtek security project key read failed; using key_id=0: ");
    EdgerunPrintStatus(Status);
    Print(L"\r\n");
    KeyId = 0;
  } else {
    Print(L"Realtek security project key_id=0x%02x\r\n", KeyId);
  }

  Status = EdgerunLoadRtl8922aFirmwareFiles(ImageHandle, &Files);
  if (EFI_ERROR(Status)) {
    Print(L"firmware files unavailable; continuing without upload: ");
    EdgerunPrintStatus(Status);
    Print(L"\r\n");
    return EFI_SUCCESS;
  }

  Status = EdgerunParseRtl8922aFirmware(&Files, &Version, RomVersion, KeyId, &Patch);
  if (EFI_ERROR(Status)) {
    Print(L"Realtek firmware parse failed: ");
    EdgerunPrintStatus(Status);
    Print(L"\r\n");
    EdgerunFreeFirmwareFiles(&Files);
    return Status;
  }

  Print(L"Realtek patch image bytes=%u\r\n", (UINT32)Patch.Len);
  Status = EdgerunDownloadRtlFirmware(Device, Patch.Data, Patch.Len);
  EdgerunFreePatchImage(&Patch);
  EdgerunFreeFirmwareFiles(&Files);
  if (EFI_ERROR(Status)) {
    return Status;
  }

  Print(L"firmware download completed; resetting controller\r\n");
  Status = EdgerunHciReset(Device);
  if (EFI_ERROR(Status)) {
    return Status;
  }

  ZeroMem(&Version, sizeof(Version));
  Status = EdgerunHciReadLocalVersion(Device, &Version);
  if (EFI_ERROR(Status)) {
    return Status;
  }
  PrintLocalVersion(&Version);

  return EFI_SUCCESS;
}
