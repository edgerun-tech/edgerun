#include "EdgerunEfiBt.h"

typedef struct {
  CONST UINT8 *Data;
  UINTN Len;
  UINT8 Prio;
} RTL_SUBSECTION_REF;

STATIC
UINT16
ReadLe16(
  IN CONST UINT8 *Data
  )
{
  return (UINT16)(Data[0] | ((UINT16)Data[1] << 8));
}

STATIC
UINT32
ReadLe32(
  IN CONST UINT8 *Data
  )
{
  return (UINT32)Data[0] |
         ((UINT32)Data[1] << 8) |
         ((UINT32)Data[2] << 16) |
         ((UINT32)Data[3] << 24);
}

STATIC
BOOLEAN
BytesEqual(
  IN CONST UINT8 *Data,
  IN CONST CHAR8 *Text,
  IN UINTN Len
  )
{
  UINTN Index;

  for (Index = 0; Index < Len; Index++) {
    if (Data[Index] != (UINT8)Text[Index]) {
      return FALSE;
    }
  }
  return TRUE;
}

STATIC
BOOLEAN
HasExtensionSignature(
  IN CONST UINT8 *Fw,
  IN UINTN FwLen
  )
{
  CONST UINT8 *Tail;

  if (Fw == NULL || FwLen < 4) {
    return FALSE;
  }

  Tail = Fw + FwLen - 4;
  return Tail[0] == RTL_EXTENSION_SIG_0 &&
         Tail[1] == RTL_EXTENSION_SIG_1 &&
         Tail[2] == RTL_EXTENSION_SIG_2 &&
         Tail[3] == RTL_EXTENSION_SIG_3;
}

STATIC
EFI_STATUS
FindProjectId(
  IN CONST UINT8 *Fw,
  IN UINTN FwLen,
  IN UINTN HeaderLen,
  OUT UINT8 *ProjectId
  )
{
  CONST UINT8 *Cursor;
  CONST UINT8 *Limit;

  if (Fw == NULL || ProjectId == NULL || FwLen < HeaderLen + 7) {
    return EFI_INVALID_PARAMETER;
  }
  if (!HasExtensionSignature(Fw, FwLen)) {
    return EFI_COMPROMISED_DATA;
  }

  Cursor = Fw + FwLen - 4;
  Limit = Fw + HeaderLen + 3;

  while (Cursor >= Limit) {
    UINT8 Opcode;
    UINT8 Length;
    UINT8 Data;

    Opcode = *--Cursor;
    Length = *--Cursor;
    Data = *--Cursor;

    if (Opcode == 0xFF) {
      break;
    }
    if (Length == 0) {
      return EFI_COMPROMISED_DATA;
    }
    if (Opcode == 0 && Length == 1) {
      *ProjectId = Data;
      return EFI_SUCCESS;
    }
    if ((UINTN)(Cursor - Fw) < Length) {
      return EFI_COMPROMISED_DATA;
    }
    Cursor -= Length;
  }

  return EFI_NOT_FOUND;
}

STATIC
EFI_STATUS
ValidateRtl8922aProject(
  IN UINT8 ProjectId,
  IN CONST EDGERUN_HCI_LOCAL_VERSION *Version
  )
{
  if (ProjectId != RTL8922A_PROJECT_ID) {
    Print(L"firmware project id %u is not RTL8922A project %u\r\n", ProjectId, RTL8922A_PROJECT_ID);
    return EFI_UNSUPPORTED;
  }

  if (Version != NULL && Version->LmpPalSubversion != RTL_ROM_LMP_8922A) {
    Print(
      L"firmware targets RTL8922A but controller LMP subversion is 0x%04x\r\n",
      Version->LmpPalSubversion
      );
    return EFI_UNSUPPORTED;
  }

  return EFI_SUCCESS;
}

STATIC
EFI_STATUS
BuildPatchImageWithOptionalConfig(
  IN CONST UINT8 *PatchData,
  IN UINTN PatchLen,
  IN CONST UINT8 *ConfigData,
  IN UINTN ConfigLen,
  OUT EDGERUN_RTL_PATCH_IMAGE *Patch
  )
{
  UINTN OutLen;
  UINT8 *Out;

  if (PatchData == NULL || Patch == NULL || PatchLen < 4) {
    return EFI_INVALID_PARAMETER;
  }

  ZeroMem(Patch, sizeof(*Patch));

  if (ConfigData == NULL || ConfigLen == 0) {
    Out = AllocateZeroPool(PatchLen);
    if (Out == NULL) {
      return EFI_OUT_OF_RESOURCES;
    }
    CopyMem(Out, PatchData, PatchLen);
    Patch->Data = Out;
    Patch->Len = PatchLen;
    return EFI_SUCCESS;
  }

  if (ConfigLen > MAX_UINTN - PatchLen) {
    return EFI_BAD_BUFFER_SIZE;
  }

  OutLen = PatchLen + ConfigLen;
  Out = AllocateZeroPool(OutLen);
  if (Out == NULL) {
    return EFI_OUT_OF_RESOURCES;
  }

  // Realtek patch payloads keep a four-byte trailer at the end. Config data is
  // inserted immediately before that trailer, matching the observed Linux flow.
  CopyMem(Out, PatchData, PatchLen - 4);
  CopyMem(Out + PatchLen - 4, ConfigData, ConfigLen);
  CopyMem(Out + PatchLen - 4 + ConfigLen, PatchData + PatchLen - 4, 4);

  Patch->Data = Out;
  Patch->Len = OutLen;
  return EFI_SUCCESS;
}

STATIC
EFI_STATUS
ParseLegacyEpatch(
  IN CONST EDGERUN_RTL_FIRMWARE_FILES *Files,
  IN CONST EDGERUN_HCI_LOCAL_VERSION *Version,
  IN UINT8 RomVersion,
  OUT EDGERUN_RTL_PATCH_IMAGE *Patch
  )
{
  CONST UINT8 *Fw;
  UINTN FwLen;
  UINT8 ProjectId;
  UINT16 NumPatches;
  UINTN MetaOff;
  UINTN Needed;
  CONST UINT8 *ChipIdBase;
  CONST UINT8 *PatchLenBase;
  CONST UINT8 *PatchOffBase;
  UINT16 TargetChipId;
  UINTN Index;

  Fw = Files->FwData;
  FwLen = Files->FwLen;

  if (FwLen < 14 || !BytesEqual(Fw, RTL_EPATCH_SIGNATURE, 8)) {
    return EFI_COMPROMISED_DATA;
  }

  ProjectId = 0;
  EFI_STATUS Status = FindProjectId(Fw, FwLen, 14, &ProjectId);
  if (EFI_ERROR(Status)) {
    return Status;
  }
  Status = ValidateRtl8922aProject(ProjectId, Version);
  if (EFI_ERROR(Status)) {
    return Status;
  }

  NumPatches = ReadLe16(Fw + 12);
  MetaOff = 14;
  Needed = MetaOff + ((UINTN)NumPatches * 2) + ((UINTN)NumPatches * 2) + ((UINTN)NumPatches * 4);
  if (NumPatches == 0 || Needed > FwLen) {
    return EFI_COMPROMISED_DATA;
  }

  ChipIdBase = Fw + MetaOff;
  PatchLenBase = ChipIdBase + ((UINTN)NumPatches * 2);
  PatchOffBase = PatchLenBase + ((UINTN)NumPatches * 2);
  TargetChipId = (UINT16)(RomVersion + 1);

  for (Index = 0; Index < NumPatches; Index++) {
    UINT16 ChipId;
    UINT16 PatchLen;
    UINT32 PatchOff;

    ChipId = ReadLe16(ChipIdBase + Index * 2);
    if (ChipId != TargetChipId) {
      continue;
    }

    PatchLen = ReadLe16(PatchLenBase + Index * 2);
    PatchOff = ReadLe32(PatchOffBase + Index * 4);
    if (PatchLen < 4 || PatchOff > FwLen || PatchLen > FwLen - PatchOff) {
      return EFI_COMPROMISED_DATA;
    }

    Print(
      L"legacy EPATCH selected chip_id=0x%04x patch_off=0x%08x patch_len=%u\r\n",
      ChipId,
      PatchOff,
      PatchLen
      );
    return BuildPatchImageWithOptionalConfig(
             Fw + PatchOff,
             PatchLen,
             Files->CfgData,
             Files->CfgLen,
             Patch
             );
  }

  Print(L"legacy EPATCH has no chip_id=0x%04x patch for ROM version 0x%02x\r\n", TargetChipId, RomVersion);
  return EFI_NOT_FOUND;
}

STATIC
EFI_STATUS
InsertSubsection(
  IN OUT RTL_SUBSECTION_REF *Sections,
  IN OUT UINTN *SectionCount,
  IN UINTN MaxSections,
  IN CONST UINT8 *Data,
  IN UINTN Len,
  IN UINT8 Prio
  )
{
  UINTN Pos;

  if (*SectionCount >= MaxSections) {
    return EFI_OUT_OF_RESOURCES;
  }

  Pos = 0;
  while (Pos < *SectionCount && Sections[Pos].Prio <= Prio) {
    Pos++;
  }

  if (Pos < *SectionCount) {
    CopyMem(&Sections[Pos + 1], &Sections[Pos], (*SectionCount - Pos) * sizeof(Sections[0]));
  }

  Sections[Pos].Data = Data;
  Sections[Pos].Len = Len;
  Sections[Pos].Prio = Prio;
  *SectionCount += 1;
  return EFI_SUCCESS;
}

STATIC
EFI_STATUS
ParseV2Subsections(
  IN UINT32 Opcode,
  IN CONST UINT8 *Data,
  IN UINTN Len,
  IN UINT8 RomVersion,
  IN OUT RTL_SUBSECTION_REF *Sections,
  IN OUT UINTN *SectionCount,
  IN UINTN MaxSections
  )
{
  UINT16 Count;
  UINTN Offset;
  UINTN Index;

  if (Data == NULL || Len < 4) {
    return EFI_COMPROMISED_DATA;
  }

  Count = ReadLe16(Data);
  Offset = 4;

  for (Index = 0; Index < Count; Index++) {
    UINT8 Eco;
    UINT8 Prio;
    UINTN PayloadLen;
    CONST UINT8 *Payload;

    if (Offset + 8 > Len) {
      return EFI_COMPROMISED_DATA;
    }

    Eco = Data[Offset + 0];
    Prio = Data[Offset + 1];
    PayloadLen = ReadLe32(Data + Offset + 4);
    Payload = Data + Offset + 8;

    if (PayloadLen > Len - Offset - 8) {
      return EFI_COMPROMISED_DATA;
    }

    if (Opcode == RTL_PATCH_SECURITY_HEADER) {
      // RTL8922A can carry security-header subsections. We do not yet read the
      // chip key id, so skip them instead of accidentally selecting the wrong
      // signed section.
      Offset += 8 + PayloadLen;
      continue;
    }

    if (Eco == (UINT8)(RomVersion + 1)) {
      EFI_STATUS Status;
      Status = InsertSubsection(Sections, SectionCount, MaxSections, Payload, PayloadLen, Prio);
      if (EFI_ERROR(Status)) {
        return Status;
      }
    }

    Offset += 8 + PayloadLen;
  }

  return EFI_SUCCESS;
}

STATIC
EFI_STATUS
BuildPatchFromSections(
  IN CONST RTL_SUBSECTION_REF *Sections,
  IN UINTN SectionCount,
  IN CONST UINT8 *ConfigData,
  IN UINTN ConfigLen,
  OUT EDGERUN_RTL_PATCH_IMAGE *Patch
  )
{
  UINTN Total;
  UINTN Index;
  UINT8 *Merged;
  UINTN Offset;
  EDGERUN_RTL_PATCH_IMAGE Raw;
  EFI_STATUS Status;

  if (Sections == NULL || SectionCount == 0 || Patch == NULL) {
    return EFI_NOT_FOUND;
  }

  Total = 0;
  for (Index = 0; Index < SectionCount; Index++) {
    if (Sections[Index].Len > MAX_UINTN - Total) {
      return EFI_BAD_BUFFER_SIZE;
    }
    Total += Sections[Index].Len;
  }
  if (Total < 4) {
    return EFI_COMPROMISED_DATA;
  }

  Merged = AllocateZeroPool(Total);
  if (Merged == NULL) {
    return EFI_OUT_OF_RESOURCES;
  }

  Offset = 0;
  for (Index = 0; Index < SectionCount; Index++) {
    CopyMem(Merged + Offset, Sections[Index].Data, Sections[Index].Len);
    Offset += Sections[Index].Len;
  }

  ZeroMem(&Raw, sizeof(Raw));
  Raw.Data = Merged;
  Raw.Len = Total;

  Status = BuildPatchImageWithOptionalConfig(Raw.Data, Raw.Len, ConfigData, ConfigLen, Patch);
  FreePool(Merged);
  return Status;
}

STATIC
EFI_STATUS
ParseV2Epatch(
  IN CONST EDGERUN_RTL_FIRMWARE_FILES *Files,
  IN CONST EDGERUN_HCI_LOCAL_VERSION *Version,
  IN UINT8 RomVersion,
  OUT EDGERUN_RTL_PATCH_IMAGE *Patch
  )
{
  CONST UINT8 *Fw;
  UINTN FwLen;
  UINT8 ProjectId;
  UINT32 NumSections;
  UINTN Offset;
  UINT32 Index;
  RTL_SUBSECTION_REF Sections[64];
  UINTN SectionCount;
  EFI_STATUS Status;

  Fw = Files->FwData;
  FwLen = Files->FwLen;

  if (FwLen < 20 || !BytesEqual(Fw, RTL_EPATCH_SIGNATURE_V2, 8)) {
    return EFI_COMPROMISED_DATA;
  }

  ProjectId = 0;
  Status = FindProjectId(Fw, FwLen, 20, &ProjectId);
  if (EFI_ERROR(Status)) {
    return Status;
  }
  Status = ValidateRtl8922aProject(ProjectId, Version);
  if (EFI_ERROR(Status)) {
    return Status;
  }

  NumSections = ReadLe32(Fw + 16);
  Offset = 20;
  ZeroMem(Sections, sizeof(Sections));
  SectionCount = 0;

  for (Index = 0; Index < NumSections; Index++) {
    UINT32 Opcode;
    UINT32 SectionLen;
    CONST UINT8 *SectionData;

    if (Offset + 8 > FwLen) {
      return EFI_COMPROMISED_DATA;
    }

    Opcode = ReadLe32(Fw + Offset);
    SectionLen = ReadLe32(Fw + Offset + 4);
    SectionData = Fw + Offset + 8;
    Offset += 8;

    if ((UINTN)SectionLen > FwLen - Offset) {
      return EFI_COMPROMISED_DATA;
    }

    if (Opcode == RTL_PATCH_SNIPPETS || Opcode == RTL_PATCH_DUMMY_HEADER || Opcode == RTL_PATCH_SECURITY_HEADER) {
      Status = ParseV2Subsections(Opcode, SectionData, SectionLen, RomVersion, Sections, &SectionCount, 64);
      if (EFI_ERROR(Status)) {
        return Status;
      }
    }

    Offset += SectionLen;
  }

  Print(L"RTBTCore selected %u subsection(s) for ROM eco 0x%02x\r\n", (UINT32)SectionCount, (UINT8)(RomVersion + 1));
  return BuildPatchFromSections(Sections, SectionCount, Files->CfgData, Files->CfgLen, Patch);
}

EFI_STATUS
EdgerunParseRtl8922aFirmware(
  IN CONST EDGERUN_RTL_FIRMWARE_FILES *Files,
  IN CONST EDGERUN_HCI_LOCAL_VERSION *Version,
  IN UINT8 RomVersion,
  OUT EDGERUN_RTL_PATCH_IMAGE *Patch
  )
{
  if (Files == NULL || Files->FwData == NULL || Files->FwLen == 0 || Patch == NULL) {
    return EFI_INVALID_PARAMETER;
  }

  ZeroMem(Patch, sizeof(*Patch));

  if (Files->FwLen >= 8 && BytesEqual(Files->FwData, RTL_EPATCH_SIGNATURE_V2, 8)) {
    return ParseV2Epatch(Files, Version, RomVersion, Patch);
  }
  if (Files->FwLen >= 8 && BytesEqual(Files->FwData, RTL_EPATCH_SIGNATURE, 8)) {
    return ParseLegacyEpatch(Files, Version, RomVersion, Patch);
  }

  Print(L"unknown Realtek firmware signature\r\n");
  return EFI_COMPROMISED_DATA;
}

EFI_STATUS
EdgerunDownloadRtlFirmware(
  IN OUT EDGERUN_BT_USB *Device,
  IN CONST UINT8 *Data,
  IN UINTN DataLen
  )
{
  UINTN FragCount;
  UINTN FragIndex;
  UINTN Offset;
  UINT8 DownloadIndex;

  if (Device == NULL || Data == NULL || DataLen == 0) {
    return EFI_INVALID_PARAMETER;
  }

  FragCount = (DataLen + RTL_FRAG_LEN - 1) / RTL_FRAG_LEN;
  Offset = 0;
  DownloadIndex = 0;

  Print(L"downloading Realtek firmware bytes=%u fragments=%u\r\n", (UINT32)DataLen, (UINT32)FragCount);

  for (FragIndex = 0; FragIndex < FragCount; FragIndex++) {
    EFI_STATUS Status;
    UINT8 Params[1 + RTL_FRAG_LEN];
    UINTN FragLen;
    UINT8 Event[260];
    UINTN EventLen;

    FragLen = DataLen - Offset;
    if (FragLen > RTL_FRAG_LEN) {
      FragLen = RTL_FRAG_LEN;
    }

    Params[0] = DownloadIndex;
    DownloadIndex++;
    if (DownloadIndex == 0x7F) {
      DownloadIndex = 1;
    }

    if (FragIndex == FragCount - 1) {
      Params[0] |= 0x80;
    }

    CopyMem(&Params[1], Data + Offset, FragLen);

    EventLen = sizeof(Event);
    Status = EdgerunHciCommand(Device, HCI_OP_RTL_DOWNLOAD_FW, Params, FragLen + 1, Event, &EventLen);
    if (EFI_ERROR(Status)) {
      Print(L"firmware fragment %u/%u failed: ", (UINT32)(FragIndex + 1), (UINT32)FragCount);
      EdgerunPrintStatus(Status);
      Print(L"\r\n");
      return Status;
    }

    if (EventLen < 7) {
      return EFI_DEVICE_ERROR;
    }

    Offset += FragLen;
  }

  return EFI_SUCCESS;
}

VOID
EdgerunFreePatchImage(
  IN OUT EDGERUN_RTL_PATCH_IMAGE *Patch
  )
{
  if (Patch == NULL) {
    return;
  }
  if (Patch->Data != NULL) {
    FreePool(Patch->Data);
  }
  ZeroMem(Patch, sizeof(*Patch));
}
