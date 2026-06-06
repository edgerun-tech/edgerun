(module
  (memory (export "memory") 576)
  (global $VORBIS_OK i32 (i32.const 0))
  (global $VORBIS_ERR_BOUNDS i32 (i32.const -1))
  (global $VORBIS_ERR_BAD_ARCHIVE i32 (i32.const -2))
  (global $VORBIS_ERR_TOO_MANY_PACKETS i32 (i32.const -3))
  (global $VORBIS_ERR_PCM_TOO_LARGE i32 (i32.const -4))
  (global $VORBIS_ERR_TODO_DECODE i32 (i32.const -5))
  (global $VORBIS_ERR_TOO_MANY_BOOKS i32 (i32.const -6))
  (global $VORBIS_ERR_TOO_MANY_ENTRIES i32 (i32.const -7))
  (global $VORBIS_ERR_BAD_CODEBOOK i32 (i32.const -8))
  (global $VORBIS_ERR_BAD_INDEX i32 (i32.const -9))
  (global $VORBIS_ERR_TOO_MANY_SETUP i32 (i32.const -10))
  (global $VORBIS_MAX_PACKETS i32 (i32.const 1024))
  (global $VORBIS_MAX_PCM_FRAMES i32 (i32.const 262144))
  (global $VORBIS_MAX_CODEBOOKS i32 (i32.const 64))
  (global $VORBIS_MAX_CODEBOOK_ENTRIES i32 (i32.const 8192))
  (global $VORBIS_MAX_MULTIPLICANDS i32 (i32.const 512))
  (global $VORBIS_CODEBOOK_SYNC i32 (i32.const 5653314))
  (global $VORBIS_MAX_SETUP_OBJS i32 (i32.const 64))
  (global $VORBIS_MAX_FLOOR_PARTS i32 (i32.const 64))
  (global $VORBIS_MAX_FLOOR_CLASSES i32 (i32.const 64))
  (global $VORBIS_MAX_FLOOR_VALUES i32 (i32.const 512))
  (global $VORBIS_MAX_RESIDUE_CLASSES i32 (i32.const 64))
  (global $VORBIS_MAX_RESIDUE_BOOKS i32 (i32.const 512))
  (global $VORBIS_MAX_MAPPING_SUBMAPS i32 (i32.const 16))
  (global $VORBIS_MAX_BLOCK_SIZE i32 (i32.const 8192))
  (global $VORBIS_MAX_OVERLAP_FRAMES i32 (i32.const 4096))
  (global $VORBIS_MAX_PACKET_FLOOR_CLASSES i32 (i32.const 65536))
  (global $VORBIS_MAX_PACKET_FLOOR_VALUES i32 (i32.const 524288))
  (global $VORBIS_MAX_PACKET_FLOOR_SEGMENTS i32 (i32.const 524288))
  (global $VORBIS_MAX_PACKET_RESIDUE_PARTS i32 (i32.const 524288))
  (global $VORBIS_MAX_PACKET_RESIDUE_VALUES i32 (i32.const 524288))
  (global $VORBIS_PACKET_RESIDUE_PARTS_PER_PACKET i32 (i32.const 512))
  (global $VORBIS_PACKET_RESIDUE_VALUES_PER_PACKET i32 (i32.const 512))
  (global $SIZEOF_SETUP_STATE i32 (i32.const 68))
  (global $SIZEOF_SAMPLE_STATE i32 (i32.const 12328))
  (global $SIZEOF_RAWSOUND_VIEW i32 (i32.const 28))
  (global $SIZEOF_BITREADER i32 (i32.const 32))
  (global $SIZEOF_FLOOR_HEADER i32 (i32.const 32))
  (global $SIZEOF_RESIDUE_HEADER i32 (i32.const 28))
  (global $SIZEOF_MAPPING_HEADER i32 (i32.const 20))
  (global $SIZEOF_MODE_HEADER i32 (i32.const 8))
  (global $SIZEOF_CODEBOOK_HEADER i32 (i32.const 60))
  (global $BSS_SETUP_STATE i32 (i32.const 0x00000000))
  (global $BSS_SAMPLE_STATE i32 (i32.const 0x00000100))
  (global $BSS_RAWSOUND_VIEW i32 (i32.const 0x00003200))
  (global $BSS_BITREADER i32 (i32.const 0x00003300))
  (global $BSS_SILENCE_PCM i32 (i32.const 0x00003400))
  (global $BSS_FLOOR_RESIDUE_PCM i32 (i32.const 0x000C3400))
  (global $BSS_MDCT_SYNTH_PCM i32 (i32.const 0x00143400))
  (global $BSS_CODEBOOK_HEADERS i32 (i32.const 0x001C3400))
  (global $BSS_CODEBOOK_LENGTHS i32 (i32.const 0x001C4400))
  (global $BSS_CODEBOOK_CODES i32 (i32.const 0x001CC400))
  (global $BSS_CODEBOOK_MULTIPLICANDS i32 (i32.const 0x001D4400))
  (global $BSS_CANONICAL_TEMP i32 (i32.const 0x001D4C00))
  (global $BSS_FLOOR_HEADERS i32 (i32.const 0x001D5000))
  (global $BSS_FLOOR_PARTITIONS i32 (i32.const 0x001D5800))
  (global $BSS_FLOOR_CLASS_DIMS i32 (i32.const 0x001D5A00))
  (global $BSS_FLOOR_CLASS_SUBBITS i32 (i32.const 0x001D5C00))
  (global $BSS_FLOOR_CLASS_MASTER i32 (i32.const 0x001D5E00))
  (global $BSS_FLOOR_CLASS_BOOKS i32 (i32.const 0x001D6000))
  (global $BSS_FLOOR_VALUES i32 (i32.const 0x001D6800))
  (global $BSS_RESIDUE_HEADERS i32 (i32.const 0x001D7000))
  (global $BSS_RESIDUE_CASCADES i32 (i32.const 0x001D7800))
  (global $BSS_RESIDUE_BOOKS i32 (i32.const 0x001D7A00))
  (global $BSS_MAPPING_HEADERS i32 (i32.const 0x001D8200))
  (global $BSS_MAPPING_FLOORS i32 (i32.const 0x001D8800))
  (global $BSS_MAPPING_RESIDUES i32 (i32.const 0x001D9800))
  (global $BSS_MODE_HEADERS i32 (i32.const 0x001DA800))
  (global $BSS_PACKET_MODES i32 (i32.const 0x001DAC00))
  (global $BSS_PACKET_MAPPINGS i32 (i32.const 0x001DBC00))
  (global $BSS_PACKET_FLOORS i32 (i32.const 0x001DCC00))
  (global $BSS_PACKET_RESIDUES i32 (i32.const 0x001DDC00))
  (global $BSS_PACKET_FLOOR_NONZERO i32 (i32.const 0x001DEC00))
  (global $BSS_PACKET_FLOOR_Y0 i32 (i32.const 0x001DFC00))
  (global $BSS_PACKET_FLOOR_Y1 i32 (i32.const 0x001E0C00))
  (global $BSS_PACKET_FLOOR_CLASS_COUNT i32 (i32.const 0x001E1C00))
  (global $BSS_PACKET_FLOOR_CLASS_IDS i32 (i32.const 0x001E2C00))
  (global $BSS_PACKET_FLOOR_CLASS_SELECTORS i32 (i32.const 0x00222C00))
  (global $BSS_PACKET_FLOOR_CLASS_DIMS i32 (i32.const 0x00262C00))
  (global $BSS_PACKET_FLOOR_VALUE_BOOKS i32 (i32.const 0x002A2C00))
  (global $BSS_PACKET_FLOOR_VALUES i32 (i32.const 0x004A2C00))
  (global $BSS_PACKET_FLOOR_POINT_COUNT i32 (i32.const 0x006A2C00))
  (global $BSS_PACKET_FLOOR_POINT_X i32 (i32.const 0x006A3C00))
  (global $BSS_PACKET_FLOOR_POINT_Y i32 (i32.const 0x008A3C00))
  (global $BSS_PACKET_FLOOR_POINT_ORDER i32 (i32.const 0x00AA3C00))
  (global $BSS_PACKET_FLOOR_SEGMENT_COUNT i32 (i32.const 0x00CA3C00))
  (global $BSS_PACKET_FLOOR_SEGMENT_X0 i32 (i32.const 0x00CA4C00))
  (global $BSS_PACKET_FLOOR_SEGMENT_Y0 i32 (i32.const 0x00EA4C00))
  (global $BSS_PACKET_FLOOR_SEGMENT_X1 i32 (i32.const 0x010A4C00))
  (global $BSS_PACKET_FLOOR_SEGMENT_Y1 i32 (i32.const 0x012A4C00))
  (global $BSS_PACKET_RESIDUE_PART_COUNT i32 (i32.const 0x014A4C00))
  (global $BSS_PACKET_RESIDUE_CLASSIFICATIONS i32 (i32.const 0x014A5C00))
  (global $BSS_PACKET_RESIDUE_VALUE_COUNT i32 (i32.const 0x016A5C00))
  (global $BSS_PACKET_RESIDUE_VALUE_BOOKS i32 (i32.const 0x016A6C00))
  (global $BSS_PACKET_RESIDUE_VALUE_ENTRIES i32 (i32.const 0x018A6C00))
  (global $BSS_PACKET_RESIDUE_VALUE_DIMS i32 (i32.const 0x01AA6C00))
  (global $BSS_PACKET_RESIDUE_VALUE_TARGETS i32 (i32.const 0x01CA6C00))
  (global $BSS_PACKET_RESIDUE_VALUE_VALUES i32 (i32.const 0x01EA6C00))
  (global $BSS_PACKET_BLOCK_SIZES i32 (i32.const 0x020A6C00))
  (global $BSS_PACKET_PREV_WINDOW_FLAGS i32 (i32.const 0x020A7C00))
  (global $BSS_PACKET_NEXT_WINDOW_FLAGS i32 (i32.const 0x020A8C00))
  (global $BSS_PACKET_WINDOW_LEFT_START i32 (i32.const 0x020A9C00))
  (global $BSS_PACKET_WINDOW_LEFT_END i32 (i32.const 0x020AAC00))
  (global $BSS_PACKET_WINDOW_RIGHT_START i32 (i32.const 0x020ABC00))
  (global $BSS_PACKET_WINDOW_RIGHT_END i32 (i32.const 0x020ACC00))
  (global $BSS_PACKET_WINDOW_LEFT_FRAMES i32 (i32.const 0x020ADC00))
  (global $BSS_PACKET_WINDOW_RIGHT_FRAMES i32 (i32.const 0x020AEC00))
  (global $BSS_MDCT_OUTPUT_Q15 i32 (i32.const 0x020AFC00))
  (global $BSS_WINDOW_Q15 i32 (i32.const 0x020B7C00))
  (global $BSS_LAST_CODEBOOK_STATUS i32 (i32.const 0x020BFC00))
  (global $BSS_LAST_CODEBOOK_COUNT i32 (i32.const 0x020BFC04))
  (global $BSS_LAST_CODEBOOK_ENTRIES i32 (i32.const 0x020BFC08))
  (global $BSS_LAST_CODEBOOK_NONZERO i32 (i32.const 0x020BFC0C))
  (global $BSS_LAST_CODEBOOK_MAX_LEN i32 (i32.const 0x020BFC10))
  (global $BSS_LAST_LOOKUP_VALUES i32 (i32.const 0x020BFC14))
  (global $BSS_LAST_SETUP_BYTE i32 (i32.const 0x020BFC18))
  (global $BSS_LAST_SETUP_BIT i32 (i32.const 0x020BFC1C))
  (global $BSS_LAST_FLOOR_STATUS i32 (i32.const 0x020BFC20))
  (global $BSS_LAST_RESIDUE_STATUS i32 (i32.const 0x020BFC24))
  (global $BSS_LAST_MAPPING_STATUS i32 (i32.const 0x020BFC28))
  (global $BSS_LAST_MODE_STATUS i32 (i32.const 0x020BFC2C))
  (global $BSS_LAST_FLOOR_VALUES i32 (i32.const 0x020BFC30))
  (global $BSS_LAST_RESIDUE_BOOKS i32 (i32.const 0x020BFC34))
  (global $BSS_LAST_PACKET_STATUS i32 (i32.const 0x020BFC38))
  (global $BSS_LAST_PACKET_COUNT i32 (i32.const 0x020BFC3C))
  (global $BSS_LAST_PACKET_MODE i32 (i32.const 0x020BFC40))
  (global $BSS_LAST_PACKET_MAPPING i32 (i32.const 0x020BFC44))
  (global $BSS_LAST_PACKET_FLOOR i32 (i32.const 0x020BFC48))
  (global $BSS_LAST_PACKET_RESIDUE i32 (i32.const 0x020BFC4C))
  (global $BSS_LAST_MODE_BITS i32 (i32.const 0x020BFC50))
  (global $BSS_LAST_WINDOW_STATUS i32 (i32.const 0x020BFC54))
  (global $BSS_LAST_WINDOW_PACKET i32 (i32.const 0x020BFC58))
  (global $BSS_LAST_WINDOW_BLOCK_SIZE i32 (i32.const 0x020BFC5C))
  (global $BSS_LAST_WINDOW_PREV_FLAG i32 (i32.const 0x020BFC60))
  (global $BSS_LAST_WINDOW_NEXT_FLAG i32 (i32.const 0x020BFC64))
  (global $BSS_LAST_WINDOW_LEFT_START i32 (i32.const 0x020BFC68))
  (global $BSS_LAST_WINDOW_LEFT_END i32 (i32.const 0x020BFC6C))
  (global $BSS_LAST_WINDOW_RIGHT_START i32 (i32.const 0x020BFC70))
  (global $BSS_LAST_WINDOW_RIGHT_END i32 (i32.const 0x020BFC74))
  (global $BSS_LAST_WINDOW_LEFT_FRAMES i32 (i32.const 0x020BFC78))
  (global $BSS_LAST_WINDOW_RIGHT_FRAMES i32 (i32.const 0x020BFC7C))
  (global $BSS_LAST_MDCT_BUFFER_FRAMES i32 (i32.const 0x020BFC80))
  (global $BSS_LAST_OVERLAP_FRAMES i32 (i32.const 0x020BFC84))
  (global $BSS_LAST_FLOOR_PACKET_STATUS i32 (i32.const 0x020BFC88))
  (global $BSS_LAST_FLOOR_PACKET_COUNT i32 (i32.const 0x020BFC8C))
  (global $BSS_LAST_FLOOR_PACKET_NONZERO i32 (i32.const 0x020BFC90))
  (global $BSS_LAST_FLOOR_PACKET_Y0 i32 (i32.const 0x020BFC94))
  (global $BSS_LAST_FLOOR_PACKET_Y1 i32 (i32.const 0x020BFC98))
  (global $BSS_LAST_FLOOR_PACKET_Y_RANGE i32 (i32.const 0x020BFC9C))
  (global $BSS_LAST_FLOOR_PACKET_CLASS_COUNT i32 (i32.const 0x020BFCA0))
  (global $BSS_LAST_FLOOR_PACKET_VALUE_COUNT i32 (i32.const 0x020BFCA4))
  (global $BSS_LAST_FLOOR_PACKET_SCALAR_COUNT i32 (i32.const 0x020BFCA8))
  (global $BSS_LAST_FLOOR_PACKET_POINT_COUNT i32 (i32.const 0x020BFCAC))
  (global $BSS_LAST_FLOOR_PACKET_SORTED_COUNT i32 (i32.const 0x020BFCB0))
  (global $BSS_LAST_FLOOR_PACKET_SEGMENT_COUNT i32 (i32.const 0x020BFCB4))
  (global $BSS_LAST_RESIDUE_PACKET_STATUS i32 (i32.const 0x020BFCB8))
  (global $BSS_LAST_RESIDUE_PACKET_COUNT i32 (i32.const 0x020BFCBC))
  (global $BSS_LAST_RESIDUE_PACKET_PARTITION_COUNT i32 (i32.const 0x020BFCC0))
  (global $BSS_LAST_RESIDUE_PACKET_CLASS_COUNT i32 (i32.const 0x020BFCC4))
  (global $BSS_LAST_RESIDUE_PACKET_BOOK_COUNT i32 (i32.const 0x020BFCC8))
  (global $BSS_LAST_RESIDUE_PACKET_SCALAR_COUNT i32 (i32.const 0x020BFCCC))
  (global $BSS_LAST_RESIDUE_PACKET_VALUE_COUNT i32 (i32.const 0x020BFCD0))
  (global $BSS_LAST_RESIDUE_WORK_VECTOR_STATUS i32 (i32.const 0x020BFCD4))
  (global $BSS_LAST_RESIDUE_WORK_VECTOR_COUNT i32 (i32.const 0x020BFCD8))
  (global $BSS_LAST_RESIDUE_WORK_VECTOR_NONZERO i32 (i32.const 0x020BFCDC))
  (global $BSS_LAST_FLOOR_APPLY_STATUS i32 (i32.const 0x020BFCE0))
  (global $BSS_LAST_FLOOR_APPLY_COUNT i32 (i32.const 0x020BFCE4))
  (global $BSS_LAST_FLOOR_APPLY_NONZERO i32 (i32.const 0x020BFCE8))
  (global $BSS_LAST_FLOOR_RESIDUE_PCM_STATUS i32 (i32.const 0x020BFCEC))
  (global $BSS_LAST_FLOOR_RESIDUE_PCM_FRAMES i32 (i32.const 0x020BFCF0))
  (global $BSS_LAST_PCM_STATUS i32 (i32.const 0x020BFCF4))
  (global $BSS_LAST_SILENT_PACKET_COUNT i32 (i32.const 0x020BFCF8))
  (global $BSS_LAST_BAD_CODEBOOK_INDEX i32 (i32.const 0x020BFCFC))
  (global $BSS_LAST_BAD_CODEBOOK_ENTRIES i32 (i32.const 0x020BFD00))
  (global $BSS_LAST_BAD_CODEBOOK_MAX_LENGTH i32 (i32.const 0x020BFD04))
  (global $BSS_LAST_MDCT_SYNTH_STATUS i32 (i32.const 0x020BFD08))
  (global $BSS_LAST_MDCT_SYNTH_PACKETS i32 (i32.const 0x020BFD0C))
  (global $BSS_LAST_MDCT_SYNTH_FRAMES i32 (i32.const 0x020BFD10))
  (global $BSS_LAST_MDCT_SYNTH_NONZERO i32 (i32.const 0x020BFD14))
  (global $BSS_LAST_MDCT_SYNTH_OVERLAP_NONZERO i32 (i32.const 0x020BFD18))
  (global $BSS_LAST_IMDCT_KERNEL_BINS i32 (i32.const 0x020BFD1C))
  (global $BSS_LAST_IMDCT_KERNEL_SAMPLES i32 (i32.const 0x020BFD20))
  (global $BSS_LAST_IMDCT_KERNEL_CROSS_TERMS i32 (i32.const 0x020BFD24))
  (global $BSS_MDCT_INPUT_Q15 i32 (i32.const 0x020BFD40))
  (global $BSS_OVERLAP_Q15 i32 (i32.const 0x020C7D40))
  (global $BSS_COS_Q15 i32 (i32.const 0x020CBD40))
  (global $BSS_FLOOR_GAIN_Q15 i32 (i32.const 0x020CBF44))
  (global $SS_READY i32 (i32.const 0))
  (global $SS_BLOCK_SIZE_SHORT i32 (i32.const 4))
  (global $SS_BLOCK_SIZE_LONG i32 (i32.const 8))
  (global $SS_CODEBOOK_COUNT i32 (i32.const 12))
  (global $SS_CODEBOOK_STATUS i32 (i32.const 16))
  (global $SS_CODEBOOK_PARSED i32 (i32.const 20))
  (global $SS_CODEBOOK_ENTRIES i32 (i32.const 24))
  (global $SS_CODEBOOK_NONZERO i32 (i32.const 28))
  (global $SS_CODEBOOK_MAX_LEN i32 (i32.const 32))
  (global $SS_LOOKUP_VALUES i32 (i32.const 36))
  (global $SS_SETUP_BIT_BYTE i32 (i32.const 40))
  (global $SS_SETUP_BIT_OFFSET i32 (i32.const 44))
  (global $SS_TIME_COUNT i32 (i32.const 48))
  (global $SS_FLOOR_COUNT i32 (i32.const 52))
  (global $SS_RESIDUE_COUNT i32 (i32.const 56))
  (global $SS_MAPPING_COUNT i32 (i32.const 60))
  (global $SS_MODE_COUNT i32 (i32.const 64))
  (global $CB_DIMENSIONS i32 (i32.const 0))
  (global $CB_ENTRIES i32 (i32.const 4))
  (global $CB_LENGTH_OFFSET i32 (i32.const 8))
  (global $CB_CODE_OFFSET i32 (i32.const 12))
  (global $CB_NONZERO_COUNT i32 (i32.const 16))
  (global $CB_MAX_LENGTH i32 (i32.const 20))
  (global $CB_ORDERED i32 (i32.const 24))
  (global $CB_SPARSE i32 (i32.const 28))
  (global $CB_LOOKUP_TYPE i32 (i32.const 32))
  (global $CB_LOOKUP_MIN i32 (i32.const 36))
  (global $CB_LOOKUP_DELTA i32 (i32.const 40))
  (global $CB_LOOKUP_VALUE_BITS i32 (i32.const 44))
  (global $CB_LOOKUP_SEQUENCE i32 (i32.const 48))
  (global $CB_MULTIPLICAND_COUNT i32 (i32.const 52))
  (global $CB_MULTIPLICAND_OFFSET i32 (i32.const 56))
  (global $FH_PARTITION_COUNT i32 (i32.const 0))
  (global $FH_CLASS_COUNT i32 (i32.const 4))
  (global $FH_MULTIPLIER i32 (i32.const 8))
  (global $FH_RANGE_BITS i32 (i32.const 12))
  (global $FH_VALUE_COUNT i32 (i32.const 16))
  (global $FH_PART_OFFSET i32 (i32.const 20))
  (global $FH_CLASS_OFFSET i32 (i32.const 24))
  (global $FH_VALUE_OFFSET i32 (i32.const 28))
  (global $RH_RESIDUE_TYPE i32 (i32.const 0))
  (global $RH_BEGIN i32 (i32.const 4))
  (global $RH_END i32 (i32.const 8))
  (global $RH_PARTITION_SIZE i32 (i32.const 12))
  (global $RH_CLASSIFICATIONS i32 (i32.const 16))
  (global $RH_CLASSBOOK i32 (i32.const 20))
  (global $RH_BOOK_OFFSET i32 (i32.const 24))
  (global $MH_SUBMAPS i32 (i32.const 0))
  (global $MH_COUPLING_STEPS i32 (i32.const 4))
  (global $MH_MUX i32 (i32.const 8))
  (global $MH_FLOOR_OFFSET i32 (i32.const 12))
  (global $MH_RESIDUE_OFFSET i32 (i32.const 16))
  (global $MD_BLOCK_FLAG i32 (i32.const 0))
  (global $MD_MAPPING i32 (i32.const 4))
  (global $BR_DATA i32 (i32.const 0))
  (global $BR_LEN i32 (i32.const 8))
  (global $BR_BYTE_POS i32 (i32.const 16))
  (global $BR_BIT_POS i32 (i32.const 24))
  (global $BR_ERROR i32 (i32.const 28))
  (global $SS_SAMPLE_RATE i32 (i32.const 0))
  (global $SS_FRAME_COUNT i32 (i32.const 4))
  (global $SS_LOOP_START i32 (i32.const 8))
  (global $SS_LOOP_END i32 (i32.const 12))
  (global $SS_LOOP_FLAG i32 (i32.const 16))
  (global $SS_PACKET_COUNT i32 (i32.const 20))
  (global $SS_DECODE_STATUS i32 (i32.const 24))
  (global $SS_PCM_PTR i32 (i32.const 32))
  (global $SS_PACKET_PTRS i32 (i32.const 40))
  (global $SS_PACKET_LENS i32 (i32.const 8232))
  (global $RS_SAMPLE_RATE i32 (i32.const 0))
  (global $RS_FRAME_COUNT i32 (i32.const 4))
  (global $RS_LOOP_START i32 (i32.const 8))
  (global $RS_LOOP_END i32 (i32.const 12))
  (global $RS_LOOP_FLAG i32 (i32.const 16))
  (global $RS_PCM_PTR i32 (i32.const 24))
  (global (export "vorbis_codebook_headers") i32 (i32.const 0x001C3400))
  (global (export "vorbis_codebook_lengths") i32 (i32.const 0x001C4400))
  (global (export "vorbis_codebook_codes") i32 (i32.const 0x001CC400))
  (global (export "vorbis_codebook_multiplicands") i32 (i32.const 0x001D4400))
  (global (export "vorbis_floor_headers") i32 (i32.const 0x001D5000))
  (global (export "vorbis_floor_partitions") i32 (i32.const 0x001D5800))
  (global (export "vorbis_floor_class_dims") i32 (i32.const 0x001D5A00))
  (global (export "vorbis_floor_class_subbits") i32 (i32.const 0x001D5C00))
  (global (export "vorbis_floor_class_master") i32 (i32.const 0x001D5E00))
  (global (export "vorbis_floor_class_books") i32 (i32.const 0x001D6000))
  (global (export "vorbis_floor_values") i32 (i32.const 0x001D6800))
  (global (export "vorbis_residue_headers") i32 (i32.const 0x001D7000))
  (global (export "vorbis_residue_cascades") i32 (i32.const 0x001D7800))
  (global (export "vorbis_residue_books") i32 (i32.const 0x001D7A00))
  (global (export "vorbis_mapping_headers") i32 (i32.const 0x001D8200))
  (global (export "vorbis_mapping_floors") i32 (i32.const 0x001D8800))
  (global (export "vorbis_mapping_residues") i32 (i32.const 0x001D9800))
  (global (export "vorbis_mode_headers") i32 (i32.const 0x001DA800))
  (global (export "vorbis_packet_modes") i32 (i32.const 0x001DAC00))
  (global (export "vorbis_packet_mappings") i32 (i32.const 0x001DBC00))
  (global (export "vorbis_packet_floors") i32 (i32.const 0x001DCC00))
  (global (export "vorbis_packet_residues") i32 (i32.const 0x001DDC00))
  (global (export "vorbis_packet_floor_nonzero") i32 (i32.const 0x001DEC00))
  (global (export "vorbis_packet_floor_y0") i32 (i32.const 0x001DFC00))
  (global (export "vorbis_packet_floor_y1") i32 (i32.const 0x001E0C00))
  (global (export "vorbis_packet_floor_class_count") i32 (i32.const 0x001E1C00))
  (global (export "vorbis_packet_floor_class_ids") i32 (i32.const 0x001E2C00))
  (global (export "vorbis_packet_floor_class_selectors") i32 (i32.const 0x00222C00))
  (global (export "vorbis_packet_floor_class_dims") i32 (i32.const 0x00262C00))
  (global (export "vorbis_packet_floor_value_books") i32 (i32.const 0x002A2C00))
  (global (export "vorbis_packet_floor_values") i32 (i32.const 0x004A2C00))
  (global (export "vorbis_packet_floor_point_count") i32 (i32.const 0x006A2C00))
  (global (export "vorbis_packet_floor_point_x") i32 (i32.const 0x006A3C00))
  (global (export "vorbis_packet_floor_point_y") i32 (i32.const 0x008A3C00))
  (global (export "vorbis_packet_floor_point_order") i32 (i32.const 0x00AA3C00))
  (global (export "vorbis_packet_floor_segment_count") i32 (i32.const 0x00CA3C00))
  (global (export "vorbis_packet_floor_segment_x0") i32 (i32.const 0x00CA4C00))
  (global (export "vorbis_packet_floor_segment_y0") i32 (i32.const 0x00EA4C00))
  (global (export "vorbis_packet_floor_segment_x1") i32 (i32.const 0x010A4C00))
  (global (export "vorbis_packet_floor_segment_y1") i32 (i32.const 0x012A4C00))
  (global (export "vorbis_packet_residue_partition_count") i32 (i32.const 0x014A4C00))
  (global (export "vorbis_packet_residue_classifications") i32 (i32.const 0x014A5C00))
  (global (export "vorbis_packet_residue_value_count") i32 (i32.const 0x016A5C00))
  (global (export "vorbis_packet_residue_value_books") i32 (i32.const 0x016A6C00))
  (global (export "vorbis_packet_residue_value_entries") i32 (i32.const 0x018A6C00))
  (global (export "vorbis_packet_residue_value_dims") i32 (i32.const 0x01AA6C00))
  (global (export "vorbis_packet_residue_value_targets") i32 (i32.const 0x01CA6C00))
  (global (export "vorbis_packet_residue_values") i32 (i32.const 0x01EA6C00))
  (global (export "vorbis_packet_block_sizes") i32 (i32.const 0x020A6C00))
  (global (export "vorbis_packet_prev_window_flags") i32 (i32.const 0x020A7C00))
  (global (export "vorbis_packet_next_window_flags") i32 (i32.const 0x020A8C00))
  (global (export "vorbis_packet_window_left_start") i32 (i32.const 0x020A9C00))
  (global (export "vorbis_packet_window_left_end") i32 (i32.const 0x020AAC00))
  (global (export "vorbis_packet_window_right_start") i32 (i32.const 0x020ABC00))
  (global (export "vorbis_packet_window_right_end") i32 (i32.const 0x020ACC00))
  (global (export "vorbis_packet_window_left_frames") i32 (i32.const 0x020ADC00))
  (global (export "vorbis_packet_window_right_frames") i32 (i32.const 0x020AEC00))
  (global (export "vorbis_mdct_input_q15") i32 (i32.const 0x020BFD40))
  (global (export "vorbis_mdct_output_q15") i32 (i32.const 0x020AFC00))
  (global (export "vorbis_window_q15") i32 (i32.const 0x020B7C00))
  (global (export "vorbis_overlap_q15") i32 (i32.const 0x020C7D40))
  (global (export "vorbis_cos_q15_quarter") i32 (i32.const 0x020CBD40))
  (global (export "vorbis_floor_gain_q15") i32 (i32.const 0x020CBF44))

  (func $read_u32be (param $data i32) (param $off i32) (result i32)
    local.get $data local.get $off i32.add i32.load8_u i32.const 24 i32.shl
    local.get $data local.get $off i32.const 1 i32.add i32.add i32.load8_u i32.const 16 i32.shl
    i32.or
    local.get $data local.get $off i32.const 2 i32.add i32.add i32.load8_u i32.const 8 i32.shl
    i32.or
    local.get $data local.get $off i32.const 3 i32.add i32.add i32.load8_u
    i32.or
  )

  (func $ilog_u32 (param $val i32) (result i32)
    (local $n i32)
    i32.const 0 local.set $n
    (block $done
      (loop $loop
        local.get $val i32.eqz br_if $done
        local.get $val i32.const 1 i32.shr_u local.set $val
        local.get $n i32.const 1 i32.add local.set $n
        br $loop
      )
    )
    local.get $n
  )

  (func $pow_u32_capped (param $val i32) (param $pow i32) (result i32)
    (local $r i32)
    i32.const 1 local.set $r
    (block $done
      (loop $loop
        local.get $pow i32.eqz br_if $done
        local.get $r local.get $val i32.mul local.tee $r
        i32.const 0 i32.lt_s
        if
          i32.const 0x7fffffff return
        end
        local.get $pow i32.const 1 i32.sub local.set $pow
        br $loop
      )
    )
    local.get $r
  )

  (func $reverse_bits (param $val i32) (param $bits i32) (result i32)
    (local $r i32) (local $i i32)
    i32.const 0 local.set $r
    i32.const 0 local.set $i
    (block $done
      (loop $loop
        local.get $i local.get $bits i32.ge_u br_if $done
        local.get $r i32.const 1 i32.shl local.set $r
        local.get $r local.get $val i32.const 1 i32.and i32.or local.set $r
        local.get $val i32.const 1 i32.shr_u local.set $val
        local.get $i i32.const 1 i32.add local.set $i
        br $loop
      )
    )
    local.get $r
  )

  (func $unpack_float_q15 (param $val i32) (result i32)
    (local $mant i32) (local $exp i32)
    local.get $val i32.const 0x1fffff i32.and local.set $mant
    local.get $val i32.const 0x80000000 i32.and
    if
      i32.const 0 local.get $mant i32.sub local.set $mant
    end
    local.get $val i32.const 21 i32.shr_u i32.const 0x3ff i32.and i32.const 788 i32.sub local.set $exp
    local.get $exp i32.const 0 i32.ge_s
    if
      local.get $exp i32.const 15 i32.gt_s
      if
        local.get $mant i32.const 0 i32.lt_s
        if
          i32.const -32768 return
        else
          i32.const 32767 return
        end
      end
      local.get $mant local.get $exp i32.shl local.set $mant
    else
      i32.const 0 local.get $exp i32.sub local.set $exp
      local.get $exp i32.const 31 i32.ge_s
      if
        i32.const 0 return
      end
      local.get $mant local.get $exp i32.shr_s local.set $mant
    end
    local.get $mant i32.const 32767 i32.gt_s
    if
      i32.const 32767 local.set $mant
    end
    local.get $mant i32.const -32768 i32.lt_s
    if
      i32.const -32768 local.set $mant
    end
    local.get $mant
  )

  (func $floor1_predict_y (param $x0 i32) (param $y0 i32) (param $x1 i32) (param $y1 i32) (param $x i32) (result i32)
    (local $dx i32)
    local.get $x1 local.get $x0 i32.sub local.tee $dx
    i32.eqz
    if
      local.get $y0 return
    end
    local.get $x local.get $x0 i32.sub
    local.get $y1 local.get $y0 i32.sub i32.mul
    local.get $dx i32.div_s
    local.get $y0 i32.add
  )

  (func $cos_q15_from_phase (param $phase i32) (result i32)
    (local $idx i32) (local $frac i32) (local $sign i32) (local $v0 i32) (local $v1 i32)
    local.get $phase i32.const 0xffff i32.and local.set $phase
    i32.const 0 local.set $sign
    local.get $phase i32.const 32768 i32.ge_u
    if
      local.get $phase i32.const 32768 i32.sub local.set $phase
      i32.const 1 local.set $sign
    end
    local.get $phase i32.const 16384 i32.gt_u
    if
      i32.const 32768 local.get $phase i32.sub local.set $phase
    end
    local.get $phase i32.const 6 i32.shr_u local.set $idx
    local.get $phase i32.const 63 i32.and local.set $frac
    local.get $idx i32.const 256 i32.lt_u
    if
      global.get $BSS_COS_Q15 local.get $idx i32.const 1 i32.shl i32.add i32.load16_s local.set $v0
      global.get $BSS_COS_Q15 local.get $idx i32.const 1 i32.shl i32.add i32.const 2 i32.add i32.load16_s local.set $v1
      local.get $v1 local.get $v0 i32.sub
      local.get $frac i32.mul
      i32.const 6 i32.shr_s
      local.get $v0 i32.add
      local.set $phase
    else
      global.get $BSS_COS_Q15 i32.const 512 i32.add i32.load16_s local.set $phase
    end
    local.get $sign
    if
      i32.const 0 local.get $phase i32.sub local.set $phase
    end
    local.get $phase
  )

  (func $read_i32be_sample (param $data i32) (param $len i32) (param $cursor_ptr i32) (result i32)
    (local $cursor i32) (local $val i32)
    local.get $cursor_ptr i32.load offset=0 local.set $cursor
    local.get $cursor i32.const 4 i32.add local.get $len i32.gt_u
    if
      i32.const -1 return
    end
    local.get $data local.get $cursor i32.add i32.load8_u i32.const 24 i32.shl
    local.get $data local.get $cursor i32.const 1 i32.add i32.add i32.load8_u i32.const 16 i32.shl
    i32.or
    local.get $data local.get $cursor i32.const 2 i32.add i32.add i32.load8_u i32.const 8 i32.shl
    i32.or
    local.get $data local.get $cursor i32.const 3 i32.add i32.add i32.load8_u
    i32.or
    local.set $val
    local.get $cursor_ptr local.get $cursor i32.const 4 i32.add i32.store offset=0
    local.get $val
  )

  (func $vorbis_bitreader_init (export "vorbis_bitreader_init")
    (param $br i32) (param $data i32) (param $len i32)
    local.get $br global.get $BR_DATA i32.add local.get $data i64.extend_i32_u i64.store offset=0
    local.get $br global.get $BR_LEN i32.add local.get $len i64.extend_i32_u i64.store offset=0
    local.get $br global.get $BR_BYTE_POS i32.add i64.const 0 i64.store offset=0
    local.get $br global.get $BR_BIT_POS i32.add i32.const 0 i32.store offset=0
    local.get $br global.get $BR_ERROR i32.add i32.const 0 i32.store offset=0
  )

  (func $read_bits_internal (param $br i32) (param $n i32) (result i32)
    (local $val i32) (local $byte_pos i32) (local $bit_pos i32) (local $data i32) (local $len i32)
    local.get $br global.get $BR_BYTE_POS i32.add i64.load offset=0 i32.wrap_i64 local.set $byte_pos
    local.get $br global.get $BR_BIT_POS i32.add i32.load offset=0 local.set $bit_pos
    local.get $br global.get $BR_DATA i32.add i64.load offset=0 i32.wrap_i64 local.set $data
    local.get $br global.get $BR_LEN i32.add i64.load offset=0 i32.wrap_i64 local.set $len
    i32.const 0 local.set $val
    (block $done
      (loop $read
        local.get $n i32.eqz br_if $done
        local.get $byte_pos local.get $len i32.ge_u
        if
          local.get $br global.get $BR_ERROR i32.add i32.const -1 i32.store offset=0
          i32.const 0 return
        end
        local.get $data local.get $byte_pos i32.add i32.load8_u
        local.get $bit_pos i32.shr_u
        i32.const 1 i32.and
        local.get $n i32.const 1 i32.sub local.tee $n
        i32.shl
        local.get $val i32.or
        local.set $val
        local.get $bit_pos i32.const 1 i32.add local.tee $bit_pos
        i32.const 8 i32.ne
        br_if $read
        i32.const 0 local.set $bit_pos
        local.get $byte_pos i32.const 1 i32.add local.set $byte_pos
        br $read
      )
    )
    local.get $br global.get $BR_BYTE_POS i32.add local.get $byte_pos i64.extend_i32_u i64.store offset=0
    local.get $br global.get $BR_BIT_POS i32.add local.get $bit_pos i32.store offset=0
    local.get $val
  )

  (func $read_bit_internal (param $br i32) (result i32)
    local.get $br i32.const 1 call $read_bits_internal
  )

  (func (export "vorbis_bitreader_read_bits")
    (param $br i32) (param $n i32) (result i32)
    local.get $br local.get $n call $read_bits_internal
  )

  (func (export "vorbis_bitreader_read_bit")
    (param $br i32) (result i32)
    local.get $br i32.const 1 call $read_bits_internal
  )

  (func $vorbis_setup_init (export "vorbis_setup_init")
    (param $setup i32)
    (local $i i32)
    i32.const 0 local.set $i
    (block $done
      (loop $loop
        local.get $i i32.const 17 i32.ge_u br_if $done
        local.get $setup local.get $i i32.const 2 i32.shl i32.add i32.const 0 i32.store offset=0
        local.get $i i32.const 1 i32.add local.set $i
        br $loop
      )
    )
  )

  (func $vorbis_codebook_get_header (export "vorbis_codebook_get_header")
    (param $index i32) (result i32)
    (local $count i32)
    global.get $BSS_LAST_CODEBOOK_COUNT i32.load offset=0 local.set $count
    local.get $index local.get $count i32.ge_u
    if
      i32.const 0 return
    end
    global.get $BSS_CODEBOOK_HEADERS local.get $index global.get $SIZEOF_CODEBOOK_HEADER i32.mul i32.add
  )

  (func (export "vorbis_codebook_get_lengths")
    (param $index i32) (result i32)
    (local $hdr i32)
    local.get $index call $vorbis_codebook_get_header
    local.tee $hdr i32.eqz
    if
      i32.const 0 return
    end
    global.get $BSS_CODEBOOK_LENGTHS
    local.get $hdr global.get $CB_LENGTH_OFFSET i32.add i32.load offset=0
    i32.const 2 i32.shl i32.add
  )

  (func (export "vorbis_codebook_get_codes")
    (param $index i32) (result i32)
    (local $hdr i32)
    local.get $index call $vorbis_codebook_get_header
    local.tee $hdr i32.eqz
    if
      i32.const 0 return
    end
    global.get $BSS_CODEBOOK_CODES
    local.get $hdr global.get $CB_CODE_OFFSET i32.add i32.load offset=0
    i32.const 2 i32.shl i32.add
  )

  (func (export "vorbis_codebook_get_multiplicands")
    (param $index i32) (result i32)
    (local $hdr i32)
    local.get $index call $vorbis_codebook_get_header
    local.tee $hdr i32.eqz
    if
      i32.const 0 return
    end
    global.get $BSS_CODEBOOK_MULTIPLICANDS
    local.get $hdr global.get $CB_MULTIPLICAND_OFFSET i32.add i32.load offset=0
    i32.const 2 i32.shl i32.add
  )

  (func (export "vorbis_setup_get_codebook_tables")
    (result i32) (result i32) (result i32) (result i32) (result i32)
    global.get $BSS_CODEBOOK_HEADERS
    global.get $BSS_CODEBOOK_LENGTHS
    global.get $BSS_CODEBOOK_CODES
    global.get $BSS_CODEBOOK_MULTIPLICANDS
    global.get $BSS_LAST_CODEBOOK_COUNT i32.load offset=0
  )

  (func $vorbis_codebook_lookup1_values (export "vorbis_codebook_lookup1_values")
    (param $entries i32) (param $dimensions i32) (result i32)
    (local $val i32)
    i32.const 1 local.set $val
    (block $done
      (loop $loop
        local.get $val local.get $dimensions i32.gt_u br_if $done
        local.get $val local.get $entries i32.mul local.set $val
        br $loop
      )
    )
    local.get $val
  )

  (func $vorbis_codebook_build_canonical (export "vorbis_codebook_build_canonical")
    (param $book_index i32) (param $setup i32) (result i32)
    (local $hdr i32) (local $entry_count i32) (local $len_offset i32) (local $code_offset i32)
    (local $i i32) (local $length i32) (local $code i32) (local $ishift i32) (local $i4 i32)
    (local $carry_val i32) (local $j i32) (local $temp_val i32)
    local.get $book_index call $vorbis_codebook_get_header
    local.tee $hdr i32.eqz
    if
      global.get $VORBIS_ERR_BAD_INDEX return
    end
    i32.const 0 local.set $i
    (block $zero_done
      (loop $zero_loop
        local.get $i i32.const 33 i32.ge_u br_if $zero_done
        global.get $BSS_CANONICAL_TEMP local.get $i i32.const 2 i32.shl i32.add i32.const 0 i32.store offset=0
        local.get $i i32.const 1 i32.add local.set $i
        br $zero_loop
      )
    )
    local.get $hdr global.get $CB_ENTRIES i32.add i32.load offset=0 local.set $entry_count
    local.get $hdr global.get $CB_LENGTH_OFFSET i32.add i32.load offset=0 local.set $len_offset
    local.get $hdr global.get $CB_CODE_OFFSET i32.add i32.load offset=0 local.set $code_offset
    i32.const 0 local.set $i
    (block $done
      (loop $loop
        local.get $i local.get $entry_count i32.ge_u br_if $done
        global.get $BSS_CODEBOOK_LENGTHS local.get $len_offset local.get $i i32.add i32.const 2 i32.shl i32.add i32.load offset=0 local.set $length
        local.get $length i32.eqz
        if
          local.get $i i32.const 1 i32.add local.set $i
          br $loop
        end
        i32.const 32 local.get $length i32.sub local.set $ishift
        i32.const 1 local.get $ishift i32.shl local.set $i4
        global.get $BSS_CANONICAL_TEMP local.get $length i32.const 2 i32.shl i32.add i32.load offset=0 local.set $code
        global.get $BSS_CODEBOOK_CODES local.get $code_offset local.get $i i32.add i32.const 2 i32.shl i32.add local.get $code i32.store offset=0
        local.get $code local.get $i4 i32.and
        if
          global.get $BSS_CANONICAL_TEMP local.get $length i32.const 2 i32.shl i32.add i32.const -4 i32.add i32.load offset=0 local.set $carry_val
        else
          local.get $code local.get $i4 i32.or local.set $carry_val
          local.get $length i32.const 1 i32.sub local.set $j
          (block $carry_done
            (loop $carry_loop
              local.get $j i32.const 1 i32.lt_s br_if $carry_done
              global.get $BSS_CANONICAL_TEMP local.get $j i32.const 2 i32.shl i32.add i32.load offset=0 local.set $temp_val
              local.get $temp_val local.get $code i32.ne br_if $carry_done
              i32.const 32 local.get $j i32.sub local.set $ishift
              i32.const 1 local.get $ishift i32.shl local.set $i4
              local.get $temp_val local.get $i4 i32.and
              if
                global.get $BSS_CANONICAL_TEMP local.get $j i32.const 2 i32.shl i32.add i32.const -4 i32.add i32.load offset=0
                local.set $temp_val
                global.get $BSS_CANONICAL_TEMP local.get $j i32.const 2 i32.shl i32.add local.get $temp_val i32.store offset=0
                br $carry_done
              else
                global.get $BSS_CANONICAL_TEMP local.get $j i32.const 2 i32.shl i32.add
                local.get $temp_val local.get $i4 i32.or i32.store offset=0
              end
              local.get $j i32.const 1 i32.sub local.set $j
              br $carry_loop
            )
          )
        end
        global.get $BSS_CANONICAL_TEMP local.get $length i32.const 2 i32.shl i32.add local.get $carry_val i32.store offset=0
        local.get $length i32.const 1 i32.add local.set $j
        (block $prop_done
          (loop $prop_loop
            local.get $j i32.const 32 i32.gt_u br_if $prop_done
            global.get $BSS_CANONICAL_TEMP local.get $j i32.const 2 i32.shl i32.add i32.load offset=0
            local.get $code i32.ne br_if $prop_done
            global.get $BSS_CANONICAL_TEMP local.get $j i32.const 2 i32.shl i32.add local.get $carry_val i32.store offset=0
            local.get $j i32.const 1 i32.add local.set $j
            br $prop_loop
          )
        )
        local.get $i i32.const 1 i32.add local.set $i
        br $loop
      )
    )
    i32.const 0
  )

  (func $vorbis_codebook_decode_scalar (export "vorbis_codebook_decode_scalar")
    (param $br i32) (param $book_index i32) (param $setup i32) (result i32)
    (local $hdr i32) (local $entries i32) (local $max_len i32) (local $len_offset i32) (local $code_offset i32)
    (local $val i32) (local $len i32) (local $j i32) (local $candidate i32)
    global.get $BSS_LAST_BAD_CODEBOOK_INDEX local.get $book_index i32.store offset=0
    local.get $book_index call $vorbis_codebook_get_header
    local.tee $hdr i32.eqz
    if
      i32.const 0 return
    end
    local.get $hdr global.get $CB_ENTRIES i32.add i32.load offset=0 local.set $entries
    global.get $BSS_LAST_BAD_CODEBOOK_ENTRIES local.get $entries i32.store offset=0
    local.get $hdr global.get $CB_MAX_LENGTH i32.add i32.load offset=0 local.set $max_len
    global.get $BSS_LAST_BAD_CODEBOOK_MAX_LENGTH local.get $max_len i32.store offset=0
    local.get $max_len i32.eqz if i32.const 0 return end
    local.get $max_len i32.const 32 i32.gt_u if i32.const 0 return end
    local.get $hdr global.get $CB_LENGTH_OFFSET i32.add i32.load offset=0 local.set $len_offset
    local.get $hdr global.get $CB_CODE_OFFSET i32.add i32.load offset=0 local.set $code_offset
    i32.const 0 local.set $val
    i32.const 0 local.set $len
    (block $done
      (loop $read_loop
        local.get $len local.get $max_len i32.ge_u br_if $done
        local.get $br call $read_bit_internal
        local.get $len i32.shl
        local.get $val i32.or
        local.set $val
        local.get $len i32.const 1 i32.add local.set $len
        i32.const 0 local.set $j
        (block $entry_done
          (loop $entry_loop
            local.get $j local.get $entries i32.ge_u br_if $entry_done
            global.get $BSS_CODEBOOK_LENGTHS
            local.get $len_offset local.get $j i32.add i32.const 2 i32.shl i32.add
            i32.load offset=0
            local.get $len i32.ne
            if
              local.get $j i32.const 1 i32.add local.set $j
              br $entry_loop
            end
            global.get $BSS_CODEBOOK_CODES
            local.get $code_offset local.get $j i32.add i32.const 2 i32.shl i32.add
            i32.load offset=0
            local.set $candidate
            local.get $candidate
            i32.const 32 local.get $len i32.sub i32.shr_u
            local.get $len
            call $reverse_bits
            local.get $val i32.ne
            if
              local.get $j i32.const 1 i32.add local.set $j
              br $entry_loop
            end
            local.get $j return
          )
        )
        br $read_loop
      )
    )
    i32.const 0
  )

  ;; TODO: remaining functions

  (func (export "vorbis_last_codebook_status") (result i32)
    global.get $BSS_LAST_CODEBOOK_STATUS i32.load offset=0
  )

  (func (export "vorbis_last_codebook_count") (result i32)
    global.get $BSS_LAST_CODEBOOK_COUNT i32.load offset=0
  )

  (func (export "vorbis_last_codebook_entries") (result i32)
    global.get $BSS_LAST_CODEBOOK_ENTRIES i32.load offset=0
  )

  (func (export "vorbis_last_codebook_nonzero") (result i32)
    global.get $BSS_LAST_CODEBOOK_NONZERO i32.load offset=0
  )

  (func (export "vorbis_last_codebook_max_len") (result i32)
    global.get $BSS_LAST_CODEBOOK_MAX_LEN i32.load offset=0
  )

  (func (export "vorbis_last_lookup_values") (result i32)
    global.get $BSS_LAST_LOOKUP_VALUES i32.load offset=0
  )

  (func (export "vorbis_last_setup_byte") (result i32)
    global.get $BSS_LAST_SETUP_BYTE i32.load offset=0
  )

  (func (export "vorbis_last_setup_bit") (result i32)
    global.get $BSS_LAST_SETUP_BIT i32.load offset=0
  )

  (func (export "vorbis_last_floor_status") (result i32)
    global.get $BSS_LAST_FLOOR_STATUS i32.load offset=0
  )

  (func (export "vorbis_last_residue_status") (result i32)
    global.get $BSS_LAST_RESIDUE_STATUS i32.load offset=0
  )

  (func (export "vorbis_last_mapping_status") (result i32)
    global.get $BSS_LAST_MAPPING_STATUS i32.load offset=0
  )

  (func (export "vorbis_last_mode_status") (result i32)
    global.get $BSS_LAST_MODE_STATUS i32.load offset=0
  )

  (func (export "vorbis_last_floor_values") (result i32)
    global.get $BSS_LAST_FLOOR_VALUES i32.load offset=0
  )

  (func (export "vorbis_last_residue_books") (result i32)
    global.get $BSS_LAST_RESIDUE_BOOKS i32.load offset=0
  )

  (func (export "vorbis_last_packet_status") (result i32)
    global.get $BSS_LAST_PACKET_STATUS i32.load offset=0
  )

  (func (export "vorbis_last_packet_count") (result i32)
    global.get $BSS_LAST_PACKET_COUNT i32.load offset=0
  )

  (func (export "vorbis_last_packet_mode") (result i32)
    global.get $BSS_LAST_PACKET_MODE i32.load offset=0
  )

  (func (export "vorbis_last_packet_mapping") (result i32)
    global.get $BSS_LAST_PACKET_MAPPING i32.load offset=0
  )

  (func (export "vorbis_last_packet_floor") (result i32)
    global.get $BSS_LAST_PACKET_FLOOR i32.load offset=0
  )

  (func (export "vorbis_last_packet_residue") (result i32)
    global.get $BSS_LAST_PACKET_RESIDUE i32.load offset=0
  )

  (func (export "vorbis_last_mode_bits") (result i32)
    global.get $BSS_LAST_MODE_BITS i32.load offset=0
  )

  (func (export "vorbis_last_window_status") (result i32)
    global.get $BSS_LAST_WINDOW_STATUS i32.load offset=0
  )

  (func (export "vorbis_last_window_packet") (result i32)
    global.get $BSS_LAST_WINDOW_PACKET i32.load offset=0
  )

  (func (export "vorbis_last_window_block_size") (result i32)
    global.get $BSS_LAST_WINDOW_BLOCK_SIZE i32.load offset=0
  )

  (func (export "vorbis_last_window_prev_flag") (result i32)
    global.get $BSS_LAST_WINDOW_PREV_FLAG i32.load offset=0
  )

  (func (export "vorbis_last_window_next_flag") (result i32)
    global.get $BSS_LAST_WINDOW_NEXT_FLAG i32.load offset=0
  )

  (func (export "vorbis_last_window_left_start") (result i32)
    global.get $BSS_LAST_WINDOW_LEFT_START i32.load offset=0
  )

  (func (export "vorbis_last_window_left_end") (result i32)
    global.get $BSS_LAST_WINDOW_LEFT_END i32.load offset=0
  )

  (func (export "vorbis_last_window_right_start") (result i32)
    global.get $BSS_LAST_WINDOW_RIGHT_START i32.load offset=0
  )

  (func (export "vorbis_last_window_right_end") (result i32)
    global.get $BSS_LAST_WINDOW_RIGHT_END i32.load offset=0
  )

  (func (export "vorbis_last_window_left_frames") (result i32)
    global.get $BSS_LAST_WINDOW_LEFT_FRAMES i32.load offset=0
  )

  (func (export "vorbis_last_window_right_frames") (result i32)
    global.get $BSS_LAST_WINDOW_RIGHT_FRAMES i32.load offset=0
  )

  (func (export "vorbis_last_mdct_buffer_frames") (result i32)
    global.get $BSS_LAST_MDCT_BUFFER_FRAMES i32.load offset=0
  )

  (func (export "vorbis_last_overlap_frames") (result i32)
    global.get $BSS_LAST_OVERLAP_FRAMES i32.load offset=0
  )

  (func (export "vorbis_last_floor_packet_status") (result i32)
    global.get $BSS_LAST_FLOOR_PACKET_STATUS i32.load offset=0
  )

  (func (export "vorbis_last_floor_packet_count") (result i32)
    global.get $BSS_LAST_FLOOR_PACKET_COUNT i32.load offset=0
  )

  (func (export "vorbis_last_floor_packet_nonzero") (result i32)
    global.get $BSS_LAST_FLOOR_PACKET_NONZERO i32.load offset=0
  )

  (func (export "vorbis_last_floor_packet_y0") (result i32)
    global.get $BSS_LAST_FLOOR_PACKET_Y0 i32.load offset=0
  )

  (func (export "vorbis_last_floor_packet_y1") (result i32)
    global.get $BSS_LAST_FLOOR_PACKET_Y1 i32.load offset=0
  )

  (func (export "vorbis_last_floor_packet_y_range") (result i32)
    global.get $BSS_LAST_FLOOR_PACKET_Y_RANGE i32.load offset=0
  )

  (func (export "vorbis_last_floor_packet_class_count") (result i32)
    global.get $BSS_LAST_FLOOR_PACKET_CLASS_COUNT i32.load offset=0
  )

  (func (export "vorbis_last_floor_packet_value_count") (result i32)
    global.get $BSS_LAST_FLOOR_PACKET_VALUE_COUNT i32.load offset=0
  )

  (func (export "vorbis_last_floor_packet_scalar_count") (result i32)
    global.get $BSS_LAST_FLOOR_PACKET_SCALAR_COUNT i32.load offset=0
  )

  (func (export "vorbis_last_floor_packet_point_count") (result i32)
    global.get $BSS_LAST_FLOOR_PACKET_POINT_COUNT i32.load offset=0
  )

  (func (export "vorbis_last_floor_packet_sorted_count") (result i32)
    global.get $BSS_LAST_FLOOR_PACKET_SORTED_COUNT i32.load offset=0
  )

  (func (export "vorbis_last_floor_packet_segment_count") (result i32)
    global.get $BSS_LAST_FLOOR_PACKET_SEGMENT_COUNT i32.load offset=0
  )

  (func (export "vorbis_last_residue_packet_status") (result i32)
    global.get $BSS_LAST_RESIDUE_PACKET_STATUS i32.load offset=0
  )

  (func (export "vorbis_last_residue_packet_count") (result i32)
    global.get $BSS_LAST_RESIDUE_PACKET_COUNT i32.load offset=0
  )

  (func (export "vorbis_last_residue_packet_partition_count") (result i32)
    global.get $BSS_LAST_RESIDUE_PACKET_PARTITION_COUNT i32.load offset=0
  )

  (func (export "vorbis_last_residue_packet_class_count") (result i32)
    global.get $BSS_LAST_RESIDUE_PACKET_CLASS_COUNT i32.load offset=0
  )

  (func (export "vorbis_last_residue_packet_book_count") (result i32)
    global.get $BSS_LAST_RESIDUE_PACKET_BOOK_COUNT i32.load offset=0
  )

  (func (export "vorbis_last_residue_packet_scalar_count") (result i32)
    global.get $BSS_LAST_RESIDUE_PACKET_SCALAR_COUNT i32.load offset=0
  )

  (func (export "vorbis_last_residue_packet_value_count") (result i32)
    global.get $BSS_LAST_RESIDUE_PACKET_VALUE_COUNT i32.load offset=0
  )

  (func (export "vorbis_last_residue_work_vector_status") (result i32)
    global.get $BSS_LAST_RESIDUE_WORK_VECTOR_STATUS i32.load offset=0
  )

  (func (export "vorbis_last_residue_work_vector_count") (result i32)
    global.get $BSS_LAST_RESIDUE_WORK_VECTOR_COUNT i32.load offset=0
  )

  (func (export "vorbis_last_residue_work_vector_nonzero") (result i32)
    global.get $BSS_LAST_RESIDUE_WORK_VECTOR_NONZERO i32.load offset=0
  )

  (func (export "vorbis_last_floor_apply_status") (result i32)
    global.get $BSS_LAST_FLOOR_APPLY_STATUS i32.load offset=0
  )

  (func (export "vorbis_last_floor_apply_count") (result i32)
    global.get $BSS_LAST_FLOOR_APPLY_COUNT i32.load offset=0
  )

  (func (export "vorbis_last_floor_apply_nonzero") (result i32)
    global.get $BSS_LAST_FLOOR_APPLY_NONZERO i32.load offset=0
  )

  (func (export "vorbis_last_floor_residue_pcm_status") (result i32)
    global.get $BSS_LAST_FLOOR_RESIDUE_PCM_STATUS i32.load offset=0
  )

  (func (export "vorbis_last_floor_residue_pcm_frames") (result i32)
    global.get $BSS_LAST_FLOOR_RESIDUE_PCM_FRAMES i32.load offset=0
  )

  (func (export "vorbis_last_pcm_status") (result i32)
    global.get $BSS_LAST_PCM_STATUS i32.load offset=0
  )

  (func (export "vorbis_last_silent_packet_count") (result i32)
    global.get $BSS_LAST_SILENT_PACKET_COUNT i32.load offset=0
  )

  (func (export "vorbis_last_bad_codebook_index") (result i32)
    global.get $BSS_LAST_BAD_CODEBOOK_INDEX i32.load offset=0
  )

  (func (export "vorbis_last_bad_codebook_entries") (result i32)
    global.get $BSS_LAST_BAD_CODEBOOK_ENTRIES i32.load offset=0
  )

  (func (export "vorbis_last_bad_codebook_max_length") (result i32)
    global.get $BSS_LAST_BAD_CODEBOOK_MAX_LENGTH i32.load offset=0
  )

  (func (export "vorbis_last_mdct_synth_status") (result i32)
    global.get $BSS_LAST_MDCT_SYNTH_STATUS i32.load offset=0
  )

  (func (export "vorbis_last_mdct_synth_packets") (result i32)
    global.get $BSS_LAST_MDCT_SYNTH_PACKETS i32.load offset=0
  )

  (func (export "vorbis_last_mdct_synth_frames") (result i32)
    global.get $BSS_LAST_MDCT_SYNTH_FRAMES i32.load offset=0
  )

  (func (export "vorbis_last_mdct_synth_nonzero") (result i32)
    global.get $BSS_LAST_MDCT_SYNTH_NONZERO i32.load offset=0
  )

  (func (export "vorbis_last_mdct_synth_overlap_nonzero") (result i32)
    global.get $BSS_LAST_MDCT_SYNTH_OVERLAP_NONZERO i32.load offset=0
  )

  (func (export "vorbis_last_imdct_kernel_bins") (result i32)
    global.get $BSS_LAST_IMDCT_KERNEL_BINS i32.load offset=0
  )

  (func (export "vorbis_last_imdct_kernel_samples") (result i32)
    global.get $BSS_LAST_IMDCT_KERNEL_SAMPLES i32.load offset=0
  )

  (func (export "vorbis_last_imdct_kernel_cross_terms") (result i32)
    global.get $BSS_LAST_IMDCT_KERNEL_CROSS_TERMS i32.load offset=0
  )

  
  (func $vorbis_decode_codebooks (export "vorbis_decode_codebooks")
    (param $data i32) (param $len i32) (param $setup i32) (result i32)
    (local $br i32) (local $i i32) (local $hdr i32) (local $entries i32)
    (local $ordered i32) (local $sparse i32) (local $j i32) (local $clen i32)
    (local $entry_offset i32) (local $mult_offset i32) (local $nonzero i32)
    (local $max_len i32) (local $book_count i32) (local $lookup_type i32)
    (local $lookup_val_bits i32) (local $mult_count i32) (local $k i32)
    (local $cur_len i32) (local $run i32) (local $sync i32)
    global.get $BSS_BITREADER local.set $br
    local.get $br local.get $data local.get $len call $vorbis_bitreader_init
    global.get $BSS_LAST_CODEBOOK_STATUS i32.const 0 i32.store offset=0
    global.get $BSS_LAST_CODEBOOK_COUNT i32.const 0 i32.store offset=0
    global.get $BSS_LAST_CODEBOOK_ENTRIES i32.const 0 i32.store offset=0
    global.get $BSS_LAST_CODEBOOK_NONZERO i32.const 0 i32.store offset=0
    global.get $BSS_LAST_CODEBOOK_MAX_LEN i32.const 0 i32.store offset=0
    global.get $BSS_LAST_LOOKUP_VALUES i32.const 0 i32.store offset=0
    local.get $setup global.get $SS_CODEBOOK_COUNT i32.add i32.load offset=0 local.set $book_count
    local.get $book_count global.get $VORBIS_MAX_CODEBOOKS i32.gt_u
    if
      global.get $BSS_LAST_CODEBOOK_STATUS i32.const -6 i32.store offset=0
      global.get $VORBIS_ERR_TOO_MANY_BOOKS return
    end
    global.get $BSS_LAST_CODEBOOK_COUNT local.get $book_count i32.store offset=0
    i32.const 0 local.set $i
    i32.const 0 local.set $entry_offset
    i32.const 0 local.set $mult_offset
    i32.const 0 local.set $nonzero
    i32.const 0 local.set $max_len
    (block $book_done
      (loop $book_loop
        local.get $i local.get $book_count i32.ge_u br_if $book_done
        global.get $BSS_CODEBOOK_HEADERS local.get $i global.get $SIZEOF_CODEBOOK_HEADER i32.mul i32.add local.set $hdr
        i32.const 0 local.set $j
        (block $hdr_zero_done
          (loop $hdr_zero_loop
            local.get $j i32.const 15 i32.ge_u br_if $hdr_zero_done
            local.get $hdr local.get $j i32.const 2 i32.shl i32.add i32.const 0 i32.store offset=0
            local.get $j i32.const 1 i32.add local.set $j
            br $hdr_zero_loop
          )
        )
        local.get $br i32.const 24 call $read_bits_internal local.set $sync
        local.get $sync global.get $VORBIS_CODEBOOK_SYNC i32.ne
        if
          global.get $BSS_LAST_CODEBOOK_STATUS i32.const -8 i32.store offset=0
          global.get $VORBIS_ERR_BAD_CODEBOOK return
        end
        local.get $hdr global.get $CB_DIMENSIONS i32.add local.get $br i32.const 16 call $read_bits_internal i32.store offset=0
        local.get $br i32.const 24 call $read_bits_internal local.set $entries
        local.get $hdr global.get $CB_ENTRIES i32.add local.get $entries i32.store offset=0
        local.get $hdr global.get $CB_LENGTH_OFFSET i32.add local.get $entry_offset i32.store offset=0
        local.get $hdr global.get $CB_CODE_OFFSET i32.add local.get $entry_offset i32.store offset=0
        local.get $entries local.get $entry_offset i32.add local.set $j
        local.get $j global.get $VORBIS_MAX_CODEBOOK_ENTRIES i32.gt_u
        if
          global.get $BSS_LAST_CODEBOOK_STATUS i32.const -7 i32.store offset=0
          global.get $VORBIS_ERR_TOO_MANY_ENTRIES return
        end
        global.get $BSS_LAST_CODEBOOK_ENTRIES local.get $j i32.store offset=0
        local.get $br call $read_bit_internal local.set $ordered
        local.get $hdr global.get $CB_ORDERED i32.add local.get $ordered i32.store offset=0
        local.get $ordered i32.eqz
        if
          local.get $br call $read_bit_internal local.set $sparse
          local.get $hdr global.get $CB_SPARSE i32.add local.get $sparse i32.store offset=0
          i32.const 0 local.set $j
          (block $unordered_done
            (loop $unordered_loop
              local.get $j local.get $entries i32.ge_u br_if $unordered_done
              i32.const 0 local.set $clen
              local.get $hdr global.get $CB_SPARSE i32.add i32.load offset=0
              if
                local.get $br call $read_bit_internal i32.eqz
                if
                  global.get $BSS_CODEBOOK_LENGTHS local.get $entry_offset local.get $j i32.add i32.const 2 i32.shl i32.add i32.const 0 i32.store offset=0
                  local.get $j i32.const 1 i32.add local.set $j
                  br $unordered_loop
                end
              end
              local.get $br i32.const 5 call $read_bits_internal i32.const 1 i32.add local.set $clen
              global.get $BSS_CODEBOOK_LENGTHS local.get $entry_offset local.get $j i32.add i32.const 2 i32.shl i32.add local.get $clen i32.store offset=0
              local.get $clen i32.eqz if local.get $j i32.const 1 i32.add local.set $j br $unordered_loop end
              local.get $hdr global.get $CB_NONZERO_COUNT i32.add local.get $hdr global.get $CB_NONZERO_COUNT i32.add i32.load offset=0 i32.const 1 i32.add i32.store offset=0
              local.get $nonzero i32.const 1 i32.add local.set $nonzero
              local.get $clen local.get $hdr global.get $CB_MAX_LENGTH i32.add i32.load offset=0 i32.gt_u if local.get $hdr global.get $CB_MAX_LENGTH i32.add local.get $clen i32.store offset=0 end
              local.get $clen local.get $max_len i32.gt_u if local.get $clen local.set $max_len end
              local.get $j i32.const 1 i32.add local.set $j
              br $unordered_loop
            )
          )
        else
          local.get $br i32.const 5 call $read_bits_internal i32.const 1 i32.add local.set $cur_len
          i32.const 0 local.set $j
          (block $ordered_done
            (loop $ordered_loop
              local.get $j local.get $entries i32.ge_u br_if $ordered_done
              local.get $entries local.get $j i32.sub call $ilog_u32 local.set $run
              local.get $br local.get $run call $read_bits_internal local.set $run
              (block $ordered_run_done
                (loop $ordered_run_loop
                  local.get $run i32.eqz br_if $ordered_run_done
                  local.get $j local.get $entries i32.ge_u
                  if
                    global.get $BSS_LAST_CODEBOOK_STATUS i32.const -8 i32.store offset=0
                    global.get $VORBIS_ERR_BAD_CODEBOOK return
                  end
                  global.get $BSS_CODEBOOK_LENGTHS local.get $entry_offset local.get $j i32.add i32.const 2 i32.shl i32.add local.get $cur_len i32.store offset=0
                  local.get $hdr global.get $CB_NONZERO_COUNT i32.add local.get $hdr global.get $CB_NONZERO_COUNT i32.add i32.load offset=0 i32.const 1 i32.add i32.store offset=0
                  local.get $nonzero i32.const 1 i32.add local.set $nonzero
                  local.get $cur_len local.get $hdr global.get $CB_MAX_LENGTH i32.add i32.load offset=0 i32.gt_u if local.get $hdr global.get $CB_MAX_LENGTH i32.add local.get $cur_len i32.store offset=0 end
                  local.get $cur_len local.get $max_len i32.gt_u if local.get $cur_len local.set $max_len end
                  local.get $j i32.const 1 i32.add local.set $j
                  local.get $run i32.const 1 i32.sub local.set $run
                  br $ordered_run_loop
                )
              )
              local.get $cur_len i32.const 1 i32.add local.set $cur_len
              br $ordered_loop
            )
          )
        end
        local.get $i local.get $setup call $vorbis_codebook_build_canonical drop
        local.get $br i32.const 4 call $read_bits_internal local.set $lookup_type
        local.get $hdr global.get $CB_LOOKUP_TYPE i32.add local.get $lookup_type i32.store offset=0
        local.get $lookup_type i32.eqz
        if
          local.get $entry_offset local.get $entries i32.add local.set $entry_offset
          local.get $i i32.const 1 i32.add local.set $i
          br $book_loop
        end
        local.get $hdr global.get $CB_LOOKUP_MIN i32.add local.get $br i32.const 32 call $read_bits_internal i32.store offset=0
        local.get $hdr global.get $CB_LOOKUP_DELTA i32.add local.get $br i32.const 32 call $read_bits_internal i32.store offset=0
        local.get $br i32.const 4 call $read_bits_internal i32.const 1 i32.add local.set $lookup_val_bits
        local.get $hdr global.get $CB_LOOKUP_VALUE_BITS i32.add local.get $lookup_val_bits i32.store offset=0
        local.get $hdr global.get $CB_LOOKUP_SEQUENCE i32.add local.get $br call $read_bit_internal i32.store offset=0
        local.get $lookup_type i32.const 1 i32.eq
        if
          local.get $hdr global.get $CB_ENTRIES i32.add i32.load offset=0
          local.get $hdr global.get $CB_DIMENSIONS i32.add i32.load offset=0
          call $vorbis_codebook_lookup1_values
          local.set $mult_count
        else
          local.get $hdr global.get $CB_ENTRIES i32.add i32.load offset=0
          local.get $hdr global.get $CB_DIMENSIONS i32.add i32.load offset=0
          i32.mul
          local.set $mult_count
        end
        local.get $hdr global.get $CB_MULTIPLICAND_COUNT i32.add local.get $mult_count i32.store offset=0
        local.get $hdr global.get $CB_MULTIPLICAND_OFFSET i32.add local.get $mult_offset i32.store offset=0
        local.get $mult_count local.get $mult_offset i32.add local.set $k
        local.get $k global.get $VORBIS_MAX_MULTIPLICANDS i32.gt_u
        if
          global.get $BSS_LAST_CODEBOOK_STATUS i32.const -7 i32.store offset=0
          global.get $VORBIS_ERR_TOO_MANY_ENTRIES return
        end
        global.get $BSS_LAST_LOOKUP_VALUES local.get $k i32.store offset=0
        i32.const 0 local.set $k
        (block $mult_done
          (loop $mult_loop
            local.get $k local.get $mult_count i32.ge_u br_if $mult_done
            global.get $BSS_CODEBOOK_MULTIPLICANDS local.get $mult_offset local.get $k i32.add i32.const 2 i32.shl i32.add
            local.get $br local.get $lookup_val_bits call $read_bits_internal i32.store offset=0
            local.get $k i32.const 1 i32.add local.set $k
            br $mult_loop
          )
        )
        local.get $entry_offset local.get $entries i32.add local.set $entry_offset
        local.get $mult_offset local.get $mult_count i32.add local.set $mult_offset
        local.get $i i32.const 1 i32.add local.set $i
        br $book_loop
      )
    )
    global.get $BSS_LAST_CODEBOOK_ENTRIES local.get $entry_offset i32.store offset=0
    global.get $BSS_LAST_CODEBOOK_NONZERO local.get $nonzero i32.store offset=0
    global.get $BSS_LAST_CODEBOOK_MAX_LEN local.get $max_len i32.store offset=0
    global.get $BSS_LAST_LOOKUP_VALUES local.get $mult_offset i32.store offset=0
    local.get $setup global.get $SS_CODEBOOK_ENTRIES i32.add local.get $entry_offset i32.store offset=0
    local.get $setup global.get $SS_CODEBOOK_NONZERO i32.add local.get $nonzero i32.store offset=0
    local.get $setup global.get $SS_CODEBOOK_MAX_LEN i32.add local.get $max_len i32.store offset=0
    local.get $setup global.get $SS_LOOKUP_VALUES i32.add local.get $mult_offset i32.store offset=0
    local.get $setup global.get $SS_CODEBOOK_STATUS i32.add i32.const 0 i32.store offset=0
    i32.const 0
  )

  (func $vorbis_decode_floors (export "vorbis_decode_floors")
    (param $data i32) (param $len i32) (param $setup i32) (result i32)
    (local $br i32) (local $floor_count i32) (local $i i32) (local $fh i32)
    (local $val i32) (local $j i32) (local $k i32) (local $class_count i32)
    (local $max_class i32) (local $dim_val i32) (local $subbits i32)
    (local $master_val i32) (local $books_per i32) (local $b i32)
    (local $multiplier i32) (local $range_bits i32) (local $range_val i32)
    (local $part_idx i32) (local $dim_idx i32) (local $value_idx i32)
    local.get $setup global.get $SS_FLOOR_COUNT i32.add i32.load offset=0 local.set $floor_count
    local.get $floor_count global.get $VORBIS_MAX_SETUP_OBJS i32.gt_u
    if
      global.get $VORBIS_ERR_TOO_MANY_SETUP return
    end
    global.get $BSS_BITREADER local.set $br
    local.get $br local.get $data local.get $len call $vorbis_bitreader_init
    global.get $BSS_LAST_FLOOR_STATUS i32.const 0 i32.store offset=0
    global.get $BSS_LAST_FLOOR_VALUES i32.const 0 i32.store offset=0
    i32.const 0 local.set $i
    (block $floor_done
      (loop $floor_loop
        local.get $i local.get $floor_count i32.ge_u br_if $floor_done
        global.get $BSS_FLOOR_HEADERS local.get $i global.get $SIZEOF_FLOOR_HEADER i32.mul i32.add local.set $fh
        i32.const 0 local.set $j
        (block $fh_zero_done
          (loop $fh_zero_loop
            local.get $j i32.const 8 i32.ge_u br_if $fh_zero_done
            local.get $fh local.get $j i32.const 2 i32.shl i32.add i32.const 0 i32.store offset=0
            local.get $j i32.const 1 i32.add local.set $j
            br $fh_zero_loop
          )
        )
        local.get $br i32.const 16 call $read_bits_internal
        i32.const 1 i32.ne
        if
          global.get $BSS_LAST_FLOOR_STATUS i32.const -2 i32.store offset=0
          global.get $VORBIS_ERR_BAD_ARCHIVE return
        end
        local.get $br i32.const 5 call $read_bits_internal local.set $val
        local.get $val global.get $VORBIS_MAX_FLOOR_PARTS i32.gt_u
        if
          global.get $BSS_LAST_FLOOR_STATUS i32.const -10 i32.store offset=0
          global.get $VORBIS_ERR_TOO_MANY_SETUP return
        end
        local.get $fh global.get $FH_PARTITION_COUNT i32.add local.get $val i32.store offset=0
        i32.const 0 local.set $max_class
        i32.const 0 local.set $j
        (block $part_done
          (loop $part_loop
            local.get $j local.get $val i32.ge_u br_if $part_done
            local.get $br i32.const 4 call $read_bits_internal local.set $k
            global.get $BSS_FLOOR_PARTITIONS local.get $j i32.const 2 i32.shl i32.add local.get $k i32.store offset=0
            local.get $k local.get $max_class i32.ge_u if local.get $k i32.const 1 i32.add local.set $max_class end
            local.get $j i32.const 1 i32.add local.set $j
            br $part_loop
          )
        )
        local.get $max_class global.get $VORBIS_MAX_FLOOR_CLASSES i32.gt_u
        if
          global.get $BSS_LAST_FLOOR_STATUS i32.const -10 i32.store offset=0
          global.get $VORBIS_ERR_TOO_MANY_SETUP return
        end
        local.get $fh global.get $FH_CLASS_COUNT i32.add local.get $max_class i32.store offset=0
        i32.const 0 local.set $j
        (block $class_done
          (loop $class_loop
            local.get $j local.get $max_class i32.ge_u br_if $class_done
            local.get $br i32.const 3 call $read_bits_internal i32.const 1 i32.add local.set $dim_val
            global.get $BSS_FLOOR_CLASS_DIMS local.get $j i32.const 2 i32.shl i32.add local.get $dim_val i32.store offset=0
            local.get $br i32.const 2 call $read_bits_internal local.set $subbits
            global.get $BSS_FLOOR_CLASS_SUBBITS local.get $j i32.const 2 i32.shl i32.add local.get $subbits i32.store offset=0
            local.get $subbits i32.eqz
            if
              global.get $BSS_FLOOR_CLASS_MASTER local.get $j i32.const 2 i32.shl i32.add i32.const 0 i32.store offset=0
              i32.const 1 local.set $books_per
            else
              local.get $br i32.const 8 call $read_bits_internal local.set $master_val
              global.get $BSS_FLOOR_CLASS_MASTER local.get $j i32.const 2 i32.shl i32.add local.get $master_val i32.store offset=0
              i32.const 1 local.get $subbits i32.shl local.set $books_per
            end
            i32.const 0 local.set $b
            (block $book_done
              (loop $book_loop
                local.get $b local.get $books_per i32.ge_u br_if $book_done
                local.get $br i32.const 8 call $read_bits_internal i32.const 1 i32.sub local.set $k
                local.get $j i32.const 3 i32.shl local.get $b i32.or local.set $k
                local.get $k global.get $VORBIS_MAX_RESIDUE_BOOKS i32.ge_u
                if
                  global.get $BSS_LAST_FLOOR_STATUS i32.const -10 i32.store offset=0
                  global.get $VORBIS_ERR_TOO_MANY_SETUP return
                end
                global.get $BSS_FLOOR_CLASS_BOOKS local.get $k i32.const 2 i32.shl i32.add local.get $val i32.store offset=0
                local.get $b i32.const 1 i32.add local.set $b
                br $book_loop
              )
            )
            local.get $j i32.const 1 i32.add local.set $j
            br $class_loop
          )
        )
        local.get $br i32.const 2 call $read_bits_internal i32.const 1 i32.add local.set $multiplier
        local.get $fh global.get $FH_MULTIPLIER i32.add local.get $multiplier i32.store offset=0
        local.get $br i32.const 4 call $read_bits_internal local.set $range_bits
        local.get $fh global.get $FH_RANGE_BITS i32.add local.get $range_bits i32.store offset=0
        i32.const 1 local.get $range_bits i32.shl local.set $range_val
        global.get $BSS_FLOOR_VALUES i32.const 4 i32.add local.get $range_val i32.store offset=0
        i32.const 2 local.set $value_idx
        i32.const 0 local.set $j
        (block $fv_done
          (loop $fv_loop
            local.get $j local.get $fh global.get $FH_PARTITION_COUNT i32.add i32.load offset=0 i32.ge_u br_if $fv_done
            global.get $BSS_FLOOR_PARTITIONS local.get $j i32.const 2 i32.shl i32.add i32.load offset=0 local.set $part_idx
            global.get $BSS_FLOOR_CLASS_DIMS local.get $part_idx i32.const 2 i32.shl i32.add i32.load offset=0 local.set $dim_idx
            (block $dim_done
              (loop $dim_loop
                local.get $dim_idx i32.eqz br_if $dim_done
                local.get $value_idx global.get $VORBIS_MAX_FLOOR_VALUES i32.ge_u
                if
                  global.get $BSS_LAST_FLOOR_STATUS i32.const -10 i32.store offset=0
                  global.get $VORBIS_ERR_TOO_MANY_SETUP return
                end
                global.get $BSS_FLOOR_VALUES local.get $value_idx i32.const 2 i32.shl i32.add
                local.get $br local.get $range_bits call $read_bits_internal i32.store offset=0
                local.get $value_idx i32.const 1 i32.add local.set $value_idx
                local.get $dim_idx i32.const 1 i32.sub local.set $dim_idx
                br $dim_loop
              )
            )
            local.get $j i32.const 1 i32.add local.set $j
            br $fv_loop
          )
        )
        local.get $fh global.get $FH_VALUE_COUNT i32.add local.get $value_idx i32.store offset=0
        global.get $BSS_LAST_FLOOR_VALUES local.get $value_idx i32.store offset=0
        local.get $i i32.const 1 i32.add local.set $i
        br $floor_loop
      )
    )
    i32.const 0
  )



  (func $vorbis_decode_residues (export "vorbis_decode_residues")
    (param $data i32) (param $len i32) (param $setup i32) (result i32)
    (local $br i32) (local $residue_count i32) (local $i i32) (local $rh i32)
    (local $j i32) (local $k i32) (local $cascade i32) (local $has_extra i32)
    (local $val i32) (local $casc i32) (local $book_global i32)
    (local $classifications i32) (local $num_books i32)
    local.get $setup global.get $SS_RESIDUE_COUNT i32.add i32.load offset=0 local.set $residue_count
    local.get $residue_count global.get $VORBIS_MAX_SETUP_OBJS i32.gt_u
    if global.get $VORBIS_ERR_TOO_MANY_SETUP return end
    global.get $BSS_BITREADER local.set $br
    local.get $br local.get $data local.get $len call $vorbis_bitreader_init
    global.get $BSS_LAST_RESIDUE_STATUS i32.const 0 i32.store offset=0
    global.get $BSS_LAST_RESIDUE_BOOKS i32.const 0 i32.store offset=0
    i32.const 0 local.set $i
    i32.const 0 local.set $book_global
    (block $res_done
      (loop $res_loop
        local.get $i local.get $residue_count i32.ge_u br_if $res_done
        global.get $BSS_RESIDUE_HEADERS local.get $i global.get $SIZEOF_RESIDUE_HEADER i32.mul i32.add local.set $rh
        local.get $rh global.get $RH_RESIDUE_TYPE i32.add local.get $br i32.const 16 call $read_bits_internal i32.store offset=0
        local.get $rh global.get $RH_BEGIN i32.add local.get $br i32.const 24 call $read_bits_internal i32.store offset=0
        local.get $rh global.get $RH_END i32.add local.get $br i32.const 24 call $read_bits_internal i32.store offset=0
        local.get $rh global.get $RH_PARTITION_SIZE i32.add local.get $br i32.const 24 call $read_bits_internal i32.const 1 i32.add i32.store offset=0
        local.get $br i32.const 6 call $read_bits_internal i32.const 1 i32.add local.set $classifications
        local.get $classifications global.get $VORBIS_MAX_RESIDUE_CLASSES i32.gt_u
        if global.get $VORBIS_ERR_TOO_MANY_SETUP return end
        local.get $rh global.get $RH_CLASSIFICATIONS i32.add local.get $classifications i32.store offset=0
        local.get $rh global.get $RH_CLASSBOOK i32.add local.get $br i32.const 8 call $read_bits_internal i32.store offset=0
        local.get $rh global.get $RH_BOOK_OFFSET i32.add local.get $book_global i32.store offset=0
        i32.const 0 local.set $j
        (block $cascade_done
          (loop $cascade_loop
            local.get $j local.get $classifications i32.ge_u br_if $cascade_done
            local.get $br i32.const 3 call $read_bits_internal local.set $cascade
            local.get $br call $read_bit_internal local.set $has_extra
            local.get $has_extra
            if
              local.get $br i32.const 5 call $read_bits_internal i32.const 3 i32.shl local.get $cascade i32.or local.set $cascade
            end
            global.get $BSS_RESIDUE_CASCADES local.get $j i32.const 2 i32.shl i32.add local.get $cascade i32.store offset=0
            local.get $j i32.const 1 i32.add local.set $j
            br $cascade_loop
          )
        )
        local.get $classifications i32.const 3 i32.shl local.set $num_books
        i32.const 0 local.set $j
        (block $book_done
          (loop $book_loop
            local.get $j local.get $num_books i32.ge_u br_if $book_done
            local.get $j i32.const 3 i32.shr_u local.set $k
            global.get $BSS_RESIDUE_CASCADES local.get $k i32.const 2 i32.shl i32.add i32.load offset=0 local.set $casc
            i32.const 1 local.get $j i32.const 7 i32.and i32.shl local.set $k
            local.get $casc local.get $k i32.and
            if
              local.get $br i32.const 8 call $read_bits_internal local.set $val
            else
              i32.const -1 local.set $val
            end
            local.get $book_global global.get $VORBIS_MAX_RESIDUE_BOOKS i32.ge_u
            if global.get $VORBIS_ERR_TOO_MANY_SETUP return end
            global.get $BSS_RESIDUE_BOOKS local.get $book_global i32.const 2 i32.shl i32.add local.get $val i32.store offset=0
            local.get $book_global i32.const 1 i32.add local.set $book_global
            local.get $j i32.const 1 i32.add local.set $j
            br $book_loop
          )
        )
        global.get $BSS_LAST_RESIDUE_BOOKS local.get $book_global i32.store offset=0
        local.get $i i32.const 1 i32.add local.set $i
        br $res_loop
      )
    )
    i32.const 0
  )

  (func $vorbis_decode_mappings (export "vorbis_decode_mappings")
    (param $data i32) (param $len i32) (param $setup i32) (result i32)
    (local $br i32) (local $mapping_count i32) (local $i i32) (local $mh i32)
    (local $j i32) (local $val i32) (local $submaps i32) (local $mux i32)
    (local $floor_offset i32) (local $coupling i32)
    local.get $setup global.get $SS_MAPPING_COUNT i32.add i32.load offset=0 local.set $mapping_count
    local.get $mapping_count global.get $VORBIS_MAX_SETUP_OBJS i32.gt_u
    if global.get $VORBIS_ERR_TOO_MANY_SETUP return end
    global.get $BSS_BITREADER local.set $br
    local.get $br local.get $data local.get $len call $vorbis_bitreader_init
    global.get $BSS_LAST_MAPPING_STATUS i32.const 0 i32.store offset=0
    i32.const 0 local.set $i
    (block $map_done
      (loop $map_loop
        local.get $i local.get $mapping_count i32.ge_u br_if $map_done
        global.get $BSS_MAPPING_HEADERS local.get $i global.get $SIZEOF_MAPPING_HEADER i32.mul i32.add local.set $mh
        local.get $br i32.const 16 call $read_bits_internal drop
        local.get $br call $read_bit_internal
        if
          local.get $br i32.const 4 call $read_bits_internal i32.const 1 i32.add local.set $submaps
        else
          i32.const 1 local.set $submaps
        end
        local.get $submaps global.get $VORBIS_MAX_MAPPING_SUBMAPS i32.gt_u
        if global.get $VORBIS_ERR_TOO_MANY_SETUP return end
        local.get $mh global.get $MH_SUBMAPS i32.add local.get $submaps i32.store offset=0
        local.get $br call $read_bit_internal
        if
          local.get $br i32.const 8 call $read_bits_internal drop
        end
        local.get $br i32.const 2 call $read_bits_internal drop
        local.get $submaps i32.const 1 i32.le_u
        if
          i32.const 0 local.set $mux
        else
          local.get $br i32.const 4 call $read_bits_internal local.set $mux
        end
        local.get $mh global.get $MH_MUX i32.add local.get $mux i32.store offset=0
        local.get $i global.get $VORBIS_MAX_MAPPING_SUBMAPS i32.mul local.set $floor_offset
        local.get $mh global.get $MH_FLOOR_OFFSET i32.add local.get $floor_offset i32.store offset=0
        local.get $mh global.get $MH_RESIDUE_OFFSET i32.add local.get $floor_offset i32.store offset=0
        i32.const 0 local.set $j
        (block $submap_done
          (loop $submap_loop
            local.get $j local.get $submaps i32.ge_u br_if $submap_done
            local.get $br i32.const 8 call $read_bits_internal drop
            local.get $br i32.const 8 call $read_bits_internal local.set $val
            global.get $BSS_MAPPING_FLOORS local.get $floor_offset local.get $j i32.add i32.const 2 i32.shl i32.add local.get $val i32.store offset=0
            local.get $br i32.const 8 call $read_bits_internal local.set $val
            global.get $BSS_MAPPING_RESIDUES local.get $floor_offset local.get $j i32.add i32.const 2 i32.shl i32.add local.get $val i32.store offset=0
            local.get $j i32.const 1 i32.add local.set $j
            br $submap_loop
          )
        )
        local.get $i i32.const 1 i32.add local.set $i
        br $map_loop
      )
    )
    i32.const 0
  )

  (func $vorbis_decode_modes (export "vorbis_decode_modes")
    (param $data i32) (param $len i32) (param $setup i32) (result i32)
    (local $br i32) (local $mode_count i32) (local $i i32) (local $mh i32)
    local.get $setup global.get $SS_MODE_COUNT i32.add i32.load offset=0 local.set $mode_count
    local.get $mode_count global.get $VORBIS_MAX_SETUP_OBJS i32.gt_u
    if global.get $VORBIS_ERR_TOO_MANY_SETUP return end
    global.get $BSS_BITREADER local.set $br
    local.get $br local.get $data local.get $len call $vorbis_bitreader_init
    global.get $BSS_LAST_MODE_STATUS i32.const 0 i32.store offset=0
    i32.const 0 local.set $i
    (block $mode_done
      (loop $mode_loop
        local.get $i local.get $mode_count i32.ge_u br_if $mode_done
        global.get $BSS_MODE_HEADERS local.get $i global.get $SIZEOF_MODE_HEADER i32.mul i32.add local.set $mh
        local.get $mh global.get $MD_BLOCK_FLAG i32.add local.get $br call $read_bit_internal i32.store offset=0
        local.get $br i32.const 16 call $read_bits_internal drop
        local.get $br i32.const 16 call $read_bits_internal drop
        local.get $mh global.get $MD_MAPPING i32.add local.get $br i32.const 8 call $read_bits_internal i32.store offset=0
        local.get $i i32.const 1 i32.add local.set $i
        br $mode_loop
      )
    )
    i32.const 0
  )

  (func (export "vorbis_decode_setup_archive")
    (param $data i32) (param $len i32) (param $setup i32) (result i32)
    (local $br i32) (local $val i32) (local $count i32) (local $res i32)
    global.get $BSS_BITREADER local.set $br
    local.get $br local.get $data local.get $len call $vorbis_bitreader_init
    local.get $setup call $vorbis_setup_init
    local.get $br i32.const 4 call $read_bits_internal local.set $val
    local.get $setup global.get $SS_BLOCK_SIZE_SHORT i32.add i32.const 1 local.get $val i32.shl i32.store offset=0
    local.get $br i32.const 4 call $read_bits_internal local.set $val
    local.get $setup global.get $SS_BLOCK_SIZE_LONG i32.add i32.const 1 local.get $val i32.shl i32.store offset=0
    local.get $br i32.const 8 call $read_bits_internal i32.const 1 i32.add local.set $val
    local.get $setup global.get $SS_CODEBOOK_COUNT i32.add local.get $val i32.store offset=0
    local.get $data local.get $len local.get $setup call $vorbis_decode_codebooks
    local.tee $res
    if
      local.get $res return
    end
    local.get $br i32.const 6 call $read_bits_internal i32.const 1 i32.add local.set $val
    local.get $setup global.get $SS_TIME_COUNT i32.add local.get $val i32.store offset=0
    (block $time_done
      (loop $time_loop
        local.get $val i32.eqz br_if $time_done
        local.get $br i32.const 16 call $read_bits_internal drop
        local.get $val i32.const 1 i32.sub local.set $val
        br $time_loop
      )
    )
    local.get $br i32.const 6 call $read_bits_internal i32.const 1 i32.add local.set $val
    local.get $setup global.get $SS_FLOOR_COUNT i32.add local.get $val i32.store offset=0
    local.get $data local.get $len local.get $setup call $vorbis_decode_floors
    local.tee $res
    if
      local.get $res return
    end
    local.get $br i32.const 6 call $read_bits_internal i32.const 1 i32.add local.set $val
    local.get $setup global.get $SS_RESIDUE_COUNT i32.add local.get $val i32.store offset=0
    local.get $data local.get $len local.get $setup call $vorbis_decode_residues
    local.tee $res
    if
      local.get $res return
    end
    local.get $br i32.const 6 call $read_bits_internal i32.const 1 i32.add local.set $val
    local.get $setup global.get $SS_MAPPING_COUNT i32.add local.get $val i32.store offset=0
    local.get $data local.get $len local.get $setup call $vorbis_decode_mappings
    local.tee $res
    if
      local.get $res return
    end
    local.get $br i32.const 6 call $read_bits_internal i32.const 1 i32.add local.set $val
    local.get $setup global.get $SS_MODE_COUNT i32.add local.get $val i32.store offset=0
    local.get $data local.get $len local.get $setup call $vorbis_decode_modes
    local.tee $res
    if
      local.get $res return
    end
    local.get $br global.get $BR_BYTE_POS i32.add i64.load offset=0 i32.wrap_i64 local.set $val
    local.get $setup global.get $SS_SETUP_BIT_BYTE i32.add local.get $val i32.store offset=0
    global.get $BSS_LAST_SETUP_BYTE local.get $val i32.store offset=0
    local.get $br global.get $BR_BIT_POS i32.add i32.load offset=0 local.set $val
    local.get $setup global.get $SS_SETUP_BIT_OFFSET i32.add local.get $val i32.store offset=0
    global.get $BSS_LAST_SETUP_BIT local.get $val i32.store offset=0
    local.get $setup global.get $SS_READY i32.add i32.const 1 i32.store offset=0
    i32.const 0
  )

  (func $vorbis_decode_sample_packets (export "vorbis_decode_sample_packets")
    (param $sample i32) (param $setup i32) (result i32)
    (local $br i32) (local $mode_count i32) (local $mode_bits i32) (local $index i32)
    (local $packet_ptr i32) (local $packet_len i32) (local $mode_index i32)
    (local $mode_header i32) (local $prev_flag i32) (local $next_flag i32)
    (local $result i32) (local $mapping i32) (local $mapping_header i32)
    (local $mux i32) (local $floor i32) (local $floor_header i32)
    (local $residue i32) (local $residue_header i32) (local $error i32)
    (local $flags i32)

    local.get $setup
    if (result i32)
      local.get $setup
    else
      global.get $BSS_SETUP_STATE
    end
    local.set $setup

    global.get $BSS_LAST_PACKET_STATUS global.get $VORBIS_ERR_TODO_DECODE i32.store offset=0
    global.get $BSS_LAST_PACKET_COUNT i32.const 0 i32.store offset=0
    global.get $BSS_LAST_PACKET_MODE i32.const 0 i32.store offset=0
    global.get $BSS_LAST_PACKET_MAPPING i32.const 0 i32.store offset=0
    global.get $BSS_LAST_PACKET_FLOOR i32.const 0 i32.store offset=0
    global.get $BSS_LAST_PACKET_RESIDUE i32.const 0 i32.store offset=0
    global.get $BSS_LAST_MODE_BITS i32.const 0 i32.store offset=0
    global.get $BSS_LAST_WINDOW_STATUS global.get $VORBIS_ERR_TODO_DECODE i32.store offset=0
    global.get $BSS_LAST_WINDOW_PACKET i32.const 0 i32.store offset=0
    global.get $BSS_LAST_WINDOW_BLOCK_SIZE i32.const 0 i32.store offset=0
    global.get $BSS_LAST_WINDOW_PREV_FLAG i32.const 0 i32.store offset=0
    global.get $BSS_LAST_WINDOW_NEXT_FLAG i32.const 0 i32.store offset=0
    global.get $BSS_LAST_WINDOW_LEFT_START i32.const 0 i32.store offset=0
    global.get $BSS_LAST_WINDOW_LEFT_END i32.const 0 i32.store offset=0
    global.get $BSS_LAST_WINDOW_RIGHT_START i32.const 0 i32.store offset=0
    global.get $BSS_LAST_WINDOW_RIGHT_END i32.const 0 i32.store offset=0
    global.get $BSS_LAST_WINDOW_LEFT_FRAMES i32.const 0 i32.store offset=0
    global.get $BSS_LAST_WINDOW_RIGHT_FRAMES i32.const 0 i32.store offset=0
    global.get $BSS_LAST_MDCT_BUFFER_FRAMES i32.const 0 i32.store offset=0
    global.get $BSS_LAST_OVERLAP_FRAMES i32.const 0 i32.store offset=0
    global.get $BSS_LAST_FLOOR_PACKET_STATUS global.get $VORBIS_ERR_TODO_DECODE i32.store offset=0
    global.get $BSS_LAST_FLOOR_PACKET_COUNT i32.const 0 i32.store offset=0
    global.get $BSS_LAST_FLOOR_PACKET_NONZERO i32.const 0 i32.store offset=0
    global.get $BSS_LAST_FLOOR_PACKET_Y0 i32.const 0 i32.store offset=0
    global.get $BSS_LAST_FLOOR_PACKET_Y1 i32.const 0 i32.store offset=0
    global.get $BSS_LAST_RESIDUE_PACKET_STATUS global.get $VORBIS_ERR_TODO_DECODE i32.store offset=0
    global.get $BSS_LAST_RESIDUE_PACKET_COUNT i32.const 0 i32.store offset=0
    global.get $BSS_LAST_RESIDUE_PACKET_PARTITION_COUNT i32.const 0 i32.store offset=0
    global.get $BSS_LAST_RESIDUE_PACKET_CLASS_COUNT i32.const 0 i32.store offset=0
    global.get $BSS_LAST_RESIDUE_PACKET_BOOK_COUNT i32.const 0 i32.store offset=0
    global.get $BSS_LAST_RESIDUE_PACKET_SCALAR_COUNT i32.const 0 i32.store offset=0
    global.get $BSS_LAST_RESIDUE_PACKET_VALUE_COUNT i32.const 0 i32.store offset=0

    local.get $setup global.get $SS_READY i32.add i32.load offset=0
    i32.const 1 i32.ne
    if
      global.get $BSS_LAST_PACKET_STATUS global.get $VORBIS_ERR_BAD_ARCHIVE i32.store offset=0
      global.get $VORBIS_ERR_BAD_ARCHIVE return
    end

    local.get $setup global.get $SS_MODE_COUNT i32.add i32.load offset=0
    local.tee $mode_count
    i32.eqz
    if
      global.get $BSS_LAST_PACKET_STATUS global.get $VORBIS_ERR_BAD_ARCHIVE i32.store offset=0
      global.get $VORBIS_ERR_BAD_ARCHIVE return
    end

    local.get $mode_count i32.const 1 i32.sub
    call $ilog_u32
    local.tee $mode_bits
    global.get $BSS_LAST_MODE_BITS i32.store offset=0

    global.get $BSS_BITREADER local.set $br

    i32.const 0 local.set $index

    (block $packet_frontier
      (loop $packet_loop
        local.get $index
        local.get $sample global.get $SS_PACKET_COUNT i32.add i32.load offset=0
        i32.ge_u
        br_if $packet_frontier

        local.get $sample global.get $SS_PACKET_PTRS i32.add
        local.get $index i32.const 3 i32.shl i32.add
        i64.load offset=0 i32.wrap_i64
        local.tee $packet_ptr
        i32.eqz
        if
          global.get $BSS_LAST_PACKET_STATUS global.get $VORBIS_ERR_BAD_ARCHIVE i32.store offset=0
          global.get $VORBIS_ERR_BAD_ARCHIVE return
        end

        local.get $sample global.get $SS_PACKET_LENS i32.add
        local.get $index i32.const 2 i32.shl i32.add
        i32.load offset=0
        local.tee $packet_len
        i32.eqz
        if
          global.get $BSS_LAST_PACKET_STATUS global.get $VORBIS_ERR_BAD_ARCHIVE i32.store offset=0
          global.get $VORBIS_ERR_BAD_ARCHIVE return
        end

        local.get $br local.get $packet_ptr local.get $packet_len call $vorbis_bitreader_init

        local.get $br call $read_bit_internal drop
        local.get $br global.get $BR_ERROR i32.add i32.load offset=0
        local.tee $error
        if
          global.get $BSS_LAST_PACKET_STATUS local.get $error i32.store offset=0
          local.get $error return
        end

        local.get $br local.get $mode_bits call $read_bits_internal
        local.set $mode_index
        local.get $br global.get $BR_ERROR i32.add i32.load offset=0
        local.tee $error
        if
          global.get $BSS_LAST_PACKET_STATUS local.get $error i32.store offset=0
          local.get $error return
        end

        local.get $mode_index local.get $mode_count i32.ge_u
        if
          global.get $BSS_LAST_PACKET_STATUS global.get $VORBIS_ERR_BAD_INDEX i32.store offset=0
          global.get $VORBIS_ERR_BAD_INDEX return
        end

        global.get $BSS_PACKET_MODES local.get $index i32.const 2 i32.shl i32.add
        local.get $mode_index i32.store offset=0
        global.get $BSS_LAST_PACKET_MODE local.get $mode_index i32.store offset=0

        global.get $BSS_MODE_HEADERS
        local.get $mode_index global.get $SIZEOF_MODE_HEADER i32.mul i32.add
        local.set $mode_header

        i32.const 0 local.set $prev_flag
        i32.const 0 local.set $next_flag
        local.get $mode_header global.get $MD_BLOCK_FLAG i32.add i32.load offset=0
        if
          local.get $br i32.const 2 call $read_bits_internal
          local.set $flags
          local.get $br global.get $BR_ERROR i32.add i32.load offset=0
          local.tee $error
          if
            global.get $BSS_LAST_PACKET_STATUS local.get $error i32.store offset=0
            local.get $error return
          end
          local.get $flags i32.const 1 i32.and local.set $prev_flag
          local.get $flags i32.const 1 i32.shr_u i32.const 1 i32.and local.set $next_flag
        end

        local.get $setup
        local.get $mode_header
        local.get $index
        local.get $prev_flag
        local.get $next_flag
        call $vorbis_prepare_packet_window
        local.tee $result
        if
          global.get $BSS_LAST_PACKET_STATUS local.get $result i32.store offset=0
          local.get $result return
        end

        local.get $mode_header global.get $MD_MAPPING i32.add i32.load offset=0
        local.tee $mapping
        local.get $setup global.get $SS_MAPPING_COUNT i32.add i32.load offset=0
        i32.ge_u
        if
          global.get $BSS_LAST_PACKET_STATUS global.get $VORBIS_ERR_BAD_INDEX i32.store offset=0
          global.get $VORBIS_ERR_BAD_INDEX return
        end

        global.get $BSS_PACKET_MAPPINGS local.get $index i32.const 2 i32.shl i32.add
        local.get $mapping i32.store offset=0
        global.get $BSS_LAST_PACKET_MAPPING local.get $mapping i32.store offset=0

        global.get $BSS_MAPPING_HEADERS
        local.get $mapping global.get $SIZEOF_MAPPING_HEADER i32.mul i32.add
        local.set $mapping_header

        local.get $mapping_header global.get $MH_MUX i32.add i32.load offset=0
        local.tee $mux
        local.get $mapping_header global.get $MH_SUBMAPS i32.add i32.load offset=0
        i32.ge_u
        if
          global.get $BSS_LAST_PACKET_STATUS global.get $VORBIS_ERR_BAD_INDEX i32.store offset=0
          global.get $VORBIS_ERR_BAD_INDEX return
        end

        global.get $BSS_MAPPING_FLOORS
        local.get $mapping_header global.get $MH_FLOOR_OFFSET i32.add i32.load offset=0
        local.get $mux i32.add
        i32.const 2 i32.shl i32.add
        i32.load offset=0
        local.tee $floor
        local.get $setup global.get $SS_FLOOR_COUNT i32.add i32.load offset=0
        i32.ge_u
        if
          global.get $BSS_LAST_PACKET_STATUS global.get $VORBIS_ERR_BAD_INDEX i32.store offset=0
          global.get $VORBIS_ERR_BAD_INDEX return
        end

        global.get $BSS_PACKET_FLOORS local.get $index i32.const 2 i32.shl i32.add
        local.get $floor i32.store offset=0
        global.get $BSS_LAST_PACKET_FLOOR local.get $floor i32.store offset=0

        global.get $BSS_FLOOR_HEADERS
        local.get $floor global.get $SIZEOF_FLOOR_HEADER i32.mul i32.add
        local.set $floor_header

        local.get $br local.get $floor_header local.get $index
        call $vorbis_decode_packet_floor
        local.tee $result
        if
          global.get $BSS_LAST_PACKET_STATUS local.get $result i32.store offset=0
          local.get $result return
        end

        global.get $BSS_MAPPING_RESIDUES
        local.get $mapping_header global.get $MH_RESIDUE_OFFSET i32.add i32.load offset=0
        local.get $mux i32.add
        i32.const 2 i32.shl i32.add
        i32.load offset=0
        local.tee $residue
        local.get $setup global.get $SS_RESIDUE_COUNT i32.add i32.load offset=0
        i32.ge_u
        if
          global.get $BSS_LAST_PACKET_STATUS global.get $VORBIS_ERR_BAD_INDEX i32.store offset=0
          global.get $VORBIS_ERR_BAD_INDEX return
        end

        global.get $BSS_PACKET_RESIDUES local.get $index i32.const 2 i32.shl i32.add
        local.get $residue i32.store offset=0
        global.get $BSS_LAST_PACKET_RESIDUE local.get $residue i32.store offset=0

        global.get $BSS_RESIDUE_HEADERS
        local.get $residue global.get $SIZEOF_RESIDUE_HEADER i32.mul i32.add
        local.set $residue_header

        local.get $br local.get $residue_header local.get $index
        call $vorbis_decode_packet_residue
        local.tee $result
        if
          global.get $BSS_LAST_PACKET_STATUS local.get $result i32.store offset=0
          local.get $result return
        end

        local.get $index i32.const 1 i32.add
        local.tee $index
        global.get $BSS_LAST_PACKET_COUNT i32.store offset=0

        br $packet_loop
      )
    )

    global.get $BSS_LAST_PACKET_STATUS global.get $VORBIS_ERR_TODO_DECODE i32.store offset=0
    i32.const 0
  )

  (func (export "vorbis_decode_packet_floor")
    (param $br i32) (param $fh i32) (param $packet_index i32) (result i32)
    (local $nonzero i32) (local $y_bits i32) (local $y_range i32) (local $base i32) (local $part_count i32)
    (local $val_count i32) (local $i i32) (local $j i32) (local $k i32)
    (local $class_id i32) (local $dims i32) (local $subbits i32) (local $selector i32)
    (local $entry i32) (local $tmp i32) (local $multi i32)
    (local $x_a i32) (local $x_b i32)
    global.get $BSS_PACKET_FLOOR_NONZERO local.get $packet_index i32.const 2 i32.shl i32.add i32.const 0 i32.store offset=0
    global.get $BSS_PACKET_FLOOR_Y0 local.get $packet_index i32.const 2 i32.shl i32.add i32.const 0 i32.store offset=0
    global.get $BSS_PACKET_FLOOR_Y1 local.get $packet_index i32.const 2 i32.shl i32.add i32.const 0 i32.store offset=0
    global.get $BSS_PACKET_FLOOR_CLASS_COUNT local.get $packet_index i32.const 2 i32.shl i32.add i32.const 0 i32.store offset=0
    global.get $BSS_PACKET_FLOOR_POINT_COUNT local.get $packet_index i32.const 2 i32.shl i32.add i32.const 0 i32.store offset=0
    global.get $BSS_PACKET_FLOOR_SEGMENT_COUNT local.get $packet_index i32.const 2 i32.shl i32.add i32.const 0 i32.store offset=0
    global.get $BSS_LAST_FLOOR_PACKET_CLASS_COUNT i32.const 0 i32.store offset=0
    global.get $BSS_LAST_FLOOR_PACKET_VALUE_COUNT i32.const 0 i32.store offset=0
    global.get $BSS_LAST_FLOOR_PACKET_SCALAR_COUNT i32.const 0 i32.store offset=0
    global.get $BSS_LAST_FLOOR_PACKET_POINT_COUNT i32.const 0 i32.store offset=0
    global.get $BSS_LAST_FLOOR_PACKET_SORTED_COUNT i32.const 0 i32.store offset=0
    global.get $BSS_LAST_FLOOR_PACKET_SEGMENT_COUNT i32.const 0 i32.store offset=0
    global.get $BSS_LAST_FLOOR_PACKET_Y_RANGE i32.const 0 i32.store offset=0
    global.get $BSS_LAST_FLOOR_PACKET_COUNT
    global.get $BSS_LAST_FLOOR_PACKET_COUNT i32.load offset=0 i32.const 1 i32.add
    i32.store offset=0
    local.get $br call $read_bit_internal
    local.tee $nonzero
    global.get $BSS_PACKET_FLOOR_NONZERO local.get $packet_index i32.const 2 i32.shl i32.add i32.store offset=0
    local.get $nonzero global.get $BSS_LAST_FLOOR_PACKET_NONZERO i32.store offset=0
    local.get $nonzero i32.eqz if i32.const 0 return end
    local.get $fh global.get $FH_MULTIPLIER i32.add i32.load offset=0
    local.tee $tmp
    i32.const 1 i32.eq if
      i32.const 8 local.set $y_bits
      i32.const 256 local.set $y_range
    else
      local.get $tmp
      i32.const 2 i32.eq if
        i32.const 7 local.set $y_bits
        i32.const 128 local.set $y_range
      else
        local.get $tmp
        i32.const 3 i32.eq if
          i32.const 7 local.set $y_bits
          i32.const 86 local.set $y_range
        else
          local.get $tmp
          i32.const 4 i32.eq if
            i32.const 6 local.set $y_bits
            i32.const 64 local.set $y_range
          else
            global.get $VORBIS_ERR_BAD_ARCHIVE return
          end
        end
      end
    end
    global.get $BSS_LAST_FLOOR_PACKET_Y_RANGE local.get $y_range i32.store offset=0
    local.get $br local.get $y_bits call $read_bits_internal
    local.tee $tmp
    global.get $BSS_PACKET_FLOOR_Y0 local.get $packet_index i32.const 2 i32.shl i32.add i32.store offset=0
    local.get $tmp global.get $BSS_LAST_FLOOR_PACKET_Y0 i32.store offset=0
    local.get $br local.get $y_bits call $read_bits_internal
    local.tee $tmp
    global.get $BSS_PACKET_FLOOR_Y1 local.get $packet_index i32.const 2 i32.shl i32.add i32.store offset=0
    local.get $tmp global.get $BSS_LAST_FLOOR_PACKET_Y1 i32.store offset=0
    local.get $fh global.get $FH_PARTITION_COUNT i32.add i32.load offset=0 local.tee $part_count
    global.get $VORBIS_MAX_FLOOR_PARTS i32.gt_u if global.get $VORBIS_ERR_TOO_MANY_SETUP return end
    local.get $packet_index i32.const 6 i32.shl local.set $base
    i32.const 0 local.set $i
    i32.const 0 local.set $val_count
    (block $class_done
      (loop $class_loop
        local.get $i local.get $part_count i32.ge_u br_if $class_done
        local.get $base local.get $i i32.add local.tee $tmp
        global.get $VORBIS_MAX_PACKET_FLOOR_CLASSES i32.ge_u if global.get $VORBIS_ERR_TOO_MANY_SETUP return end
        global.get $BSS_FLOOR_PARTITIONS local.get $i i32.const 2 i32.shl i32.add i32.load offset=0 local.tee $class_id
        local.get $fh global.get $FH_CLASS_COUNT i32.add i32.load offset=0 i32.ge_u if global.get $VORBIS_ERR_BAD_ARCHIVE return end
        global.get $BSS_PACKET_FLOOR_CLASS_IDS local.get $tmp i32.const 2 i32.shl i32.add local.get $class_id i32.store offset=0
        global.get $BSS_FLOOR_CLASS_DIMS local.get $class_id i32.const 2 i32.shl i32.add i32.load offset=0 local.tee $dims
        i32.eqz if global.get $VORBIS_ERR_BAD_ARCHIVE return end
        global.get $BSS_PACKET_FLOOR_CLASS_DIMS local.get $tmp i32.const 2 i32.shl i32.add local.get $dims i32.store offset=0
        global.get $BSS_FLOOR_CLASS_SUBBITS local.get $class_id i32.const 2 i32.shl i32.add i32.load offset=0 local.tee $subbits
        i32.eqz if
          i32.const 0 local.set $selector
        else
          local.get $br
          global.get $BSS_FLOOR_CLASS_MASTER local.get $class_id i32.const 2 i32.shl i32.add i32.load offset=0
          i32.const 0
          call $vorbis_codebook_decode_scalar
          local.set $selector
        end
        global.get $BSS_PACKET_FLOOR_CLASS_SELECTORS local.get $tmp i32.const 2 i32.shl i32.add local.get $selector i32.store offset=0
        global.get $BSS_PACKET_FLOOR_CLASS_COUNT local.get $packet_index i32.const 2 i32.shl i32.add
        global.get $BSS_PACKET_FLOOR_CLASS_COUNT local.get $packet_index i32.const 2 i32.shl i32.add i32.load offset=0 i32.const 1 i32.add
        i32.store offset=0
        global.get $BSS_LAST_FLOOR_PACKET_CLASS_COUNT
        global.get $BSS_LAST_FLOOR_PACKET_CLASS_COUNT i32.load offset=0 i32.const 1 i32.add
        i32.store offset=0
        i32.const 0 local.set $j
        (block $val_done
          (loop $val_loop
            local.get $j local.get $dims i32.ge_u br_if $val_done
            global.get $BSS_LAST_FLOOR_PACKET_VALUE_COUNT i32.load offset=0
            global.get $VORBIS_MAX_FLOOR_VALUES i32.ge_u if global.get $VORBIS_ERR_TOO_MANY_SETUP return end
            local.get $packet_index i32.const 9 i32.shl
            global.get $BSS_LAST_FLOOR_PACKET_VALUE_COUNT i32.load offset=0 i32.add
            global.get $VORBIS_MAX_PACKET_FLOOR_VALUES i32.ge_u if global.get $VORBIS_ERR_TOO_MANY_SETUP return end
            local.get $subbits i32.eqz
            if
              i32.const 0 local.set $multi
            else
              i32.const 1 local.get $subbits i32.shl i32.const 1 i32.sub
              local.get $selector i32.and
              local.tee $multi
              local.get $subbits i32.shr_u local.set $selector
            end
            local.get $class_id i32.const 3 i32.shl local.get $multi i32.or
            local.tee $multi
            global.get $VORBIS_MAX_RESIDUE_BOOKS i32.ge_u if global.get $VORBIS_ERR_TOO_MANY_SETUP return end
            global.get $BSS_FLOOR_CLASS_BOOKS local.get $multi i32.const 2 i32.shl i32.add i32.load offset=0 local.set $multi
            global.get $BSS_PACKET_FLOOR_VALUE_BOOKS local.get $packet_index i32.const 9 i32.shl global.get $BSS_LAST_FLOOR_PACKET_VALUE_COUNT i32.load offset=0 i32.add i32.const 2 i32.shl i32.add local.get $multi i32.store offset=0
            local.get $multi i32.const -1 i32.eq if
              global.get $BSS_PACKET_FLOOR_VALUES local.get $packet_index i32.const 9 i32.shl global.get $BSS_LAST_FLOOR_PACKET_VALUE_COUNT i32.load offset=0 i32.add i32.const 2 i32.shl i32.add i32.const 0 i32.store offset=0
            else
              local.get $multi global.get $BSS_LAST_CODEBOOK_COUNT i32.load offset=0 i32.ge_u if global.get $VORBIS_ERR_BAD_INDEX return end
              local.get $br local.get $multi i32.const 0 call $vorbis_codebook_decode_scalar
              global.get $BSS_PACKET_FLOOR_VALUES local.get $packet_index i32.const 9 i32.shl global.get $BSS_LAST_FLOOR_PACKET_VALUE_COUNT i32.load offset=0 i32.add i32.const 2 i32.shl i32.add i32.store offset=0
              global.get $BSS_LAST_FLOOR_PACKET_SCALAR_COUNT
              global.get $BSS_LAST_FLOOR_PACKET_SCALAR_COUNT i32.load offset=0 i32.const 1 i32.add
              i32.store offset=0
            end
            global.get $BSS_LAST_FLOOR_PACKET_VALUE_COUNT
            global.get $BSS_LAST_FLOOR_PACKET_VALUE_COUNT i32.load offset=0 i32.const 1 i32.add
            i32.store offset=0
            local.get $j i32.const 1 i32.add local.set $j
            br $val_loop
          )
        )
        local.get $i i32.const 1 i32.add local.set $i
        br $class_loop
      )
    )
    ;; Prepare points
    local.get $fh global.get $FH_VALUE_COUNT i32.add i32.load offset=0 local.tee $val_count
    i32.const 2 i32.lt_u if global.get $VORBIS_ERR_BAD_ARCHIVE return end
    local.get $val_count global.get $VORBIS_MAX_FLOOR_VALUES i32.gt_u if global.get $VORBIS_ERR_TOO_MANY_SETUP return end
    global.get $BSS_LAST_FLOOR_PACKET_VALUE_COUNT i32.load offset=0 i32.const 2 i32.add
    local.get $val_count i32.ne if global.get $VORBIS_ERR_BAD_ARCHIVE return end
    local.get $packet_index i32.const 9 i32.shl local.set $base
    i32.const 0 local.set $i
    (block $pt_done
      (loop $pt_loop
        local.get $i local.get $val_count i32.ge_u br_if $pt_done
        local.get $base local.get $i i32.add local.tee $tmp
        global.get $VORBIS_MAX_PACKET_FLOOR_VALUES i32.ge_u if global.get $VORBIS_ERR_TOO_MANY_SETUP return end
        global.get $BSS_FLOOR_VALUES local.get $i i32.const 2 i32.shl i32.add i32.load offset=0
        global.get $BSS_PACKET_FLOOR_POINT_X local.get $tmp i32.const 2 i32.shl i32.add i32.store offset=0
        local.get $i i32.const 0 i32.eq
        if (result i32)
          global.get $BSS_PACKET_FLOOR_Y0 local.get $packet_index i32.const 2 i32.shl i32.add i32.load offset=0
        else
          local.get $i i32.const 1 i32.eq
          if (result i32)
            global.get $BSS_PACKET_FLOOR_Y1 local.get $packet_index i32.const 2 i32.shl i32.add i32.load offset=0
          else
            global.get $BSS_PACKET_FLOOR_VALUES local.get $base local.get $i i32.const 2 i32.sub i32.add i32.const 2 i32.shl i32.add i32.load offset=0
          end
        end
        global.get $BSS_PACKET_FLOOR_POINT_Y local.get $tmp i32.const 2 i32.shl i32.add i32.store offset=0
        global.get $BSS_PACKET_FLOOR_POINT_ORDER local.get $tmp i32.const 2 i32.shl i32.add local.get $i i32.store offset=0
        local.get $i i32.const 1 i32.add local.set $i
        br $pt_loop
      )
    )
    global.get $BSS_PACKET_FLOOR_POINT_COUNT local.get $packet_index i32.const 2 i32.shl i32.add local.get $val_count i32.store offset=0
    global.get $BSS_LAST_FLOOR_PACKET_POINT_COUNT local.get $val_count i32.store offset=0
    ;; Bubble sort points by x
    i32.const 0 local.set $i
    (block $sort_done
      (loop $sort_outer
        local.get $i local.get $val_count i32.ge_u br_if $sort_done
        local.get $i i32.const 1 i32.add local.set $j
        (block $sort_inner_done
          (loop $sort_inner
            local.get $j local.get $val_count i32.ge_u br_if $sort_inner_done
            global.get $BSS_PACKET_FLOOR_POINT_ORDER local.get $base local.get $i i32.add i32.const 2 i32.shl i32.add i32.load offset=0 local.set $tmp
            global.get $BSS_PACKET_FLOOR_POINT_ORDER local.get $base local.get $j i32.add i32.const 2 i32.shl i32.add i32.load offset=0 local.set $k
            global.get $BSS_PACKET_FLOOR_POINT_X local.get $base local.get $tmp i32.add i32.const 2 i32.shl i32.add i32.load offset=0 local.set $x_a
            global.get $BSS_PACKET_FLOOR_POINT_X local.get $base local.get $k i32.add i32.const 2 i32.shl i32.add i32.load offset=0 local.set $x_b
            local.get $x_b local.get $x_a i32.lt_u
            if
              global.get $BSS_PACKET_FLOOR_POINT_ORDER local.get $base local.get $i i32.add i32.const 2 i32.shl i32.add local.get $k i32.store offset=0
              global.get $BSS_PACKET_FLOOR_POINT_ORDER local.get $base local.get $j i32.add i32.const 2 i32.shl i32.add local.get $tmp i32.store offset=0
            end
            local.get $j i32.const 1 i32.add local.set $j
            br $sort_inner
          )
        )
        local.get $i i32.const 1 i32.add local.set $i
        br $sort_outer
      )
    )
    global.get $BSS_LAST_FLOOR_PACKET_SORTED_COUNT local.get $val_count i32.store offset=0
    local.get $val_count i32.const 2 i32.lt_u if i32.const 0 return end
    ;; Build segments
    i32.const 0 local.set $i
    (block $seg_done
      (loop $seg_loop
        local.get $val_count i32.const 1 i32.sub local.get $i i32.le_u br_if $seg_done
        global.get $BSS_PACKET_FLOOR_POINT_ORDER local.get $base local.get $i i32.add i32.const 2 i32.shl i32.add i32.load offset=0 local.set $tmp
        global.get $BSS_PACKET_FLOOR_POINT_ORDER local.get $base local.get $i i32.add i32.const 4 i32.add i32.load offset=0 local.set $k
        global.get $BSS_PACKET_FLOOR_SEGMENT_X0 local.get $base local.get $i i32.add i32.const 2 i32.shl i32.add
        global.get $BSS_PACKET_FLOOR_POINT_X local.get $base local.get $tmp i32.add i32.const 2 i32.shl i32.add i32.load offset=0
        i32.store offset=0
        global.get $BSS_PACKET_FLOOR_SEGMENT_Y0 local.get $base local.get $i i32.add i32.const 2 i32.shl i32.add
        global.get $BSS_PACKET_FLOOR_POINT_Y local.get $base local.get $tmp i32.add i32.const 2 i32.shl i32.add i32.load offset=0
        i32.store offset=0
        global.get $BSS_PACKET_FLOOR_POINT_X local.get $base local.get $k i32.add i32.const 2 i32.shl i32.add i32.load offset=0
        global.get $BSS_PACKET_FLOOR_SEGMENT_X1 local.get $base local.get $i i32.add i32.const 2 i32.shl i32.add i32.store offset=0
        global.get $BSS_PACKET_FLOOR_POINT_Y local.get $base local.get $k i32.add i32.const 2 i32.shl i32.add i32.load offset=0
        global.get $BSS_PACKET_FLOOR_SEGMENT_Y1 local.get $base local.get $i i32.add i32.const 2 i32.shl i32.add i32.store offset=0
        local.get $i i32.const 1 i32.add local.set $i
        br $seg_loop
      )
    )
    global.get $BSS_PACKET_FLOOR_SEGMENT_COUNT local.get $packet_index i32.const 2 i32.shl i32.add local.get $i i32.store offset=0
    global.get $BSS_LAST_FLOOR_PACKET_SEGMENT_COUNT local.get $i i32.store offset=0
    i32.const 0
  )

  (func (export "vorbis_decode_packet_residue")
    (param $br i32) (param $packet_index i32) (result i32)
    global.get $VORBIS_ERR_TODO_DECODE
  )

  (func (export "vorbis_prepare_packet_window")
    (param $setup i32) (param $mode i32) (param $packet_index i32) (param $prev_flag i32) (param $next_flag i32) (result i32)
    (local $block_size i32) (local $short_bs i32) (local $half i32) (local $lq i32) (local $sq i32)
    (local $left_start i32) (local $left_end i32) (local $right_start i32) (local $right_end i32)
    local.get $packet_index global.get $VORBIS_MAX_PACKETS i32.ge_u if global.get $VORBIS_ERR_BAD_INDEX return end
    global.get $BSS_PACKET_PREV_WINDOW_FLAGS local.get $packet_index i32.const 2 i32.shl i32.add i32.const 0 i32.store offset=0
    global.get $BSS_PACKET_NEXT_WINDOW_FLAGS local.get $packet_index i32.const 2 i32.shl i32.add i32.const 0 i32.store offset=0
    global.get $BSS_PACKET_WINDOW_LEFT_START local.get $packet_index i32.const 2 i32.shl i32.add i32.const 0 i32.store offset=0
    global.get $BSS_PACKET_WINDOW_LEFT_END local.get $packet_index i32.const 2 i32.shl i32.add i32.const 0 i32.store offset=0
    global.get $BSS_PACKET_WINDOW_RIGHT_START local.get $packet_index i32.const 2 i32.shl i32.add i32.const 0 i32.store offset=0
    global.get $BSS_PACKET_WINDOW_RIGHT_END local.get $packet_index i32.const 2 i32.shl i32.add i32.const 0 i32.store offset=0
    global.get $BSS_PACKET_WINDOW_LEFT_FRAMES local.get $packet_index i32.const 2 i32.shl i32.add i32.const 0 i32.store offset=0
    global.get $BSS_PACKET_WINDOW_RIGHT_FRAMES local.get $packet_index i32.const 2 i32.shl i32.add i32.const 0 i32.store offset=0
    local.get $mode global.get $MD_BLOCK_FLAG i32.add i32.load offset=0
    if
      ;; long block
      global.get $BSS_PACKET_PREV_WINDOW_FLAGS local.get $packet_index i32.const 2 i32.shl i32.add local.get $prev_flag i32.store offset=0
      global.get $BSS_PACKET_NEXT_WINDOW_FLAGS local.get $packet_index i32.const 2 i32.shl i32.add local.get $next_flag i32.store offset=0
      global.get $BSS_LAST_WINDOW_PREV_FLAG local.get $prev_flag i32.store offset=0
      global.get $BSS_LAST_WINDOW_NEXT_FLAG local.get $next_flag i32.store offset=0
      local.get $setup global.get $SS_BLOCK_SIZE_LONG i32.add i32.load offset=0 local.tee $block_size
      i32.eqz if global.get $VORBIS_ERR_BAD_ARCHIVE return end
      local.get $setup global.get $SS_BLOCK_SIZE_SHORT i32.add i32.load offset=0 local.tee $short_bs
      i32.eqz if global.get $VORBIS_ERR_BAD_ARCHIVE return end
      local.get $block_size global.get $VORBIS_MAX_BLOCK_SIZE i32.gt_u if global.get $VORBIS_ERR_TOO_MANY_SETUP return end
      local.get $short_bs local.get $block_size i32.gt_u if global.get $VORBIS_ERR_BAD_ARCHIVE return end
      global.get $BSS_PACKET_BLOCK_SIZES local.get $packet_index i32.const 2 i32.shl i32.add local.get $block_size i32.store offset=0
      global.get $BSS_LAST_WINDOW_BLOCK_SIZE local.get $block_size i32.store offset=0
      global.get $BSS_LAST_MDCT_BUFFER_FRAMES local.get $block_size i32.store offset=0
      local.get $block_size i32.const 1 i32.shr_u local.set $half
      local.get $block_size i32.const 2 i32.shr_u local.set $lq
      local.get $short_bs i32.const 2 i32.shr_u local.set $sq
      local.get $prev_flag
      if
        i32.const 0 local.set $left_start
        local.get $half local.set $left_end
      else
        local.get $lq local.get $sq i32.sub local.set $left_start
        local.get $lq local.get $sq i32.add local.set $left_end
      end
      global.get $BSS_PACKET_WINDOW_LEFT_START local.get $packet_index i32.const 2 i32.shl i32.add local.get $left_start i32.store offset=0
      global.get $BSS_PACKET_WINDOW_LEFT_END local.get $packet_index i32.const 2 i32.shl i32.add local.get $left_end i32.store offset=0
      global.get $BSS_LAST_WINDOW_LEFT_START local.get $left_start i32.store offset=0
      global.get $BSS_LAST_WINDOW_LEFT_END local.get $left_end i32.store offset=0
      local.get $left_end local.get $left_start i32.sub
      local.set $left_start
      global.get $BSS_PACKET_WINDOW_LEFT_FRAMES local.get $packet_index i32.const 2 i32.shl i32.add local.get $left_start i32.store offset=0
      global.get $BSS_LAST_WINDOW_LEFT_FRAMES local.get $left_start i32.store offset=0
      local.get $next_flag
      if
        local.get $half local.set $right_start
        local.get $block_size local.set $right_end
      else
        local.get $lq i32.const 3 i32.mul local.get $sq i32.sub local.set $right_start
        local.get $lq i32.const 3 i32.mul local.get $sq i32.add local.set $right_end
      end
      local.get $right_start global.get $BSS_LAST_WINDOW_LEFT_END i32.load offset=0 i32.lt_u if global.get $VORBIS_ERR_BAD_ARCHIVE return end
      local.get $right_end local.get $block_size i32.gt_u if global.get $VORBIS_ERR_BAD_ARCHIVE return end
      global.get $BSS_PACKET_WINDOW_RIGHT_START local.get $packet_index i32.const 2 i32.shl i32.add local.get $right_start i32.store offset=0
      global.get $BSS_PACKET_WINDOW_RIGHT_END local.get $packet_index i32.const 2 i32.shl i32.add local.get $right_end i32.store offset=0
      global.get $BSS_LAST_WINDOW_RIGHT_START local.get $right_start i32.store offset=0
      global.get $BSS_LAST_WINDOW_RIGHT_END local.get $right_end i32.store offset=0
      local.get $right_end local.get $right_start i32.sub
      global.get $BSS_PACKET_WINDOW_RIGHT_FRAMES local.get $packet_index i32.const 2 i32.shl i32.add local.tee $half i32.store offset=0
      global.get $BSS_LAST_WINDOW_RIGHT_FRAMES local.get $half i32.store offset=0
      global.get $BSS_LAST_OVERLAP_FRAMES local.get $block_size i32.const 1 i32.shr_u i32.store offset=0
    else
      ;; short block
      local.get $setup global.get $SS_BLOCK_SIZE_SHORT i32.add i32.load offset=0 local.tee $block_size
      i32.eqz if global.get $VORBIS_ERR_BAD_ARCHIVE return end
      local.get $block_size global.get $VORBIS_MAX_BLOCK_SIZE i32.gt_u if global.get $VORBIS_ERR_TOO_MANY_SETUP return end
      global.get $BSS_PACKET_BLOCK_SIZES local.get $packet_index i32.const 2 i32.shl i32.add local.get $block_size i32.store offset=0
      global.get $BSS_LAST_WINDOW_BLOCK_SIZE local.get $block_size i32.store offset=0
      global.get $BSS_LAST_MDCT_BUFFER_FRAMES local.get $block_size i32.store offset=0
      local.get $block_size i32.const 1 i32.shr_u local.tee $half
      global.get $BSS_PACKET_WINDOW_LEFT_END local.get $packet_index i32.const 2 i32.shl i32.add local.tee $lq i32.store offset=0
      global.get $BSS_PACKET_WINDOW_RIGHT_START local.get $packet_index i32.const 2 i32.shl i32.add local.get $half i32.store offset=0
      global.get $BSS_PACKET_WINDOW_LEFT_FRAMES local.get $packet_index i32.const 2 i32.shl i32.add local.get $half i32.store offset=0
      global.get $BSS_PACKET_WINDOW_RIGHT_FRAMES local.get $packet_index i32.const 2 i32.shl i32.add local.get $half i32.store offset=0
      global.get $BSS_LAST_WINDOW_LEFT_END local.get $half i32.store offset=0
      global.get $BSS_LAST_WINDOW_RIGHT_START local.get $half i32.store offset=0
      global.get $BSS_LAST_WINDOW_LEFT_FRAMES local.get $half i32.store offset=0
      global.get $BSS_LAST_WINDOW_RIGHT_FRAMES local.get $half i32.store offset=0
      global.get $BSS_LAST_OVERLAP_FRAMES local.get $half i32.store offset=0
      local.get $block_size i32.const 1 i32.shl
      global.get $BSS_PACKET_WINDOW_RIGHT_END local.get $packet_index i32.const 2 i32.shl i32.add i32.store offset=0
      global.get $BSS_LAST_WINDOW_RIGHT_END local.get $block_size i32.const 1 i32.shl i32.store offset=0
    end
    global.get $BSS_LAST_WINDOW_PACKET local.get $packet_index i32.store offset=0
    global.get $BSS_LAST_WINDOW_STATUS i32.const 0 i32.store offset=0
    i32.const 0
  )

  (func (export "vorbis_reconstruct_floor1_point_y")
    (param $packet_index i32) (param $point_index i32) (result i32)
    (local $base i32) (local $cur_x i32) (local $lo i32) (local $hi i32) (local $i i32)
    (local $nx i32) (local $x0 i32) (local $y0 i32) (local $x1 i32) (local $y1 i32) (local $pred i32)
    (local $val i32) (local $yrange i32) (local $lowroom i32) (local $highroom i32) (local $room2 i32)
    local.get $packet_index global.get $VORBIS_MAX_PACKETS i32.ge_u if global.get $VORBIS_ERR_BAD_INDEX return end
    local.get $point_index global.get $VORBIS_MAX_FLOOR_VALUES i32.ge_u if global.get $VORBIS_ERR_BAD_INDEX return end
    local.get $packet_index i32.const 9 i32.shl local.set $base
    global.get $BSS_PACKET_FLOOR_POINT_X local.get $base local.get $point_index i32.add i32.const 2 i32.shl i32.add i32.load offset=0 local.set $cur_x
    i32.const -1 local.set $lo
    i32.const -1 local.set $hi
    i32.const 0 local.set $i
    (block $neighbors_done
      (loop $neighbors_loop
        local.get $i local.get $point_index i32.ge_u br_if $neighbors_done
        (block $next_n
          global.get $BSS_PACKET_FLOOR_POINT_X local.get $base local.get $i i32.add i32.const 2 i32.shl i32.add i32.load offset=0 local.set $nx
          local.get $nx local.get $cur_x i32.ge_u
          if
            local.get $nx local.get $cur_x i32.eq br_if $next_n
            local.get $hi i32.const -1 i32.ne
            if
              global.get $BSS_PACKET_FLOOR_POINT_X local.get $base local.get $hi i32.add i32.const 2 i32.shl i32.add i32.load offset=0
              local.get $nx i32.le_u
              br_if $next_n
            end
            local.get $i local.set $hi
          else
            local.get $lo i32.const -1 i32.ne
            if
              global.get $BSS_PACKET_FLOOR_POINT_X local.get $base local.get $lo i32.add i32.const 2 i32.shl i32.add i32.load offset=0
              local.get $nx i32.ge_u
              br_if $next_n
            end
            local.get $i local.set $lo
          end
        )
        local.get $i i32.const 1 i32.add local.set $i
        br $neighbors_loop
      )
    )
    local.get $lo i32.const -1 i32.eq if global.get $VORBIS_ERR_BAD_INDEX return end
    local.get $hi i32.const -1 i32.eq if global.get $VORBIS_ERR_BAD_INDEX return end
    global.get $BSS_PACKET_FLOOR_POINT_X local.get $base local.get $lo i32.add i32.const 2 i32.shl i32.add i32.load offset=0 local.set $x0
    global.get $BSS_PACKET_FLOOR_POINT_Y local.get $base local.get $lo i32.add i32.const 2 i32.shl i32.add i32.load offset=0 local.set $y0
    global.get $BSS_PACKET_FLOOR_POINT_X local.get $base local.get $hi i32.add i32.const 2 i32.shl i32.add i32.load offset=0 local.set $x1
    global.get $BSS_PACKET_FLOOR_POINT_Y local.get $base local.get $hi i32.add i32.const 2 i32.shl i32.add i32.load offset=0 local.set $y1
    local.get $x0 local.get $y0 local.get $x1 local.get $y1 local.get $cur_x call $floor1_predict_y local.set $pred
    global.get $BSS_PACKET_FLOOR_VALUES local.get $point_index i32.const 2 i32.sub local.get $base i32.add i32.const 2 i32.shl i32.add i32.load offset=0 local.set $val
    local.get $val i32.eqz
    if
      local.get $pred return
    end
    global.get $BSS_LAST_FLOOR_PACKET_Y_RANGE i32.load offset=0 local.tee $yrange
    i32.eqz if global.get $VORBIS_ERR_BAD_ARCHIVE return end
    local.get $pred local.set $lowroom
    local.get $yrange local.get $pred i32.sub local.set $highroom
    local.get $lowroom local.get $highroom i32.le_u
    if (result i32)
      local.get $lowroom
    else
      local.get $highroom
    end
    i32.const 1 i32.shl local.set $room2
    local.get $val local.get $room2 i32.lt_u
    if (result i32)
      local.get $val i32.const 1 i32.add i32.const 1 i32.shr_u
      local.tee $room2
      drop
      local.get $val i32.const 1 i32.and
      if (result i32)
        local.get $pred local.get $room2 i32.sub
      else
        local.get $pred local.get $room2 i32.add
      end
    else
      local.get $highroom local.get $lowroom i32.gt_u
      if (result i32)
        local.get $val
      else
        local.get $yrange local.get $val i32.sub i32.const 1 i32.sub
      end
    end
  )

  (func (export "vorbis_build_residue_work_vector_q15")
    (param $packet_index i32) (result i32)
    (local $value_count i32) (local $base i32) (local $i i32) (local $idx i32) (local $val i32) (local $target i32) (local $count i32)
    local.get $packet_index global.get $VORBIS_MAX_PACKETS i32.ge_u if global.get $VORBIS_ERR_BAD_INDEX return end
    ;; Clear mdct_input_q15 to 0
    i32.const 0 local.set $i
    global.get $BSS_MDCT_INPUT_Q15 local.set $idx
    (block $clear_done
      (loop $clear_loop
        local.get $i global.get $VORBIS_MAX_BLOCK_SIZE i32.ge_u br_if $clear_done
        local.get $idx local.get $i i32.const 2 i32.shl i32.add i32.const 0 i32.store offset=0
        local.get $i i32.const 1 i32.add local.set $i
        br $clear_loop
      )
    )
    global.get $BSS_LAST_RESIDUE_WORK_VECTOR_COUNT i32.const 0 i32.store offset=0
    global.get $BSS_LAST_RESIDUE_WORK_VECTOR_NONZERO i32.const 0 i32.store offset=0
    global.get $BSS_PACKET_RESIDUE_VALUE_COUNT local.get $packet_index i32.const 2 i32.shl i32.add i32.load offset=0 local.tee $value_count
    global.get $VORBIS_PACKET_RESIDUE_VALUES_PER_PACKET i32.gt_u if global.get $VORBIS_ERR_TOO_MANY_SETUP return end
    local.get $packet_index global.get $VORBIS_PACKET_RESIDUE_VALUES_PER_PACKET i32.mul local.set $base
    global.get $BSS_LAST_RESIDUE_WORK_VECTOR_COUNT i32.const 0 i32.store offset=0
    i32.const 0 local.set $i
    (block $done
      (loop $loop
        local.get $i local.get $value_count i32.ge_u br_if $done
        local.get $base local.get $i i32.add local.tee $idx
        global.get $VORBIS_MAX_PACKET_RESIDUE_VALUES i32.ge_u if global.get $VORBIS_ERR_TOO_MANY_SETUP return end
        global.get $BSS_PACKET_RESIDUE_VALUE_TARGETS local.get $idx i32.const 2 i32.shl i32.add i32.load offset=0 local.tee $target
        global.get $VORBIS_MAX_BLOCK_SIZE i32.ge_u if global.get $VORBIS_ERR_TOO_MANY_SETUP return end
        global.get $BSS_PACKET_RESIDUE_VALUE_VALUES local.get $idx i32.const 2 i32.shl i32.add i32.load offset=0
        local.tee $val
        i32.const 32767 i32.gt_s if i32.const 32767 local.set $val end
        local.get $val i32.const -32768 i32.lt_s if i32.const -32768 local.set $val end
        global.get $BSS_MDCT_INPUT_Q15 local.get $target i32.const 2 i32.shl i32.add i32.load offset=0
        local.get $val i32.add
        local.tee $val
        i32.const 32767 i32.gt_s if i32.const 32767 local.set $val end
        local.get $val i32.const -32768 i32.lt_s if i32.const -32768 local.set $val end
        global.get $BSS_MDCT_INPUT_Q15 local.get $target i32.const 2 i32.shl i32.add local.get $val i32.store offset=0
        local.get $target i32.const 1 i32.add local.tee $target
        global.get $BSS_LAST_RESIDUE_WORK_VECTOR_COUNT i32.load offset=0 i32.gt_u
        if
          global.get $BSS_LAST_RESIDUE_WORK_VECTOR_COUNT local.get $target i32.store offset=0
        end
        local.get $val i32.eqz if
        else
          global.get $BSS_LAST_RESIDUE_WORK_VECTOR_NONZERO
          global.get $BSS_LAST_RESIDUE_WORK_VECTOR_NONZERO i32.load offset=0 i32.const 1 i32.add
          i32.store offset=0
        end
        local.get $i i32.const 1 i32.add local.set $i
        br $loop
      )
    )
    global.get $BSS_LAST_RESIDUE_WORK_VECTOR_STATUS i32.const 0 i32.store offset=0
    i32.const 0
  )

  (func (export "vorbis_apply_floor_curve_to_residue_q15")
    (param $packet_index i32) (result i32)
    (local $work_count i32) (local $seg_count i32) (local $i i32) (local $j i32) (local $base i32)
    (local $x0 i32) (local $x1 i32) (local $y0 i32) (local $y1 i32) (local $dx i32) (local $gain i32) (local $g i32) (local $val i32)
    local.get $packet_index global.get $VORBIS_MAX_PACKETS i32.ge_u if global.get $VORBIS_ERR_BAD_INDEX return end
    global.get $BSS_PACKET_FLOOR_NONZERO local.get $packet_index i32.const 2 i32.shl i32.add i32.load offset=0
    i32.eqz if i32.const 0 return end
    global.get $BSS_LAST_RESIDUE_WORK_VECTOR_COUNT i32.load offset=0 local.tee $work_count
    global.get $VORBIS_MAX_BLOCK_SIZE i32.gt_u if global.get $VORBIS_ERR_TOO_MANY_SETUP return end
    global.get $BSS_PACKET_FLOOR_SEGMENT_COUNT local.get $packet_index i32.const 2 i32.shl i32.add i32.load offset=0 local.tee $seg_count
    i32.eqz if global.get $VORBIS_ERR_BAD_ARCHIVE return end
    local.get $seg_count global.get $VORBIS_MAX_FLOOR_VALUES i32.gt_u if global.get $VORBIS_ERR_TOO_MANY_SETUP return end
    global.get $BSS_LAST_FLOOR_APPLY_COUNT i32.const 0 i32.store offset=0
    global.get $BSS_LAST_FLOOR_APPLY_NONZERO i32.const 0 i32.store offset=0
    local.get $packet_index i32.const 9 i32.shl local.set $base
    i32.const 0 local.set $i
    (block $samp_done
      (loop $samp_loop
        local.get $i local.get $work_count i32.ge_u br_if $samp_done
        i32.const 128 local.set $gain
        i32.const 0 local.set $j
        (block $seg_found
          (loop $seg_loop
            local.get $j local.get $seg_count i32.ge_u br_if $seg_found
            (block $next_seg
              global.get $BSS_PACKET_FLOOR_SEGMENT_X0 local.get $base local.get $j i32.add i32.const 2 i32.shl i32.add i32.load offset=0 local.set $x0
              global.get $BSS_PACKET_FLOOR_SEGMENT_X1 local.get $base local.get $j i32.add i32.const 2 i32.shl i32.add i32.load offset=0 local.set $x1
              local.get $i local.get $x0 i32.lt_u br_if $next_seg
              local.get $i local.get $x1 i32.ge_u br_if $next_seg
              global.get $BSS_PACKET_FLOOR_SEGMENT_Y0 local.get $base local.get $j i32.add i32.const 2 i32.shl i32.add i32.load offset=0 local.set $y0
              global.get $BSS_PACKET_FLOOR_SEGMENT_Y1 local.get $base local.get $j i32.add i32.const 2 i32.shl i32.add i32.load offset=0 local.set $y1
              local.get $x1 local.get $x0 i32.sub local.tee $dx
              i32.eqz if
                i32.const 128 local.set $gain
              else
                local.get $y1 local.get $y0 i32.sub
                local.get $i local.get $x0 i32.sub
                i32.mul
                local.get $dx i32.div_s
                local.get $y0 i32.add
                local.set $gain
              end
            )
            br $seg_found
          )
        )
        local.get $gain i32.const 0 i32.lt_s if i32.const 0 local.set $gain end
        local.get $gain i32.const 255 i32.gt_s if i32.const 255 local.set $gain end
        global.get $BSS_FLOOR_GAIN_Q15 local.get $gain i32.const 1 i32.shl i32.add i32.load16_u local.set $g
        global.get $BSS_MDCT_INPUT_Q15 local.get $i i32.const 2 i32.shl i32.add
        global.get $BSS_MDCT_INPUT_Q15 local.get $i i32.const 2 i32.shl i32.add i32.load offset=0
        local.get $g i32.mul
        i32.const 15 i32.shr_s
        i32.store offset=0
        global.get $BSS_LAST_FLOOR_APPLY_COUNT
        global.get $BSS_LAST_FLOOR_APPLY_COUNT i32.load offset=0 i32.const 1 i32.add
        i32.store offset=0
        local.get $val i32.eqz if
        else
          global.get $BSS_LAST_FLOOR_APPLY_NONZERO
          global.get $BSS_LAST_FLOOR_APPLY_NONZERO i32.load offset=0 i32.const 1 i32.add
          i32.store offset=0
        end
        local.get $i i32.const 1 i32.add local.set $i
        br $samp_loop
      )
    )
    global.get $BSS_LAST_FLOOR_APPLY_STATUS i32.const 0 i32.store offset=0
    i32.const 0
  )

  (func (export "vorbis_build_packet_window_q15")
    (param $packet_index i32) (param $block_size i32) (result i32)
    (local $i i32) (local $val i32) (local $left_start i32) (local $left_end i32) (local $right_start i32) (local $right_end i32) (local $left_frames i32) (local $right_frames i32)
    local.get $packet_index global.get $VORBIS_MAX_PACKETS i32.ge_u if global.get $VORBIS_ERR_BAD_INDEX return end
    local.get $block_size i32.eqz if global.get $VORBIS_ERR_BAD_ARCHIVE return end
    local.get $block_size global.get $VORBIS_MAX_BLOCK_SIZE i32.gt_u if global.get $VORBIS_ERR_TOO_MANY_SETUP return end
    global.get $BSS_PACKET_WINDOW_LEFT_START local.get $packet_index i32.const 2 i32.shl i32.add i32.load offset=0 local.set $left_start
    global.get $BSS_PACKET_WINDOW_LEFT_END local.get $packet_index i32.const 2 i32.shl i32.add i32.load offset=0 local.set $left_end
    global.get $BSS_PACKET_WINDOW_RIGHT_START local.get $packet_index i32.const 2 i32.shl i32.add i32.load offset=0 local.set $right_start
    global.get $BSS_PACKET_WINDOW_RIGHT_END local.get $packet_index i32.const 2 i32.shl i32.add i32.load offset=0 local.set $right_end
    global.get $BSS_PACKET_WINDOW_LEFT_FRAMES local.get $packet_index i32.const 2 i32.shl i32.add i32.load offset=0 local.set $left_frames
    global.get $BSS_PACKET_WINDOW_RIGHT_FRAMES local.get $packet_index i32.const 2 i32.shl i32.add i32.load offset=0 local.set $right_frames
    i32.const 0 local.set $i
    (block $done
      (loop $loop
        local.get $i local.get $block_size i32.ge_u br_if $done
        local.get $i local.get $left_start i32.lt_u
        if
          i32.const 0 local.set $val
        else
          local.get $i local.get $right_end i32.ge_u
          if
            i32.const 0 local.set $val
          else
            local.get $i local.get $left_end i32.lt_u
            if
              local.get $i local.get $left_start i32.sub i32.const 1 i32.add
              i32.const 32767 i32.mul
              local.get $left_frames
              i32.div_s
              local.set $val
            else
              local.get $i local.get $right_start i32.ge_u
              if
                local.get $right_end local.get $i i32.sub
                i32.const 32767 i32.mul
                local.get $right_frames
                i32.div_s
                local.set $val
              else
                i32.const 32767 local.set $val
              end
            end
          end
        end
        global.get $BSS_WINDOW_Q15 local.get $i i32.const 2 i32.shl i32.add local.get $val i32.store offset=0
        local.get $i i32.const 1 i32.add local.set $i
        br $loop
      )
    )
    i32.const 0
  )

  (func (export "vorbis_run_owned_lapped_transform_q15")
    (param $block_size i32) (result i32)
    (local $bin_count i32) (local $phase_scale i32) (local $n i32) (local $k i32)
    (local $acc i32) (local $phase i32) (local $cos i32) (local $val i32) (local $tmp i32)
    local.get $block_size i32.eqz if global.get $VORBIS_ERR_BAD_ARCHIVE return end
    local.get $block_size global.get $VORBIS_MAX_BLOCK_SIZE i32.gt_u if global.get $VORBIS_ERR_TOO_MANY_SETUP return end
    global.get $BSS_LAST_RESIDUE_WORK_VECTOR_COUNT i32.load offset=0 local.tee $bin_count
    i32.eqz if global.get $VORBIS_ERR_BAD_ARCHIVE return end
    local.get $bin_count global.get $VORBIS_MAX_BLOCK_SIZE i32.gt_u if global.get $VORBIS_ERR_TOO_MANY_SETUP return end
    local.get $block_size i32.const 1 i32.shr_u local.tee $tmp
    local.get $bin_count local.get $tmp i32.gt_u if local.get $tmp local.set $bin_count end
    local.get $bin_count i32.eqz if global.get $VORBIS_ERR_BAD_ARCHIVE return end
    global.get $BSS_LAST_IMDCT_KERNEL_BINS local.get $bin_count i32.store offset=0
    i32.const 8192 local.get $block_size i32.div_u local.tee $phase_scale
    i32.eqz if global.get $VORBIS_ERR_BAD_ARCHIVE return end
    global.get $BSS_LAST_IMDCT_KERNEL_CROSS_TERMS i32.const 0 i32.store offset=0
    global.get $BSS_LAST_IMDCT_KERNEL_SAMPLES i32.const 0 i32.store offset=0
    i32.const 0 local.set $n
    (block $done
      (loop $samp_loop
        local.get $n local.get $block_size i32.ge_u br_if $done
        local.get $n i32.const 1 i32.shl i32.const 1 i32.or local.get $block_size i32.add
        local.get $phase_scale i32.mul local.set $phase
        i32.const 0 local.set $acc
        i32.const 0 local.set $k
        (block $bin_done
          (loop $bin_loop
            local.get $k local.get $bin_count i32.ge_u br_if $bin_done
            local.get $k i32.const 1 i32.shl i32.const 1 i32.or
            local.get $phase i32.mul
            call $cos_q15_from_phase
            local.set $cos
            global.get $BSS_MDCT_INPUT_Q15 local.get $k i32.const 2 i32.shl i32.add i32.load offset=0
            local.get $cos i32.mul
            i32.const 15 i32.shr_s
            local.get $acc i32.add
            local.set $acc
            local.get $k i32.const 1 i32.add local.set $k
            br $bin_loop
          )
        )
        global.get $BSS_LAST_IMDCT_KERNEL_SAMPLES
        global.get $BSS_LAST_IMDCT_KERNEL_SAMPLES i32.load offset=0 i32.const 1 i32.add
        i32.store offset=0
        global.get $BSS_WINDOW_Q15 local.get $n i32.const 2 i32.shl i32.add i32.load offset=0
        local.get $acc i32.const 3 i32.shr_s
        i32.mul
        i32.const 15 i32.shr_s
        local.tee $val
        i32.const 32767 i32.gt_s if i32.const 32767 local.set $val end
        local.get $val i32.const -32768 i32.lt_s if i32.const -32768 local.set $val end
        global.get $BSS_MDCT_OUTPUT_Q15 local.get $n i32.const 2 i32.shl i32.add local.get $val i32.store offset=0
        local.get $n i32.const 1 i32.add local.set $n
        br $samp_loop
      )
    )
    i32.const 0 return
  )

  (func (export "vorbis_publish_floor_residue_pcm")
    (param $sample i32) (result i32)
    (local $frame_count i32) (local $work_count i32) (local $i i32) (local $val i32)
    local.get $sample global.get $SS_FRAME_COUNT i32.add i32.load offset=0 local.tee $frame_count
    i32.const 0 i32.le_s if global.get $VORBIS_ERR_BAD_ARCHIVE return end
    local.get $frame_count global.get $VORBIS_MAX_PCM_FRAMES i32.gt_u if global.get $VORBIS_ERR_PCM_TOO_LARGE return end
    global.get $BSS_LAST_FLOOR_APPLY_NONZERO i32.load offset=0 i32.eqz if global.get $VORBIS_ERR_BAD_ARCHIVE return end
    global.get $BSS_LAST_RESIDUE_WORK_VECTOR_COUNT i32.load offset=0 local.tee $work_count
    i32.eqz if global.get $VORBIS_ERR_BAD_ARCHIVE return end
    local.get $work_count global.get $VORBIS_MAX_BLOCK_SIZE i32.gt_u if global.get $VORBIS_ERR_PCM_TOO_LARGE return end
    i32.const 0 local.set $i
    (block $done
      (loop $loop
        local.get $i local.get $frame_count i32.ge_u br_if $done
        global.get $BSS_MDCT_INPUT_Q15 local.get $i local.get $work_count i32.rem_u i32.const 2 i32.shl i32.add i32.load offset=0
        local.tee $val
        i32.const 32767 i32.gt_s if i32.const 32767 local.set $val end
        local.get $val i32.const -32768 i32.lt_s if i32.const -32768 local.set $val end
        global.get $BSS_FLOOR_RESIDUE_PCM local.get $i i32.const 1 i32.shl i32.add local.get $val i32.store16 offset=0
        local.get $i i32.const 1 i32.add local.set $i
        br $loop
      )
    )
    local.get $sample global.get $SS_PCM_PTR i32.add global.get $BSS_FLOOR_RESIDUE_PCM i64.extend_i32_u i64.store offset=0
    global.get $BSS_LAST_FLOOR_RESIDUE_PCM_FRAMES local.get $frame_count i32.store offset=0
    global.get $BSS_LAST_FLOOR_RESIDUE_PCM_STATUS i32.const 0 i32.store offset=0
    i32.const 0
  )

  (func $vorbis_synthesize_mdct_window_pcm (export "vorbis_synthesize_mdct_window_pcm")
    (param $sample i32) (result i32)
    (local $packet_idx i32) (local $frame_cursor i32) (local $block_size i32) (local $half i32)
    (local $i i32) (local $val i32) (local $frame_count i32) (local $packet_count i32)
    (local $synth_nonzero i32) (local $overlap_nonzero i32) (local $synth_packets i32)
    global.get $BSS_LAST_MDCT_SYNTH_PACKETS i32.const 0 i32.store offset=0
    global.get $BSS_LAST_MDCT_SYNTH_FRAMES i32.const 0 i32.store offset=0
    global.get $BSS_LAST_MDCT_SYNTH_NONZERO i32.const 0 i32.store offset=0
    global.get $BSS_LAST_MDCT_SYNTH_OVERLAP_NONZERO i32.const 0 i32.store offset=0
    global.get $BSS_LAST_IMDCT_KERNEL_BINS i32.const 0 i32.store offset=0
    global.get $BSS_LAST_IMDCT_KERNEL_SAMPLES i32.const 0 i32.store offset=0
    global.get $BSS_LAST_IMDCT_KERNEL_CROSS_TERMS i32.const 0 i32.store offset=0
    global.get $BSS_LAST_FLOOR_RESIDUE_PCM_STATUS global.get $VORBIS_ERR_TODO_DECODE i32.store offset=0
    global.get $BSS_LAST_FLOOR_RESIDUE_PCM_FRAMES i32.const 0 i32.store offset=0
    local.get $sample global.get $SS_FRAME_COUNT i32.add i32.load offset=0
    local.tee $frame_count
    i32.const 0 i32.le_s
    if
      global.get $VORBIS_ERR_BAD_ARCHIVE
      global.get $BSS_LAST_MDCT_SYNTH_STATUS i32.store offset=0
      global.get $VORBIS_ERR_BAD_ARCHIVE return
    end
    local.get $frame_count global.get $VORBIS_MAX_PCM_FRAMES i32.gt_u
    if
      global.get $VORBIS_ERR_PCM_TOO_LARGE
      global.get $BSS_LAST_MDCT_SYNTH_STATUS i32.store offset=0
      global.get $VORBIS_ERR_PCM_TOO_LARGE return
    end
    local.get $sample global.get $SS_PACKET_COUNT i32.add i32.load offset=0
    local.tee $packet_count
    i32.eqz
    if
      global.get $VORBIS_ERR_BAD_ARCHIVE
      global.get $BSS_LAST_MDCT_SYNTH_STATUS i32.store offset=0
      global.get $VORBIS_ERR_BAD_ARCHIVE return
    end
    local.get $packet_count global.get $BSS_LAST_PACKET_COUNT i32.load offset=0
    i32.ne
    if
      global.get $VORBIS_ERR_BAD_ARCHIVE
      global.get $BSS_LAST_MDCT_SYNTH_STATUS i32.store offset=0
      global.get $VORBIS_ERR_BAD_ARCHIVE return
    end
    i32.const 0 local.set $i
    (block $zero_pcm_done
      (loop $zero_pcm_loop
        local.get $i global.get $VORBIS_MAX_PCM_FRAMES i32.ge_u br_if $zero_pcm_done
        global.get $BSS_MDCT_SYNTH_PCM local.get $i i32.const 1 i32.shl i32.add i32.const 0 i32.store16 offset=0
        local.get $i i32.const 1 i32.add local.set $i
        br $zero_pcm_loop
      )
    )
    i32.const 0 local.set $i
    (block $zero_overlap_done
      (loop $zero_overlap_loop
        local.get $i global.get $VORBIS_MAX_OVERLAP_FRAMES i32.ge_u br_if $zero_overlap_done
        global.get $BSS_OVERLAP_Q15 local.get $i i32.const 2 i32.shl i32.add i32.const 0 i32.store offset=0
        local.get $i i32.const 1 i32.add local.set $i
        br $zero_overlap_loop
      )
    )
    i32.const 0 local.set $packet_idx
    i32.const 0 local.set $frame_cursor
    i32.const 0 local.set $synth_nonzero
    i32.const 0 local.set $overlap_nonzero
    i32.const 0 local.set $synth_packets
    (block $publish
      (loop $packet_loop
        local.get $packet_idx local.get $packet_count i32.ge_u br_if $publish
        local.get $frame_cursor local.get $frame_count i32.ge_u br_if $publish
        local.get $packet_idx call $vorbis_build_residue_work_vector_q15
        local.tee $val
        if
          global.get $BSS_LAST_MDCT_SYNTH_STATUS local.get $val i32.store offset=0
          local.get $val return
        end
        local.get $packet_idx call $vorbis_apply_floor_curve_to_residue_q15
        local.tee $val
        if
          global.get $BSS_LAST_MDCT_SYNTH_STATUS local.get $val i32.store offset=0
          local.get $val return
        end
        global.get $BSS_PACKET_BLOCK_SIZES local.get $packet_idx i32.const 2 i32.shl i32.add i32.load offset=0
        local.tee $block_size
        i32.eqz
        if
          global.get $VORBIS_ERR_BAD_ARCHIVE
          global.get $BSS_LAST_MDCT_SYNTH_STATUS i32.store offset=0
          global.get $VORBIS_ERR_BAD_ARCHIVE return
        end
        local.get $block_size global.get $VORBIS_MAX_BLOCK_SIZE i32.gt_u
        if
          global.get $VORBIS_ERR_TOO_MANY_SETUP
          global.get $BSS_LAST_MDCT_SYNTH_STATUS i32.store offset=0
          global.get $VORBIS_ERR_TOO_MANY_SETUP return
        end
        local.get $block_size i32.const 1 i32.shr_u
        local.tee $half
        global.get $VORBIS_MAX_OVERLAP_FRAMES i32.gt_u
        if
          global.get $VORBIS_ERR_TOO_MANY_SETUP
          global.get $BSS_LAST_MDCT_SYNTH_STATUS i32.store offset=0
          global.get $VORBIS_ERR_TOO_MANY_SETUP return
        end
        local.get $packet_idx local.get $block_size call $vorbis_build_packet_window_q15
        local.tee $val
        if
          global.get $BSS_LAST_MDCT_SYNTH_STATUS local.get $val i32.store offset=0
          local.get $val return
        end
        local.get $block_size call $vorbis_run_owned_lapped_transform_q15
        local.tee $val
        if
          global.get $BSS_LAST_MDCT_SYNTH_STATUS local.get $val i32.store offset=0
          local.get $val return
        end
        i32.const 0 local.set $i
        (block $save_overlap
          (loop $left_loop
            local.get $i local.get $half i32.ge_u br_if $save_overlap
            local.get $frame_cursor local.get $frame_count i32.ge_u br_if $save_overlap
            global.get $BSS_MDCT_OUTPUT_Q15 local.get $i i32.const 2 i32.shl i32.add i32.load offset=0
            global.get $BSS_OVERLAP_Q15 local.get $i i32.const 2 i32.shl i32.add i32.load offset=0
            i32.add
            local.tee $val
            i32.const 32767 i32.gt_s if i32.const 32767 local.set $val end
            local.get $val i32.const -32768 i32.lt_s if i32.const -32768 local.set $val end
            global.get $BSS_MDCT_SYNTH_PCM local.get $frame_cursor i32.const 1 i32.shl i32.add local.get $val i32.store16 offset=0
            local.get $val i32.eqz if else
              local.get $synth_nonzero i32.const 1 i32.add local.set $synth_nonzero
            end
            local.get $frame_cursor i32.const 1 i32.add local.set $frame_cursor
            local.get $i i32.const 1 i32.add local.set $i
            br $left_loop
          )
        )
        i32.const 0 local.set $i
        (block $next_packet
          (loop $overlap_loop
            local.get $i local.get $half i32.ge_u br_if $next_packet
            global.get $BSS_MDCT_OUTPUT_Q15 local.get $half local.get $i i32.add i32.const 2 i32.shl i32.add i32.load offset=0
            local.tee $val
            global.get $BSS_OVERLAP_Q15 local.get $i i32.const 2 i32.shl i32.add i32.store offset=0
            local.get $val i32.eqz if else
              local.get $overlap_nonzero i32.const 1 i32.add local.set $overlap_nonzero
            end
            local.get $i i32.const 1 i32.add local.set $i
            br $overlap_loop
          )
        )
        local.get $synth_packets i32.const 1 i32.add local.set $synth_packets
        local.get $packet_idx i32.const 1 i32.add local.set $packet_idx
        br $packet_loop
      )
    )
    local.get $frame_cursor i32.eqz
    if
      global.get $VORBIS_ERR_BAD_ARCHIVE
      global.get $BSS_LAST_MDCT_SYNTH_STATUS i32.store offset=0
      global.get $VORBIS_ERR_BAD_ARCHIVE return
    end
    (block $publish_ready
      (loop $tail_loop
        local.get $frame_cursor local.get $frame_count i32.ge_u br_if $publish_ready
        global.get $BSS_MDCT_SYNTH_PCM local.get $frame_cursor i32.const 1 i32.shl i32.add i32.const 0 i32.store16 offset=0
        local.get $frame_cursor i32.const 1 i32.add local.set $frame_cursor
        br $tail_loop
      )
    )
    local.get $sample global.get $SS_PCM_PTR i32.add global.get $BSS_MDCT_SYNTH_PCM i64.extend_i32_u i64.store offset=0
    global.get $BSS_LAST_MDCT_SYNTH_FRAMES local.get $frame_count i32.store offset=0
    global.get $BSS_LAST_MDCT_SYNTH_PACKETS local.get $synth_packets i32.store offset=0
    global.get $BSS_LAST_MDCT_SYNTH_NONZERO local.get $synth_nonzero i32.store offset=0
    global.get $BSS_LAST_MDCT_SYNTH_OVERLAP_NONZERO local.get $overlap_nonzero i32.store offset=0
    global.get $BSS_LAST_MDCT_SYNTH_STATUS i32.const 0 i32.store offset=0
    i32.const 0
  )

  (func (export "vorbis_decode_sample_archive")
    (param $data i32) (param $len i32) (param $sample i32) (param $setup i32) (result i32)
    (local $cursor_ptr i32) (local $cursor i32) (local $val i32) (local $br i32)
    (local $i i32) (local $packet_ptr i32) (local $packet_len i32)
    (local $frame_count i32) (local $res i32)
    local.get $sample call $vorbis_setup_init
    global.get $BSS_BITREADER local.set $br
    i32.const 0 local.set $cursor
    local.get $cursor i32.const 4 i32.add local.get $len i32.gt_u
    if global.get $VORBIS_ERR_BOUNDS return end
    local.get $data local.get $cursor call $read_u32be local.set $val
    local.get $sample global.get $SS_SAMPLE_RATE i32.add local.get $val i32.store offset=0
    local.get $cursor i32.const 4 i32.add local.set $cursor
    local.get $data local.get $cursor call $read_u32be local.set $val
    local.get $cursor i32.const 4 i32.add local.set $cursor
    local.get $val i32.const 0 i32.lt_s
    if global.get $VORBIS_ERR_BAD_ARCHIVE return end
    local.get $val global.get $VORBIS_MAX_PCM_FRAMES i32.gt_u
    if global.get $VORBIS_ERR_PCM_TOO_LARGE return end
    local.get $sample global.get $SS_FRAME_COUNT i32.add local.get $val i32.store offset=0
    local.get $data local.get $cursor call $read_u32be local.set $val
    local.get $cursor i32.const 4 i32.add local.set $cursor
    local.get $sample global.get $SS_LOOP_START i32.add local.get $val i32.store offset=0
    local.get $data local.get $cursor call $read_u32be local.set $val
    local.get $cursor i32.const 4 i32.add local.set $cursor
    local.get $val i32.const 0 i32.ge_s
    if
      local.get $sample global.get $SS_LOOP_FLAG i32.add i32.const 0 i32.store offset=0
    else
      local.get $val i32.const -1 i32.xor local.set $val
      local.get $sample global.get $SS_LOOP_FLAG i32.add i32.const 1 i32.store offset=0
    end
    local.get $sample global.get $SS_LOOP_END i32.add local.get $val i32.store offset=0
    local.get $data local.get $cursor call $read_u32be local.set $val
    local.get $cursor i32.const 4 i32.add local.set $cursor
    local.get $val i32.const 0 i32.lt_s
    if global.get $VORBIS_ERR_BAD_ARCHIVE return end
    local.get $val global.get $VORBIS_MAX_PACKETS i32.gt_u
    if global.get $VORBIS_ERR_TOO_MANY_PACKETS return end
    local.get $sample global.get $SS_PACKET_COUNT i32.add local.get $val i32.store offset=0
    i32.const 0 local.set $i
    (block $pkt_done
      (loop $pkt_loop
        local.get $i local.get $sample global.get $SS_PACKET_COUNT i32.add i32.load offset=0 i32.ge_u br_if $pkt_done
        i32.const 0 local.set $packet_len
        (block $len_done
          (loop $len_loop
            local.get $cursor local.get $len i32.ge_u if global.get $VORBIS_ERR_BOUNDS return end
            local.get $data local.get $cursor i32.add i32.load8_u local.set $val
            local.get $cursor i32.const 1 i32.add local.set $cursor
            local.get $packet_len local.get $val i32.add local.set $packet_len
            local.get $val i32.const 255 i32.eq
            br_if $len_loop
          )
        )
        local.get $sample global.get $SS_PACKET_LENS i32.add local.get $i i32.const 2 i32.shl i32.add local.get $packet_len i32.store offset=0
        local.get $data local.get $cursor i32.add local.set $packet_ptr
        local.get $sample global.get $SS_PACKET_PTRS i32.add local.get $i i32.const 3 i32.shl i32.add local.get $packet_ptr i64.extend_i32_u i64.store offset=0
        local.get $cursor local.get $packet_len i32.add local.set $cursor
        local.get $cursor local.get $len i32.gt_u
        if global.get $VORBIS_ERR_BOUNDS return end
        local.get $i i32.const 1 i32.add local.set $i
        br $pkt_loop
      )
    )
    ;; Decode packet headers (sample, setup)
    local.get $sample local.get $setup call $vorbis_decode_sample_packets
    local.tee $res
    if
      local.get $sample global.get $SS_DECODE_STATUS i32.add local.get $res i32.store offset=0
      global.get $BSS_LAST_PCM_STATUS local.get $res i32.store offset=0
      local.get $res return
    end
    ;; Try silence path
    local.get $sample call $vorbis_publish_silence_if_all_floor_false
    if
      ;; Non-silent: try MDCT synthesis
      local.get $sample call $vorbis_synthesize_mdct_window_pcm
      if
        ;; Non-silent fail-closed
        local.get $sample global.get $SS_PCM_PTR i32.add i64.const 0 i64.store offset=0
        local.get $sample global.get $SS_DECODE_STATUS i32.add global.get $VORBIS_ERR_TODO_DECODE i32.store offset=0
        global.get $BSS_LAST_PCM_STATUS global.get $VORBIS_ERR_TODO_DECODE i32.store offset=0
      else
        ;; Non-silent success
        local.get $sample global.get $SS_DECODE_STATUS i32.add i32.const 0 i32.store offset=0
        global.get $BSS_LAST_PCM_STATUS i32.const 0 i32.store offset=0
      end
    else
      ;; Silence success
      local.get $sample global.get $SS_DECODE_STATUS i32.add i32.const 0 i32.store offset=0
      global.get $BSS_LAST_PCM_STATUS i32.const 0 i32.store offset=0
    end
    ;; Publish rawsound (even on fail-closed)
    local.get $sample call $vorbis_publish_rawsound
    i32.const 0
  )

  (func (export "vorbis_publish_silence_if_all_floor_false")
    (param $sample i32) (result i32)
    (local $i i32) (local $packet_count i32)
    global.get $BSS_LAST_SILENT_PACKET_COUNT i32.const 0 i32.store offset=0
    local.get $sample global.get $SS_PACKET_COUNT i32.add i32.load offset=0 local.tee $packet_count
    i32.eqz if
      local.get $sample global.get $SS_PCM_PTR i32.add i64.const 0 i64.store offset=0
      global.get $VORBIS_ERR_TODO_DECODE return
    end
    local.get $packet_count global.get $BSS_LAST_PACKET_COUNT i32.load offset=0 i32.ne if
      local.get $sample global.get $SS_PCM_PTR i32.add i64.const 0 i64.store offset=0
      global.get $VORBIS_ERR_TODO_DECODE return
    end
    local.get $sample global.get $SS_FRAME_COUNT i32.add i32.load offset=0 i32.const 0 i32.le_s if
      local.get $sample global.get $SS_PCM_PTR i32.add i64.const 0 i64.store offset=0
      global.get $VORBIS_ERR_TODO_DECODE return
    end
    i32.const 0 local.set $i
    (block $publish
      (loop $loop
        local.get $i local.get $packet_count i32.ge_u br_if $publish
        global.get $BSS_PACKET_FLOOR_NONZERO local.get $i i32.const 2 i32.shl i32.add i32.load offset=0
        i32.const 0 i32.ne if
          local.get $sample global.get $SS_PCM_PTR i32.add i64.const 0 i64.store offset=0
          global.get $VORBIS_ERR_TODO_DECODE return
        end
        global.get $BSS_LAST_SILENT_PACKET_COUNT
        global.get $BSS_LAST_SILENT_PACKET_COUNT i32.load offset=0 i32.const 1 i32.add
        i32.store offset=0
        local.get $i i32.const 1 i32.add local.set $i
        br $loop
      )
    )
    local.get $sample global.get $SS_PCM_PTR i32.add global.get $BSS_SILENCE_PCM i64.extend_i32_u i64.store offset=0
    i32.const 0
  )

  (func (export "vorbis_sample_get_rawsound")
    (param $sample i32) (param $raw i32) (result i32)
    (local $ptr i32)
    local.get $sample i32.eqz
    if
      global.get $BSS_SAMPLE_STATE local.set $sample
    end
    local.get $raw if else global.get $BSS_RAWSOUND_VIEW local.set $raw end
    local.get $raw global.get $RS_SAMPLE_RATE i32.add local.get $sample global.get $SS_SAMPLE_RATE i32.add i32.load offset=0 i32.store offset=0
    local.get $raw global.get $RS_FRAME_COUNT i32.add local.get $sample global.get $SS_FRAME_COUNT i32.add i32.load offset=0 i32.store offset=0
    local.get $raw global.get $RS_LOOP_START i32.add local.get $sample global.get $SS_LOOP_START i32.add i32.load offset=0 i32.store offset=0
    local.get $raw global.get $RS_LOOP_END i32.add local.get $sample global.get $SS_LOOP_END i32.add i32.load offset=0 i32.store offset=0
    local.get $raw global.get $RS_LOOP_FLAG i32.add local.get $sample global.get $SS_LOOP_FLAG i32.add i32.load offset=0 i32.store offset=0
    local.get $raw global.get $RS_PCM_PTR i32.add local.get $sample global.get $SS_PCM_PTR i32.add i64.load offset=0 i64.store offset=0
    local.get $raw
  )

  (func (export "vorbis_sample_get_pcm")
    (param $sample i32) (result i32) (result i32) (result i32)
    local.get $sample i32.eqz
    if
      global.get $BSS_SAMPLE_STATE local.set $sample
    end
    local.get $sample global.get $SS_PCM_PTR i32.add i64.load offset=0 i32.wrap_i64
    local.get $sample global.get $SS_FRAME_COUNT i32.add i32.load offset=0
    local.get $sample global.get $SS_SAMPLE_RATE i32.add i32.load offset=0
  )

  (func (export "vorbis_publish_rawsound")
    (param $sample i32) (result i32)
    global.get $BSS_RAWSOUND_VIEW global.get $RS_SAMPLE_RATE i32.add local.get $sample global.get $SS_SAMPLE_RATE i32.add i32.load offset=0 i32.store offset=0
    global.get $BSS_RAWSOUND_VIEW global.get $RS_FRAME_COUNT i32.add local.get $sample global.get $SS_FRAME_COUNT i32.add i32.load offset=0 i32.store offset=0
    global.get $BSS_RAWSOUND_VIEW global.get $RS_LOOP_START i32.add local.get $sample global.get $SS_LOOP_START i32.add i32.load offset=0 i32.store offset=0
    global.get $BSS_RAWSOUND_VIEW global.get $RS_LOOP_END i32.add local.get $sample global.get $SS_LOOP_END i32.add i32.load offset=0 i32.store offset=0
    global.get $BSS_RAWSOUND_VIEW global.get $RS_LOOP_FLAG i32.add local.get $sample global.get $SS_LOOP_FLAG i32.add i32.load offset=0 i32.store offset=0
    global.get $BSS_RAWSOUND_VIEW global.get $RS_PCM_PTR i32.add local.get $sample global.get $SS_PCM_PTR i32.add i64.load offset=0 i64.store offset=0
    global.get $BSS_RAWSOUND_VIEW
  )

  ;; TODO stubs
  (func (export "vorbis_todo_decode_codebooks") (result i32)
    global.get $VORBIS_ERR_TODO_DECODE
  )
  (func (export "vorbis_todo_decode_floors") (result i32)
    global.get $VORBIS_ERR_TODO_DECODE
  )
  (func (export "vorbis_todo_decode_residues") (result i32)
    global.get $VORBIS_ERR_TODO_DECODE
  )
  (func (export "vorbis_todo_decode_mappings") (result i32)
    global.get $VORBIS_ERR_TODO_DECODE
  )
  (func (export "vorbis_todo_inverse_mdct") (result i32)
    global.get $VORBIS_ERR_TODO_DECODE
  )

  ;; Cos quarter table (257 x i16)
  (data (i32.const 0x020CBD40) "\ff\7f\fe\7f\fd\7f\f9\7f\f5\7f\f0\7f\e9\7f\e1\7f\d8\7f\cd\7f\c1\7f\b4\7f\a6\7f\97\7f\86\7f\74\7f\61\7f\4d\7f\39\7f\23\7f\09\7f\ef\7e\d3\7e\b5\7e\95\7e\74\7e\52\7e\2e\7e\09\7e\e2\7d\ba\7d\90\7d\64\7d\37\7d\09\7d\da\7c\aa\7c\78\7c\45\7c\11\7c\db\7b\a4\7b\6c\7b\33\7b\f8\7a\bc\7a\7f\7a\41\7a\01\7a\c1\79\7f\79\3c\79\f8\78\b3\78\6d\78\26\78\dd\77\94\77\4a\77\fe\76\b2\76\65\76\16\76\c7\75\77\75\25\75\d3\74\7f\74\2b\74\d5\73\7f\73\28\73\cf\72\76\72\1c\72\c1\71\65\71\08\71\aa\70\4b\70\eb\6f\8b\6f\29\6f\c7\6e\64\6e\00\6e\9b\6d\35\6d\cf\6c\67\6c\ff\6b\96\6b\2c\6b\c1\6a\56\6a\e9\69\7c\69\0e\69\9f\68\30\68\bf\67\4e\67\dc\66\69\66\f5\65\81\65\0c\65\96\64\1f\64\a8\63\30\63\b7\62\3d\62\c3\61\48\61\cc\60\50\60\d3\5f\55\5f\d7\5e\58\5e\d8\5d\57\5d\d6\5c\54\5c\d2\5b\4e\5b\cb\5a\46\5a\c1\59\3b\59\b5\58\2e\58\a6\57\1e\57\95\56\0c\56\82\55\f7\54\6c\54\e0\53\54\53\c7\52\3a\52\ac\51\1d\51\8e\50\fe\4f\6e\4f\dd\4e\4c\4e\ba\4d\27\4d\94\4c\01\4c\6c\4b\d8\4a\42\4a\ad\49\16\49\7f\48\e8\47\50\47\b8\46\1f\46\86\45\ec\44\52\44\b7\43\1c\43\80\42\e4\41\48\41\ab\40\0e\40\70\3f\d2\3e\33\3e\94\3d\f4\3c\54\3c\b4\3b\13\3b\72\3a\d0\39\2e\39\8c\38\e9\37\46\37\a2\36\fe\35\5a\35\b5\34\10\34\6b\33\c5\32\1f\32\78\31\d1\30\2a\30\82\2f\da\2e\32\2e\89\2d\e0\2c\37\2c\8d\2b\e3\2a\39\2a\8e\29\e3\28\38\28\8c\27\e0\26\34\26\87\25\da\24\2d\24\7f\23\d1\22\23\22\74\21\c5\20\16\20\66\1f\b6\1e\06\1e\56\1d\a5\1c\f4\1b\43\1b\91\1a\df\19\2d\19\7b\18\c8\17\15\17\62\16\ae\15\fb\14\47\14\93\13\df\12\2b\12\76\11\c1\10\0c\10\57\0f\a1\0e\ec\0d\36\0d\80\0c\ca\0b\14\0b\5d\0a\a7\09\f0\08\39\08\82\07\cb\06\14\06\5c\05\a5\04\ed\03\35\03\7d\02\c5\01\0d\01\54\00\9c\00\00\00")

  ;; Floor gain table (256 x i16)
  (data (i32.const 0x020CBF44) "\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\01\00\01\00\01\00\01\00\01\00\01\00\01\00\01\00\01\00\02\00\02\00\02\00\02\00\02\00\03\00\03\00\03\00\03\00\03\00\04\00\04\00\04\00\04\00\05\00\05\00\05\00\06\00\06\00\06\00\07\00\07\00\08\00\08\00\09\00\09\00\0a\00\0b\00\0b\00\0c\00\0d\00\0e\00\0f\00\10\00\11\00\12\00\13\00\14\00\15\00\17\00\18\00\1a\00\1c\00\1d\00\1f\00\21\00\23\00\26\00\28\00\2b\00\2e\00\31\00\34\00\37\00\3a\00\3e\00\43\00\47\00\4b\00\51\00\56\00\5c\00\62\00\69\00\6e\00\76\00\7e\00\86\00\8f\00\98\00\a2\00\ac\00\b7\00\c3\00\cf\00\dc\00\ea\00\f9\00\08\01\19\01\2a\01\3d\01\51\01\66\01\7c\01\94\01\ad\01\c8\01\e4\01\02\02\22\02\43\02\67\02\8d\02\b5\02\df\02\0c\03\3b\03\6d\03\a2\03\da\03\15\04\54\04\96\04\dc\04\26\05\75\05\c8\05\21\06\7f\06\e2\06\4c\07\bc\07\33\08\af\08\34\09\bf\09\51\0a\ec\0a\8e\0b\39\0c\ed\0c\aa\0d\71\0e\43\0f\20\10\09\11\ff\11\02\13\14\14\34\15\65\16\aa\17\00\19\6b\1a\ed\1b\87\1d\3b\1f\0a\21\f8\22\06\25\37\27\8e\29\0f\2c\bb\2e\97\31\ff\7f")
)
