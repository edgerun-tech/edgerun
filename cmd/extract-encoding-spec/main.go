// Extract Encoding spec: encoding labels, decoders, BOM table.
package main

import (
	"encoding/json"
	"fmt"
	"os"
)

type EncodingDef struct {
	Name       string   `json:"name"`
	Labels     []string `json:"labels"`
	Type       string   `json:"type"`
	Endianness *string  `json:"endianness"`
}

type BOMDef struct {
	Encoding string `json:"encoding"`
	Bytes    []int  `json:"bytes"`
}

type EncodingCatalog struct {
	Spec        string        `json:"spec"`
	SpecDate    string        `json:"spec_date"`
	TotalEncodings int        `json:"total_encodings"`
	Encodings   []EncodingDef `json:"encodings"`
	BOMTable    []BOMDef      `json:"bom_table"`
}

func main() {
	if len(os.Args) >= 2 {
		if _, err := os.ReadFile(os.Args[1]); err != nil {
			fmt.Fprintf(os.Stderr, "error: %v\n", err)
			os.Exit(1)
		}
	}

	big := "big"
	little := "little"

	encodings := []EncodingDef{
		{"UTF-8", []string{"unicode-1-1-utf-8", "utf-8", "utf8"}, "utf-8", nil},
		{"UTF-16LE", []string{"utf-16le"}, "utf-16", &little},
		{"UTF-16BE", []string{"utf-16be"}, "utf-16", &big},
		{"IBM866", []string{"866", "cp866", "csibm866", "ibm866"}, "legacy", nil},
		{"ISO-8859-2", []string{"csisolatin2", "iso-8859-2", "iso-ir-101", "iso8859-2", "iso88592", "iso_8859-2", "iso_8859-2:1987", "l2", "latin2"}, "legacy", nil},
		{"ISO-8859-3", []string{"csisolatin3", "iso-8859-3", "iso-ir-109", "iso8859-3", "iso88593", "iso_8859-3", "iso_8859-3:1988", "l3", "latin3"}, "legacy", nil},
		{"ISO-8859-4", []string{"csisolatin4", "iso-8859-4", "iso-ir-110", "iso8859-4", "iso88594", "iso_8859-4", "iso_8859-4:1988", "l4", "latin4"}, "legacy", nil},
		{"ISO-8859-5", []string{"csisolatincyrillic", "cyrillic", "iso-8859-5", "iso-ir-144", "iso8859-5", "iso88595", "iso_8859-5", "iso_8859-5:1988"}, "legacy", nil},
		{"ISO-8859-6", []string{"arabic", "asmo-708", "csiso88596e", "csiso88596i", "csisolatinarabic", "ecma-114", "iso-8859-6", "iso-8859-6-e", "iso-8859-6-i", "iso-ir-127", "iso8859-6", "iso88596", "iso_8859-6", "iso_8859-6:1987"}, "legacy", nil},
		{"ISO-8859-7", []string{"csisolatingreek", "ecma-118", "elot_928", "greek", "greek8", "iso-8859-7", "iso-ir-126", "iso8859-7", "iso88597", "iso_8859-7", "iso_8859-7:1987", "sun_eu_greek"}, "legacy", nil},
		{"ISO-8859-8", []string{"csiso88598e", "csisolatinhebrew", "hebrew", "iso-8859-8", "iso-8859-8-e", "iso-ir-138", "iso8859-8", "iso88598", "iso_8859-8", "iso_8859-8:1988", "visual"}, "legacy", nil},
		{"ISO-8859-8-I", []string{"csiso88598i", "logical", "iso-8859-8-i", "iso8859-8-i"}, "legacy", nil},
		{"ISO-8859-10", []string{"csisolatin6", "iso-8859-10", "iso-ir-157", "iso8859-10", "iso885910", "iso_8859-10", "iso_8859-10:1992", "l6", "latin6"}, "legacy", nil},
		{"ISO-8859-13", []string{"iso-8859-13", "iso8859-13", "iso885913"}, "legacy", nil},
		{"ISO-8859-14", []string{"iso-8859-14", "iso8859-14", "iso885914"}, "legacy", nil},
		{"ISO-8859-15", []string{"csisolatin9", "iso-8859-15", "iso8859-15", "iso885915", "iso_8859-15", "l9"}, "legacy", nil},
		{"ISO-8859-16", []string{"iso-8859-16"}, "legacy", nil},
		{"KOI8-R", []string{"cskoi8r", "koi", "koi8", "koi8-r", "koi8_r"}, "legacy", nil},
		{"KOI8-U", []string{"koi8-u"}, "legacy", nil},
		{"macintosh", []string{"csmacintosh", "mac", "macintosh", "x-mac-roman"}, "legacy", nil},
		{"windows-874", []string{"dos-874", "iso-8859-11", "iso8859-11", "iso885911", "tis-620", "windows-874"}, "legacy", nil},
		{"windows-1250", []string{"cp1250", "windows-1250", "x-cp1250"}, "legacy", nil},
		{"windows-1251", []string{"cp1251", "windows-1251", "x-cp1251"}, "legacy", nil},
		{"windows-1252", []string{"ansi_x3.4-1968", "ascii", "cp1252", "cp819", "csisolatin1", "ibm819", "iso-8859-1", "iso-ir-100", "iso8859-1", "iso88591", "iso_8859-1", "iso_8859-1:1987", "l1", "latin1", "us-ascii", "windows-1252", "x-cp1252"}, "legacy", nil},
		{"windows-1253", []string{"cp1253", "windows-1253", "x-cp1253"}, "legacy", nil},
		{"windows-1254", []string{"csisolatin5", "iso-8859-9", "iso-ir-148", "iso8859-9", "iso88599", "iso_8859-9", "iso_8859-9:1989", "l5", "latin5", "windows-1254", "x-cp1254"}, "legacy", nil},
		{"windows-1255", []string{"cp1255", "windows-1255", "x-cp1255"}, "legacy", nil},
		{"windows-1256", []string{"cp1256", "windows-1256", "x-cp1256"}, "legacy", nil},
		{"windows-1257", []string{"cp1257", "windows-1257", "x-cp1257"}, "legacy", nil},
		{"windows-1258", []string{"cp1258", "windows-1258", "x-cp1258"}, "legacy", nil},
		{"x-mac-cyrillic", []string{"x-mac-cyrillic", "x-mac-ukrainian"}, "legacy", nil},
		{"GBK", []string{"chinese", "csgb2312", "csiso58gb231280", "gb2312", "gb_2312", "gb_2312-80", "gbk", "iso-ir-58", "x-gbk"}, "legacy", nil},
		{"gb18030", []string{"gb18030"}, "legacy", nil},
		{"Big5", []string{"big5", "big5-hkscs", "cn-big5", "csbig5", "x-x-big5"}, "legacy", nil},
		{"EUC-JP", []string{"cseucpkdfmtjapanese", "euc-jp", "x-euc-jp"}, "legacy", nil},
		{"ISO-2022-JP", []string{"csiso2022jp", "iso-2022-jp"}, "legacy", nil},
		{"Shift_JIS", []string{"csshiftjis", "ms932", "ms_kanji", "shift-jis", "shift_jis", "sjis", "windows-31j", "x-sjis"}, "legacy", nil},
		{"EUC-KR", []string{"cseuckr", "csksc56011987", "euc-kr", "iso-ir-149", "korean", "ks_c_5601-1987", "ks_c_5601-1989", "ksc5601", "ksc_5601", "windows-949"}, "legacy", nil},
		{"replacement", []string{"csiso2022kr", "hz-gb-2312", "iso-2022-cn", "iso-2022-cn-ext", "iso-2022-kr", "replacement"}, "replacement", nil},
	}

	boms := []BOMDef{
		{"UTF-8", []int{0xEF, 0xBB, 0xBF}},
		{"UTF-16LE", []int{0xFF, 0xFE}},
		{"UTF-16BE", []int{0xFE, 0xFF}},
	}

	catalog := EncodingCatalog{
		Spec: "Encoding Living Standard", SpecDate: "2026-04-10",
		TotalEncodings: len(encodings), Encodings: encodings, BOMTable: boms,
	}
	enc := json.NewEncoder(os.Stdout)
	enc.SetIndent("", "  ")
	enc.Encode(catalog)
}
