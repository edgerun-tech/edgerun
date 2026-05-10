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

  if (EventLen < 7) {
    return EFI_DEVICE_ERROR;
  }

  *RomVersion = Event[6];
  return EFI_SUCCESS;
}

STATIC
EFI_STATUS
UploadRtl8922aFirmwareStub(
  IN OUT EDGERUN_BT_USB *Device,
  IN UINT8 RomVersion,
  IN CONST EDGERUN_HCI_LOCAL_VERSION *Version
  )
{
  (VOID)Device;
  (VOID)RomVersion;
  (VOID)Version;

  Print(L"RTL8922A firmware upload not implemented yet\r\n");
  Print(L"Next: EPATCH parse + rtl8922au_config apply + vendor chunks through 0xfc20\r\n");

  return EFI_SUCCESS;
}

EFI_STATUS
EdgerunRealtekInitRtl8922A(
  IN OUT EDGERUN_BT_USB *Device
  )
{
  EFI_STATUS Status;
  EDGERUN_HCI_LOCAL_VERSION Version;
  UINT8 RomVersion;

  if (Device == NULL || Device->UsbIo == NULL) {
    return EFI_INVALID_PARAMETER;
  }

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

  if (Version.Manufacturer != RTL_COMPANY_ID) {
    Print(L"warning: HCI manufacturer is not Realtek (expected 0x%04x)\r\n", RTL_COMPANY_ID);
  }

  if (Version.LmpPalSubversion != RTL_ROM_LMP_8922A) {
    Print(L"warning: LMP subversion is not RTL8922A (expected 0x%04x)\r\n", RTL_ROM_LMP_8922A);
  }

  RomVersion = 0;
  Status = ReadRealtekRomVersion(Device, &RomVersion);
  if (EFI_ERROR(Status)) {
    Print(L"Realtek ROM version read failed: ");
    EdgerunPrintStatus(Status);
    Print(L"\r\n");
  } else {
    Print(L"Realtek ROM version=0x%02x\r\n", RomVersion);
  }

  Status = UploadRtl8922aFirmwareStub(Device, RomVersion, &Version);
  if (EFI_ERROR(Status)) {
    return Status;
  }

  return EFI_SUCCESS;
}
