#include "EdgerunEfiBt.h"

STATIC
EFI_STATUS
OpenBootVolume(
  IN EFI_HANDLE ImageHandle,
  OUT EFI_FILE_PROTOCOL **Root
  )
{
  EFI_STATUS Status;
  EFI_LOADED_IMAGE_PROTOCOL *LoadedImage;
  EFI_SIMPLE_FILE_SYSTEM_PROTOCOL *FileSystem;

  if (ImageHandle == NULL || Root == NULL) {
    return EFI_INVALID_PARAMETER;
  }

  *Root = NULL;
  LoadedImage = NULL;
  Status = gBS->HandleProtocol(
                  ImageHandle,
                  &gEfiLoadedImageProtocolGuid,
                  (VOID **)&LoadedImage
                  );
  if (EFI_ERROR(Status)) {
    return Status;
  }

  FileSystem = NULL;
  Status = gBS->HandleProtocol(
                  LoadedImage->DeviceHandle,
                  &gEfiSimpleFileSystemProtocolGuid,
                  (VOID **)&FileSystem
                  );
  if (EFI_ERROR(Status)) {
    return Status;
  }

  return FileSystem->OpenVolume(FileSystem, Root);
}

EFI_STATUS
EdgerunLoadFileFromBootVolume(
  IN EFI_HANDLE ImageHandle,
  IN CONST CHAR16 *Path,
  OUT UINT8 **Data,
  OUT UINTN *DataLen
  )
{
  EFI_STATUS Status;
  EFI_FILE_PROTOCOL *Root;
  EFI_FILE_PROTOCOL *File;
  EFI_FILE_INFO *Info;
  UINTN InfoSize;
  UINTN ReadSize;
  UINT8 *Buffer;

  if (Path == NULL || Data == NULL || DataLen == NULL) {
    return EFI_INVALID_PARAMETER;
  }

  *Data = NULL;
  *DataLen = 0;
  Root = NULL;
  File = NULL;
  Info = NULL;
  Buffer = NULL;

  Status = OpenBootVolume(ImageHandle, &Root);
  if (EFI_ERROR(Status)) {
    return Status;
  }

  Status = Root->Open(Root, &File, (CHAR16 *)Path, EFI_FILE_MODE_READ, 0);
  if (EFI_ERROR(Status)) {
    Root->Close(Root);
    return Status;
  }

  InfoSize = 0;
  Status = File->GetInfo(File, &gEfiFileInfoGuid, &InfoSize, NULL);
  if (Status != EFI_BUFFER_TOO_SMALL || InfoSize == 0) {
    File->Close(File);
    Root->Close(Root);
    return EFI_DEVICE_ERROR;
  }

  Info = AllocateZeroPool(InfoSize);
  if (Info == NULL) {
    File->Close(File);
    Root->Close(Root);
    return EFI_OUT_OF_RESOURCES;
  }

  Status = File->GetInfo(File, &gEfiFileInfoGuid, &InfoSize, Info);
  if (EFI_ERROR(Status)) {
    FreePool(Info);
    File->Close(File);
    Root->Close(Root);
    return Status;
  }

  if (Info->FileSize == 0 || Info->FileSize > MAX_UINTN) {
    FreePool(Info);
    File->Close(File);
    Root->Close(Root);
    return EFI_BAD_BUFFER_SIZE;
  }

  Buffer = AllocateZeroPool((UINTN)Info->FileSize);
  if (Buffer == NULL) {
    FreePool(Info);
    File->Close(File);
    Root->Close(Root);
    return EFI_OUT_OF_RESOURCES;
  }

  ReadSize = (UINTN)Info->FileSize;
  Status = File->Read(File, &ReadSize, Buffer);
  if (EFI_ERROR(Status) || ReadSize != (UINTN)Info->FileSize) {
    if (!EFI_ERROR(Status)) {
      Status = EFI_DEVICE_ERROR;
    }
    FreePool(Buffer);
    FreePool(Info);
    File->Close(File);
    Root->Close(Root);
    return Status;
  }

  *Data = Buffer;
  *DataLen = ReadSize;

  FreePool(Info);
  File->Close(File);
  Root->Close(Root);
  return EFI_SUCCESS;
}

VOID
EdgerunFreeFirmwareFiles(
  IN OUT EDGERUN_RTL_FIRMWARE_FILES *Files
  )
{
  if (Files == NULL) {
    return;
  }
  if (Files->FwData != NULL) {
    FreePool(Files->FwData);
  }
  if (Files->CfgData != NULL) {
    FreePool(Files->CfgData);
  }
  ZeroMem(Files, sizeof(*Files));
}

STATIC
EFI_STATUS
LoadFirstExisting(
  IN EFI_HANDLE ImageHandle,
  IN CONST CHAR16 *Path0,
  IN CONST CHAR16 *Path1,
  IN CONST CHAR16 *Path2,
  IN CONST CHAR16 *Path3,
  OUT UINT8 **Data,
  OUT UINTN *DataLen
  )
{
  EFI_STATUS Status;

  Status = EdgerunLoadFileFromBootVolume(ImageHandle, Path0, Data, DataLen);
  if (!EFI_ERROR(Status)) {
    return Status;
  }
  Status = EdgerunLoadFileFromBootVolume(ImageHandle, Path1, Data, DataLen);
  if (!EFI_ERROR(Status)) {
    return Status;
  }
  Status = EdgerunLoadFileFromBootVolume(ImageHandle, Path2, Data, DataLen);
  if (!EFI_ERROR(Status)) {
    return Status;
  }
  return EdgerunLoadFileFromBootVolume(ImageHandle, Path3, Data, DataLen);
}

EFI_STATUS
EdgerunLoadRtl8922aFirmwareFiles(
  IN EFI_HANDLE ImageHandle,
  OUT EDGERUN_RTL_FIRMWARE_FILES *Files
  )
{
  EFI_STATUS Status;

  if (Files == NULL) {
    return EFI_INVALID_PARAMETER;
  }

  ZeroMem(Files, sizeof(*Files));

  Status = LoadFirstExisting(
             ImageHandle,
             L"\\EFI\\BOOT\\firmware\\rtl8922au_fw",
             L"\\EFI\\BOOT\\firmware\\rtl8922au_fw.bin",
             L"\\firmware\\rtl8922au_fw",
             L"\\firmware\\rtl8922au_fw.bin",
             &Files->FwData,
             &Files->FwLen
             );
  if (EFI_ERROR(Status)) {
    Print(L"rtl8922au_fw not found on boot volume\r\n");
    return Status;
  }

  Status = LoadFirstExisting(
             ImageHandle,
             L"\\EFI\\BOOT\\firmware\\rtl8922au_config",
             L"\\EFI\\BOOT\\firmware\\rtl8922au_config.bin",
             L"\\firmware\\rtl8922au_config",
             L"\\firmware\\rtl8922au_config.bin",
             &Files->CfgData,
             &Files->CfgLen
             );
  if (EFI_ERROR(Status)) {
    Files->CfgData = NULL;
    Files->CfgLen = 0;
    Print(L"rtl8922au_config not found; continuing without config\r\n");
  }

  Print(L"loaded rtl8922au_fw bytes=%u\r\n", (UINT32)Files->FwLen);
  if (Files->CfgData != NULL) {
    Print(L"loaded rtl8922au_config bytes=%u\r\n", (UINT32)Files->CfgLen);
  }

  return EFI_SUCCESS;
}
