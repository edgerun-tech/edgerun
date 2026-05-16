package spec

import (
	"github.com/spf13/cobra"

	pb "edgerunrefcore/proto/go/spec"
)

// ---- Hardcoded Extractors ----

func runEncoding(cmd *cobra.Command, args []string) error {
	cat := &pb.Catalog{SpecName: "Encoding Living Standard"}
	for _, e := range encodingTable {
		cat.Encodings = append(cat.Encodings, &pb.EncodingDef{
			Name: e.name, Labels: e.labels, Type: e.typ, Endianness: e.endian,
		})
	}
	for _, b := range bomTable {
		cat.BomTable = append(cat.BomTable, &pb.BomEntry{Encoding: b.enc, Bytes: b.bytes})
	}
	return writeProto(cat)
}

type encEntry struct {
	name   string
	labels []string
	typ    pb.EncodingType
	endian string
}

var encodingTable = []encEntry{
	{"UTF-8", []string{"unicode-1-1-utf-8", "utf-8", "utf8"}, pb.EncodingType_ENCODING_TYPE_UTF8, ""},
	{"UTF-16BE", []string{"utf-16be"}, pb.EncodingType_ENCODING_TYPE_UTF16, "big"},
	{"UTF-16LE", []string{"utf-16le"}, pb.EncodingType_ENCODING_TYPE_UTF16, "little"},
	{"IBM866", []string{"866", "cp866", "csibm866", "ibm866"}, pb.EncodingType_ENCODING_TYPE_LEGACY, ""},
	{"ISO-8859-2", []string{"csisolatin2", "iso-8859-2", "iso-ir-101", "iso8859-2", "iso88592", "iso_8859-2", "l2", "latin2"}, pb.EncodingType_ENCODING_TYPE_LEGACY, ""},
	{"ISO-8859-3", []string{"csisolatin3", "iso-8859-3", "iso-ir-109", "iso8859-3", "iso88593", "iso_8859-3", "l3", "latin3"}, pb.EncodingType_ENCODING_TYPE_LEGACY, ""},
	{"ISO-8859-4", []string{"csisolatin4", "iso-8859-4", "iso-ir-110", "iso8859-4", "iso88594", "iso_8859-4", "l4", "latin4"}, pb.EncodingType_ENCODING_TYPE_LEGACY, ""},
	{"ISO-8859-5", []string{"csisolatincyrillic", "cyrillic", "iso-8859-5", "iso-ir-144", "iso8859-5", "iso88595", "iso_8859-5"}, pb.EncodingType_ENCODING_TYPE_LEGACY, ""},
	{"ISO-8859-6", []string{"arabic", "asmo-708", "csiso88596e", "csiso88596i", "csisolatinarabic", "ecma-114", "iso-8859-6", "iso-8859-6-e", "iso-8859-6-i", "iso-ir-127", "iso8859-6", "iso88596"}, pb.EncodingType_ENCODING_TYPE_LEGACY, ""},
	{"ISO-8859-7", []string{"csisolatingreek", "ecma-118", "elot_928", "greek", "greek8", "iso-8859-7", "iso-ir-126", "iso8859-7", "iso88597", "iso_8859-7"}, pb.EncodingType_ENCODING_TYPE_LEGACY, ""},
	{"ISO-8859-8", []string{"csisolatinhebrew", "hebrew", "iso-8859-8", "iso-8859-8-e", "iso-ir-138", "iso8859-8", "iso88598", "iso_8859-8", "visual"}, pb.EncodingType_ENCODING_TYPE_LEGACY, ""},
	{"ISO-8859-8-I", []string{"csiso88598i", "iso-8859-8-i", "logical"}, pb.EncodingType_ENCODING_TYPE_LEGACY, ""},
	{"ISO-8859-10", []string{"csisolatin6", "iso-8859-10", "iso-ir-157", "iso8859-10", "iso885910", "iso_8859-10", "l6", "latin6"}, pb.EncodingType_ENCODING_TYPE_LEGACY, ""},
	{"ISO-8859-13", []string{"iso-8859-13", "iso8859-13", "iso885913"}, pb.EncodingType_ENCODING_TYPE_LEGACY, ""},
	{"ISO-8859-14", []string{"iso-8859-14", "iso8859-14", "iso885914"}, pb.EncodingType_ENCODING_TYPE_LEGACY, ""},
	{"ISO-8859-15", []string{"csisolatin9", "iso-8859-15", "iso8859-15", "iso885915", "iso_8859-15", "l9"}, pb.EncodingType_ENCODING_TYPE_LEGACY, ""},
	{"ISO-8859-16", []string{"iso-8859-16"}, pb.EncodingType_ENCODING_TYPE_LEGACY, ""},
	{"KOI8-R", []string{"cskoi8r", "koi", "koi8", "koi8-r", "koi8_r"}, pb.EncodingType_ENCODING_TYPE_LEGACY, ""},
	{"KOI8-U", []string{"koi8-u"}, pb.EncodingType_ENCODING_TYPE_LEGACY, ""},
	{"macintosh", []string{"csmacintosh", "mac", "macintosh", "x-mac-roman"}, pb.EncodingType_ENCODING_TYPE_LEGACY, ""},
	{"windows-874", []string{"dos-874", "iso-8859-11", "iso8859-11", "iso885911", "tis-620", "windows-874"}, pb.EncodingType_ENCODING_TYPE_LEGACY, ""},
	{"windows-1250", []string{"cp1250", "windows-1250", "x-cp1250"}, pb.EncodingType_ENCODING_TYPE_LEGACY, ""},
	{"windows-1251", []string{"cp1251", "windows-1251", "x-cp1251"}, pb.EncodingType_ENCODING_TYPE_LEGACY, ""},
	{"windows-1252", []string{"ansi_x3.4-1968", "ascii", "cp1252", "cp819", "csisolatin1", "ibm819", "iso-8859-1", "iso-ir-100", "iso8859-1", "iso88591", "iso_8859-1", "iso_8859-1:1987", "l1", "latin1", "us-ascii", "windows-1252", "x-cp1252"}, pb.EncodingType_ENCODING_TYPE_LEGACY, ""},
	{"windows-1253", []string{"cp1253", "windows-1253", "x-cp1253"}, pb.EncodingType_ENCODING_TYPE_LEGACY, ""},
	{"windows-1254", []string{"cp1254", "csisolatin5", "iso-8859-9", "iso-ir-148", "iso8859-9", "iso88599", "iso_8859-9", "l5", "latin5", "windows-1254", "x-cp1254"}, pb.EncodingType_ENCODING_TYPE_LEGACY, ""},
	{"windows-1255", []string{"cp1255", "windows-1255", "x-cp1255"}, pb.EncodingType_ENCODING_TYPE_LEGACY, ""},
	{"windows-1256", []string{"cp1256", "windows-1256", "x-cp1256"}, pb.EncodingType_ENCODING_TYPE_LEGACY, ""},
	{"windows-1257", []string{"cp1257", "windows-1257", "x-cp1257"}, pb.EncodingType_ENCODING_TYPE_LEGACY, ""},
	{"windows-1258", []string{"cp1258", "windows-1258", "x-cp1258"}, pb.EncodingType_ENCODING_TYPE_LEGACY, ""},
	{"x-mac-cyrillic", []string{"x-mac-cyrillic", "x-mac-ukrainian"}, pb.EncodingType_ENCODING_TYPE_LEGACY, ""},
	{"GBK", []string{"chinese", "csgb2312", "csiso58gb231280", "gb2312", "gb_2312", "gb_2312-80", "gbk", "hz-gb-2312", "iso-ir-58", "x-gbk"}, pb.EncodingType_ENCODING_TYPE_LEGACY, ""},
	{"gb18030", []string{"gb18030"}, pb.EncodingType_ENCODING_TYPE_LEGACY, ""},
	{"Big5", []string{"big5", "big5-hkscs", "cn-big5", "csbig5", "x-x-big5"}, pb.EncodingType_ENCODING_TYPE_LEGACY, ""},
	{"EUC-JP", []string{"cseucpkdfmtjapanese", "euc-jp", "x-euc-jp"}, pb.EncodingType_ENCODING_TYPE_LEGACY, ""},
	{"ISO-2022-JP", []string{"csiso2022jp", "iso-2022-jp"}, pb.EncodingType_ENCODING_TYPE_LEGACY, ""},
	{"Shift_JIS", []string{"csshiftjis", "ms932", "ms_kanji", "shift-jis", "shift_jis", "sjis", "windows-31j", "x-sjis"}, pb.EncodingType_ENCODING_TYPE_LEGACY, ""},
	{"EUC-KR", []string{"cseuckr", "csksc56011987", "euc-kr", "iso-ir-149", "korean", "ks_c_5601-1987", "ks_c_5601-1989", "ksc5601", "ksc_5601", "windows-949"}, pb.EncodingType_ENCODING_TYPE_LEGACY, ""},
	{"replacement", []string{"csiso2022kr", "iso-2022-cn", "iso-2022-cn-ext", "iso-2022-kr", "replacement"}, pb.EncodingType_ENCODING_TYPE_REPLACEMENT, ""},
	{"x-user-defined", []string{"x-user-defined"}, pb.EncodingType_ENCODING_TYPE_LEGACY, ""},
}

type bomEntry struct {
	enc   string
	bytes []byte
}

var bomTable = []bomEntry{
	{"UTF-8", []byte{0xEF, 0xBB, 0xBF}},
	{"UTF-16LE", []byte{0xFF, 0xFE}},
	{"UTF-16BE", []byte{0xFE, 0xFF}},
}
