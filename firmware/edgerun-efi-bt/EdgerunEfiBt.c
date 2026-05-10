#include "EdgerunEfiBt.h"

STATIC
VOID
PrintVersion(
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

VOID
EdgerunPrintStatus(
  IN EFI_STATUS Status
  )
{
  if (!EFI_ERROR(Status)) {
    Print(L"EFI_SUCCESS");
    return;
  }
  Print(L"0x%lx", Status);
}

VOID
EdgerunPrintBytesReverse(
  IN CONST UINT8 *Bytes,
  IN UINTN Len,
  IN CHAR16 Separator
  )
{
  UINTN Index;

  for (Index = Len; Index > 0; Index--) {
    Print(L"%02x", Bytes[Index - 1]);
    if (Index > 1 && Separator != 0) {
      Print(L"%c", Separator);
    }
  }
}

EFI_STATUS
EFIAPI
UefiMain(
  IN EFI_HANDLE ImageHandle,
  IN EFI_SYSTEM_TABLE *SystemTable
  )
{
  EFI_STATUS Status;
  EDGERUN_BT_USB Device;
  EDGERUN_HCI_LOCAL_VERSION Version;
  UINT8 BdAddr[6];

  (VOID)ImageHandle;
  (VOID)SystemTable;

  Print(L"\r\nEdgeRun UEFI Bluetooth bring-up (C/EDK II)\r\n");
  Print(L"target: Realtek RTL8922AE USB Bluetooth HCI\r\n");
  Print(L"mode: visible diagnostic app; no disk/NVRAM writes\r\n\r\n");

  ZeroMem(&Device, sizeof(Device));

  Status = EdgerunFindRtl8922UsbBluetooth(&Device);
  if (EFI_ERROR(Status)) {
    Print(L"RTL8922AE Bluetooth USB function not found: ");
    EdgerunPrintStatus(Status);
    Print(L"\r\n");
    return Status;
  }

  Print(
    L"USB Bluetooth: vid=0x%04x pid=0x%04x intf=0x%02x event=0x%02x acl_out=0x%02x acl_in=0x%02x\r\n",
    Device.VendorId,
    Device.ProductId,
    Device.InterfaceNumber,
    Device.EventEndpoint,
    Device.AclOutEndpoint,
    Device.AclInEndpoint
    );

  Status = EdgerunRealtekInitRtl8922A(&Device);
  if (EFI_ERROR(Status)) {
    Print(L"Realtek init failed: ");
    EdgerunPrintStatus(Status);
    Print(L"\r\n");
    return Status;
  }

  Status = EdgerunHciReadLocalVersion(&Device, &Version);
  if (EFI_ERROR(Status)) {
    Print(L"Read local version failed after init: ");
    EdgerunPrintStatus(Status);
    Print(L"\r\n");
    return Status;
  }
  PrintVersion(&Version);

  Status = EdgerunHciReadBdAddr(&Device, BdAddr);
  if (EFI_ERROR(Status)) {
    Print(L"Read BD_ADDR failed: ");
    EdgerunPrintStatus(Status);
    Print(L"\r\n");
    return Status;
  }

  Print(L"BD_ADDR=");
  EdgerunPrintBytesReverse(BdAddr, sizeof(BdAddr), L':');
  Print(L"\r\n");

  Status = EdgerunBleStartAdvertising(&Device, "ER-EFI-ADMIN");
  if (EFI_ERROR(Status)) {
    Print(L"BLE advertising failed: ");
    EdgerunPrintStatus(Status);
    Print(L"\r\n");
    return Status;
  }

  Print(L"BLE advertising enabled as ER-EFI-ADMIN\r\n");
  Print(L"Controller alive; staying in firmware loop.\r\n");

  while (TRUE) {
    gBS->Stall(250000);
  }
}
