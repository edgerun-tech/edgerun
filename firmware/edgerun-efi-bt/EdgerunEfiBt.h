#ifndef EDGERUN_EFI_BT_H_
#define EDGERUN_EFI_BT_H_

#include <Uefi.h>
#include <Protocol/UsbIo.h>
#include <Protocol/LoadedImage.h>
#include <Protocol/SimpleFileSystem.h>
#include <Guid/FileInfo.h>
#include <Library/UefiLib.h>
#include <Library/UefiBootServicesTableLib.h>
#include <Library/BaseMemoryLib.h>
#include <Library/MemoryAllocationLib.h>
#include <Library/BaseLib.h>

#define EDGERUN_RTL_VENDOR_ID          0x0BDA
#define EDGERUN_RTL8922_PRODUCT_ID     0x8922

#define EDGERUN_USB_CLASS_WIRELESS     0xE0
#define EDGERUN_USB_SUBCLASS_RF        0x01
#define EDGERUN_USB_PROTOCOL_BT        0x01

#define EDGERUN_HCI_EVENT_EP_DEFAULT   0x81
#define EDGERUN_HCI_ACL_OUT_EP_DEFAULT 0x02
#define EDGERUN_HCI_ACL_IN_EP_DEFAULT  0x82

#define HCI_OPCODE(ogf, ocf) ((UINT16)((((UINT16)(ogf)) << 10) | ((UINT16)(ocf))))

#define HCI_OGF_CONTROLLER_BASEBAND 0x03
#define HCI_OGF_INFORMATIONAL       0x04
#define HCI_OGF_LE                  0x08
#define HCI_OGF_VENDOR              0x3F

#define HCI_OCF_RESET               0x0003
#define HCI_OCF_READ_LOCAL_VERSION  0x0001
#define HCI_OCF_READ_BD_ADDR        0x0009

#define HCI_OP_RESET                HCI_OPCODE(HCI_OGF_CONTROLLER_BASEBAND, HCI_OCF_RESET)
#define HCI_OP_READ_LOCAL_VERSION   HCI_OPCODE(HCI_OGF_INFORMATIONAL, HCI_OCF_READ_LOCAL_VERSION)
#define HCI_OP_READ_BD_ADDR         HCI_OPCODE(HCI_OGF_INFORMATIONAL, HCI_OCF_READ_BD_ADDR)

#define HCI_EVT_COMMAND_COMPLETE    0x0E
#define HCI_EVT_COMMAND_STATUS      0x0F

#define HCI_OP_RTL_READ_ROM_VERSION HCI_OPCODE(HCI_OGF_VENDOR, 0x006D) // 0xFC6D
#define HCI_OP_RTL_DOWNLOAD_FW      HCI_OPCODE(HCI_OGF_VENDOR, 0x0020) // 0xFC20

#define RTL_COMPANY_ID              0x005D
#define RTL_ROM_LMP_8922A           0x8922
#define RTL8922A_HCI_REVISION       0x000A
#define RTL8922A_HCI_VERSION        0x0C
#define RTL8922A_PROJECT_ID         44

#define RTL_FRAG_LEN                252
#define RTL_EPATCH_SIGNATURE        "Realtech"
#define RTL_EPATCH_SIGNATURE_V2     "RTBTCore"
#define RTL_EXTENSION_SIG_0         0x51
#define RTL_EXTENSION_SIG_1         0x04
#define RTL_EXTENSION_SIG_2         0xFD
#define RTL_EXTENSION_SIG_3         0x77

#define RTL_PATCH_SNIPPETS          0x01
#define RTL_PATCH_DUMMY_HEADER      0x02
#define RTL_PATCH_SECURITY_HEADER   0x03

#define EDGERUN_USB_TIMEOUT_MS      1000
#define EDGERUN_HCI_EVENT_TIMEOUT_MS 50
#define EDGERUN_HCI_MAX_POLLS       200

typedef struct {
  EFI_USB_IO_PROTOCOL *UsbIo;
  UINT16 VendorId;
  UINT16 ProductId;
  UINT8 InterfaceNumber;
  UINT8 EventEndpoint;
  UINT8 AclOutEndpoint;
  UINT8 AclInEndpoint;
} EDGERUN_BT_USB;

typedef struct {
  UINT8 HciVersion;
  UINT16 HciRevision;
  UINT8 LmpPalVersion;
  UINT16 Manufacturer;
  UINT16 LmpPalSubversion;
} EDGERUN_HCI_LOCAL_VERSION;

typedef struct {
  UINT8 *FwData;
  UINTN FwLen;
  UINT8 *CfgData;
  UINTN CfgLen;
} EDGERUN_RTL_FIRMWARE_FILES;

typedef struct {
  UINT8 *Data;
  UINTN Len;
} EDGERUN_RTL_PATCH_IMAGE;

EFI_STATUS
EdgerunFindRtl8922UsbBluetooth(
  OUT EDGERUN_BT_USB *Device
  );

EFI_STATUS
EdgerunLoadFileFromBootVolume(
  IN EFI_HANDLE ImageHandle,
  IN CONST CHAR16 *Path,
  OUT UINT8 **Data,
  OUT UINTN *DataLen
  );

VOID
EdgerunFreeFirmwareFiles(
  IN OUT EDGERUN_RTL_FIRMWARE_FILES *Files
  );

EFI_STATUS
EdgerunLoadRtl8922aFirmwareFiles(
  IN EFI_HANDLE ImageHandle,
  OUT EDGERUN_RTL_FIRMWARE_FILES *Files
  );

EFI_STATUS
EdgerunHciCommand(
  IN OUT EDGERUN_BT_USB *Device,
  IN UINT16 Opcode,
  IN CONST UINT8 *Params,
  IN UINTN ParamsLen,
  OUT UINT8 *Event,
  IN OUT UINTN *EventLen
  );

EFI_STATUS
EdgerunHciReset(
  IN OUT EDGERUN_BT_USB *Device
  );

EFI_STATUS
EdgerunHciReadLocalVersion(
  IN OUT EDGERUN_BT_USB *Device,
  OUT EDGERUN_HCI_LOCAL_VERSION *Version
  );

EFI_STATUS
EdgerunHciReadBdAddr(
  IN OUT EDGERUN_BT_USB *Device,
  OUT UINT8 BdAddr[6]
  );

EFI_STATUS
EdgerunRealtekInitRtl8922A(
  IN OUT EDGERUN_BT_USB *Device,
  IN EFI_HANDLE ImageHandle
  );

EFI_STATUS
EdgerunParseRtl8922aFirmware(
  IN CONST EDGERUN_RTL_FIRMWARE_FILES *Files,
  IN CONST EDGERUN_HCI_LOCAL_VERSION *Version,
  IN UINT8 RomVersion,
  OUT EDGERUN_RTL_PATCH_IMAGE *Patch
  );

EFI_STATUS
EdgerunDownloadRtlFirmware(
  IN OUT EDGERUN_BT_USB *Device,
  IN CONST UINT8 *Data,
  IN UINTN DataLen
  );

VOID
EdgerunFreePatchImage(
  IN OUT EDGERUN_RTL_PATCH_IMAGE *Patch
  );

EFI_STATUS
EdgerunBleStartAdvertising(
  IN OUT EDGERUN_BT_USB *Device,
  IN CONST CHAR8 *Name
  );

VOID
EdgerunPrintStatus(
  IN EFI_STATUS Status
  );

VOID
EdgerunPrintBytesReverse(
  IN CONST UINT8 *Bytes,
  IN UINTN Len,
  IN CHAR16 Separator
  );

#endif
