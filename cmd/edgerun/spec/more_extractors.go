package spec

import (
	"github.com/spf13/cobra"

	pb "edgerunrefcore/proto/go/spec"
)

// ---- Remaining Hardcoded Extractors ----

func runFetch(cmd *cobra.Command, args []string) error {
	cat := &pb.Catalog{SpecName: "Fetch Living Standard"}
	for _, e := range fetchEnums {
		cat.FetchEnums = append(cat.FetchEnums, &pb.FetchEnumDef{Name: e.name, Variants: e.variants})
	}
	return writeProto(cat)
}

type fetchEnum struct{ name string; variants []string }

var fetchEnums = []fetchEnum{
	{"RequestMode", []string{"same-origin", "no-cors", "cors", "navigate"}},
	{"RequestCredentials", []string{"omit", "same-origin", "include"}},
	{"RequestCache", []string{"default", "no-store", "reload", "no-cache", "force-cache", "only-if-cached"}},
	{"RequestRedirect", []string{"follow", "error", "manual"}},
	{"RequestPriority", []string{"high", "low", "auto"}},
	{"ResponseType", []string{"basic", "cors", "default", "error", "opaque", "opaqueredirect"}},
	{"ReferrerPolicy", []string{"", "no-referrer", "no-referrer-when-downgrade", "same-origin", "origin", "strict-origin", "origin-when-cross-origin", "strict-origin-when-cross-origin", "unsafe-url"}},
	{"RequestDestination", []string{"", "audio", "audioworklet", "document", "embed", "font", "frame", "iframe", "image", "manifest", "object", "paintworklet", "report", "script", "sharedworker", "style", "track", "video", "worker", "xslt"}},
}

func runURL(cmd *cobra.Command, args []string) error {
	cat := &pb.Catalog{SpecName: "URL Living Standard"}
	for _, s := range urlStates {
		cat.UrlStates = append(cat.UrlStates, &pb.UrlParserStateDef{Name: s})
	}
	return writeProto(cat)
}

var urlStates = []string{
	"scheme-start", "special-relative-or-authority", "path-or-authority",
	"authority", "host", "hostname", "port", "file", "file-slash",
	"file-host", "path-start", "path", "cannot-be-a-base-url-path",
	"query", "fragment", "relative", "relative-slash", "query-state",
}

func runUIEvents(cmd *cobra.Command, args []string) error {
	cat := &pb.Catalog{SpecName: "UI Events"}
	for _, k := range keyboardKeys {
		cat.KeyEvents = append(cat.KeyEvents, &pb.KeyEventDef{Key: k, Kind: "keyboard"})
	}
	for _, m := range mouseButtons {
		cat.KeyEvents = append(cat.KeyEvents, &pb.KeyEventDef{Key: m.name, Kind: "mouse"})
	}
	for _, mk := range modifierKeys {
		cat.KeyEvents = append(cat.KeyEvents, &pb.KeyEventDef{Key: mk, Kind: "modifier"})
	}
	return writeProto(cat)
}

var keyboardKeys = []string{
	"Enter", "Tab", " ", "Escape", "Backspace", "Delete", "Insert",
	"ArrowUp", "ArrowDown", "ArrowLeft", "ArrowRight",
	"Home", "End", "PageUp", "PageDown",
	"F1", "F2", "F3", "F4", "F5", "F6", "F7", "F8", "F9", "F10", "F11", "F12",
	"Shift", "Control", "Alt", "Meta", "CapsLock", "NumLock", "ScrollLock",
}

var mouseButtons = []struct{ name string }{
	{"Left"}, {"Middle"}, {"Right"}, {"Back"}, {"Forward"},
}

var modifierKeys = []string{"Alt", "Control", "Shift", "Meta"}

func runWebIDL(cmd *cobra.Command, args []string) error {
	cat := &pb.Catalog{SpecName: "Web IDL"}
	for _, t := range webidlPrimitiveTypes {
		cat.WebidlTypes = append(cat.WebidlTypes, &pb.WebIdlType{Name: t, Kind: pb.WebIdlTypeKind_WEBIDL_TYPE_PRIMITIVE})
	}
	for _, t := range webidlStringTypes {
		cat.WebidlTypes = append(cat.WebidlTypes, &pb.WebIdlType{Name: t, Kind: pb.WebIdlTypeKind_WEBIDL_TYPE_STRING})
	}
	for _, t := range webidlObjectTypes {
		cat.WebidlTypes = append(cat.WebidlTypes, &pb.WebIdlType{Name: t, Kind: pb.WebIdlTypeKind_WEBIDL_TYPE_OBJECT})
	}
	for _, t := range webidlSpecialTypes {
		cat.WebidlTypes = append(cat.WebidlTypes, &pb.WebIdlType{Name: t, Kind: pb.WebIdlTypeKind_WEBIDL_TYPE_SPECIAL})
	}
	return writeProto(cat)
}

var webidlPrimitiveTypes = []string{
	"byte", "octet", "short", "unsigned short", "long", "unsigned long",
	"long long", "unsigned long long", "float", "double", "unrestricted float",
	"unrestricted double", "boolean",
}

var webidlStringTypes = []string{"DOMString", "ByteString", "USVString"}

var webidlObjectTypes = []string{
	"object", "RegExp", "Error", "ArrayBuffer",
	"DataView", "Int8Array", "Uint8Array", "Uint8ClampedArray",
	"Int16Array", "Uint16Array", "Int32Array", "Uint32Array",
	"Float32Array", "Float64Array", "BigInt64Array", "BigUint64Array",
}

var webidlSpecialTypes = []string{
	"any", "bigint", "symbol", "Promise",
	"sequence", "record", "FrozenArray", "ObservableArray",
}

func runFlexbox(cmd *cobra.Command, args []string) error {
	cat := &pb.Catalog{SpecName: "CSS Flexible Box Layout"}
	for _, p := range flexboxProperties {
		cat.FlexboxProperties = append(cat.FlexboxProperties, &pb.FlexPropertyDef{
			Name: p.name, Values: p.values, Description: p.desc, Category: p.category,
		})
	}
	return writeProto(cat)
}

type flexEntry struct{ name string; values []string; desc, category string }

var flexboxProperties = []flexEntry{
	{"display", []string{"flex", "inline-flex"}, "Establishes flex container", "container"},
	{"flex-direction", []string{"row", "row-reverse", "column", "column-reverse"}, "Main axis direction", "container"},
	{"flex-wrap", []string{"nowrap", "wrap", "wrap-reverse"}, "Line wrapping behavior", "container"},
	{"flex-flow", []string{"<'flex-direction'>", "<'flex-wrap'>"}, "Shorthand for flex-direction and flex-wrap", "container"},
	{"justify-content", []string{"flex-start", "flex-end", "center", "space-between", "space-around", "space-evenly", "start", "end", "left", "right", "normal", "stretch"}, "Main-axis alignment", "container"},
	{"align-items", []string{"flex-start", "flex-end", "center", "baseline", "stretch", "start", "end", "self-start", "self-end", "normal"}, "Cross-axis alignment", "container"},
	{"align-content", []string{"flex-start", "flex-end", "center", "space-between", "space-around", "space-evenly", "stretch", "start", "end", "normal"}, "Line stacking alignment", "container"},
	{"gap", []string{"<'row-gap'>", "<'column-gap'>"}, "Gutters between flex items", "container"},
	{"flex", []string{"none", "<'flex-grow'> <'flex-shrink'>? <'flex-basis'>?"}, "Shorthand for flex-grow, flex-shrink, flex-basis", "child"},
	{"flex-grow", []string{"<number>"}, "Growth factor", "child"},
	{"flex-shrink", []string{"<number>"}, "Shrinkage factor", "child"},
	{"flex-basis", []string{"content", "<'width'>"}, "Initial main size", "child"},
	{"align-self", []string{"auto", "flex-start", "flex-end", "center", "baseline", "stretch", "start", "end", "self-start", "self-end", "normal"}, "Override align-items", "child"},
	{"order", []string{"<integer>"}, "Paint/display order", "child"},
}

func runGrid(cmd *cobra.Command, args []string) error {
	cat := &pb.Catalog{SpecName: "CSS Grid Layout"}
	for _, p := range gridProperties {
		cat.GridProperties = append(cat.GridProperties, &pb.GridPropertyDef{
			Name: p.name, Values: p.values, Description: p.desc,
		})
	}
	return writeProto(cat)
}

type gridEntry struct{ name string; values []string; desc string }

var gridProperties = []gridEntry{
	{"display", []string{"grid", "inline-grid"}, "Establishes grid container",},
	{"grid-template-columns", []string{"none", "<track-list>", "<auto-track-list>"}, "Column track sizing"},
	{"grid-template-rows", []string{"none", "<track-list>", "<auto-track-list>"}, "Row track sizing"},
	{"grid-template-areas", []string{"none", "<'grid-template-areas'>"}, "Named grid areas"},
	{"grid-template", []string{"none", "<'grid-template-rows'> [ / <'grid-template-columns'> ]?"}, "Shorthand for template"},
	{"grid-auto-columns", []string{"<track-size>"}, "Implicit column track size"},
	{"grid-auto-rows", []string{"<track-size>"}, "Implicit row track size"},
	{"grid-auto-flow", []string{"row", "column", "dense", "row dense", "column dense"}, "Auto-placement algorithm"},
	{"grid", []string{"<'grid-template'>", "<'grid-template-rows'> / [ auto-flow && dense? ] <'grid-auto-columns'>?"}, "Shorthand"},
	{"column-gap", []string{"normal", "<length-percentage>"}, "Column gutters"},
	{"row-gap", []string{"normal", "<length-percentage>"}, "Row gutters"},
	{"gap", []string{"<'row-gap'> <'column-gap'>?"}, "Shorthand for gutters"},
	{"grid-column-start", []string{"auto", "<custom-ident>", "<integer>", "span <custom-ident>", "span <integer>"}, "Column start line"},
	{"grid-column-end", []string{"auto", "<custom-ident>", "<integer>", "span <custom-ident>", "span <integer>"}, "Column end line"},
	{"grid-row-start", []string{"auto", "<custom-ident>", "<integer>", "span <custom-ident>", "span <integer>"}, "Row start line"},
	{"grid-row-end", []string{"auto", "<custom-ident>", "<integer>", "span <custom-ident>", "span <integer>"}, "Row end line"},
	{"grid-column", []string{"<'grid-column-start'> [ / <'grid-column-end'> ]?"}, "Shorthand for column placement"},
	{"grid-row", []string{"<'grid-row-start'> [ / <'grid-row-end'> ]?"}, "Shorthand for row placement"},
	{"justify-items", []string{"normal", "stretch", "center", "start", "end", "flex-start", "flex-end", "self-start", "self-end", "left", "right", "baseline", "first baseline", "last baseline", "safe", "unsafe"}, "Inline-axis alignment"},
	{"align-items", []string{"normal", "stretch", "center", "start", "end", "flex-start", "flex-end", "self-start", "self-end", "left", "right", "baseline", "first baseline", "last baseline", "safe", "unsafe"}, "Block-axis alignment"},
	{"place-items", []string{"<'align-items'> <'justify-items'>?"}, "Shorthand"},
	{"justify-content", []string{"normal", "stretch", "center", "start", "end", "flex-start", "flex-end", "left", "right", "space-between", "space-around", "space-evenly", "safe", "unsafe"}, "Track inline-axis alignment"},
	{"align-content", []string{"normal", "stretch", "center", "start", "end", "flex-start", "flex-end", "left", "right", "space-between", "space-around", "space-evenly", "safe", "unsafe"}, "Track block-axis alignment"},
	{"place-content", []string{"<'align-content'> <'justify-content'>?"}, "Shorthand"},
}

func runMedia(cmd *cobra.Command, args []string) error {
	cat := &pb.Catalog{SpecName: "CSS Media Queries"}
	for _, m := range mediaTypes {
		cat.MediaProperties = append(cat.MediaProperties, &pb.MediaPropertyDef{Name: "media-type:" + m})
	}
	for _, m := range mediaFeatures {
		cat.MediaProperties = append(cat.MediaProperties, &pb.MediaPropertyDef{
			Name: m.name, Value: m.value, Description: m.desc,
		})
	}
	return writeProto(cat)
}

type mediaEntry struct{ name, value, desc string }

var mediaTypes = []string{"all", "print", "screen"}

var mediaFeatures = []mediaEntry{
	{"width", "<length>", "Viewport width"},
	{"height", "<length>", "Viewport height"},
	{"aspect-ratio", "<ratio>", "Viewport aspect ratio"},
	{"orientation", "portrait | landscape", "Viewport orientation"},
	{"prefers-color-scheme", "light | dark | no-preference", "Color scheme preference"},
	{"prefers-reduced-motion", "no-preference | reduce", "Motion preference"},
	{"prefers-contrast", "no-preference | more | less | custom", "Contrast preference"},
	{"prefers-reduced-transparency", "no-preference | reduce", "Transparency preference"},
	{"forced-colors", "none | active", "Forced colors mode"},
	{"display-mode", "browser | minimal-ui | standalone | fullscreen", "Display mode"},
	{"resolution", "<resolution>", "Pixel density"},
	{"any-hover", "none | hover", "Hover capability of any pointing device"},
	{"any-pointer", "none | coarse | fine", "Pointer capability of any pointing device"},
	{"hover", "none | hover", "Hover capability of primary input"},
	{"pointer", "none | coarse | fine", "Pointer capability of primary input"},
	{"scripting", "none | initial-only | enabled", "Scripting capability"},
	{"update", "none | slow | fast", "Update frequency"},
	{"color", "<integer>", "Bits per color component"},
	{"color-index", "<integer>", "Entries in color lookup table"},
	{"monochrome", "<integer>", "Bits per pixel in monochrome frame buffer"},
}

func runSyntax(cmd *cobra.Command, args []string) error {
	cat := &pb.Catalog{SpecName: "CSS Syntax"}
	for _, t := range cssTokens {
		cat.SyntaxDefs = append(cat.SyntaxDefs, &pb.SyntaxDef{
			Name: t, Kind: "token",
		})
	}
	for _, s := range tokenizerStates {
		cat.SyntaxDefs = append(cat.SyntaxDefs, &pb.SyntaxDef{
			Name: s, Kind: "state",
		})
	}
	return writeProto(cat)
}

var cssTokens = []string{
	"ident-token", "function-token", "at-keyword-token",
	"hash-token", "string-token", "bad-string-token",
	"url-token", "bad-url-token", "delim-token",
	"number-token", "percentage-token", "dimension-token",
	"whitespace-token", "cdO-token", "cdc-token",
	"colon-token", "semicolon-token", "comma-token",
	"open-square-token", "close-square-token",
	"open-paren-token", "close-paren-token",
	"open-curly-token", "close-curly-token",
}

var tokenizerStates = []string{
	"top-level", "data", "comment-start", "comment-start-dash",
	"comment", "comment-end-dash", "comment-end",
	"comment-end-comment-length",
	"consume-ident-sequence", "consume-number", "consume-remaining-unicode-range",
	"consume-escaped-code-point",
}

func runColor(cmd *cobra.Command, args []string) error {
	cat := &pb.Catalog{SpecName: "CSS Color"}
	for _, c := range namedColors {
		cat.ColorDefs = append(cat.ColorDefs, &pb.ColorDef{
			Name: c.name, ColorSpace: "srgb",
			Components: c.components, Alpha: c.alpha,
		})
	}
	return writeProto(cat)
}

type colorEntry struct {
	name       string
	components []float64 // r, g, b in 0-1 range
	alpha      string
}

var namedColors = []colorEntry{
	{"aliceblue", []float64{0.941, 0.973, 1.0}, "1"},
	{"antiquewhite", []float64{0.98, 0.922, 0.843}, "1"},
	{"aqua", []float64{0, 1, 1}, "1"},
	{"aquamarine", []float64{0.498, 1, 0.831}, "1"},
	{"azure", []float64{0.941, 1, 1}, "1"},
	{"beige", []float64{0.961, 0.961, 0.863}, "1"},
	{"bisque", []float64{1, 0.894, 0.769}, "1"},
	{"black", []float64{0, 0, 0}, "1"},
	{"blanchedalmond", []float64{1, 0.922, 0.804}, "1"},
	{"blue", []float64{0, 0, 1}, "1"},
	{"blueviolet", []float64{0.541, 0.169, 0.886}, "1"},
	{"brown", []float64{0.647, 0.165, 0.165}, "1"},
	{"burlywood", []float64{0.871, 0.722, 0.529}, "1"},
	{"cadetblue", []float64{0.373, 0.62, 0.627}, "1"},
	{"chartreuse", []float64{0.498, 1, 0}, "1"},
	{"chocolate", []float64{0.824, 0.412, 0.118}, "1"},
	{"coral", []float64{1, 0.498, 0.314}, "1"},
	{"cornflowerblue", []float64{0.392, 0.584, 0.929}, "1"},
	{"cornsilk", []float64{1, 0.973, 0.863}, "1"},
	{"crimson", []float64{0.863, 0.078, 0.235}, "1"},
	{"cyan", []float64{0, 1, 1}, "1"},
	{"darkblue", []float64{0, 0, 0.545}, "1"},
	{"darkcyan", []float64{0, 0.545, 0.545}, "1"},
	{"darkgoldenrod", []float64{0.722, 0.525, 0.043}, "1"},
	{"darkgray", []float64{0.663, 0.663, 0.663}, "1"},
	{"darkgreen", []float64{0, 0.392, 0}, "1"},
	{"darkgrey", []float64{0.663, 0.663, 0.663}, "1"},
	{"darkkhaki", []float64{0.741, 0.718, 0.42}, "1"},
	{"darkmagenta", []float64{0.545, 0, 0.545}, "1"},
	{"darkolivegreen", []float64{0.333, 0.42, 0.184}, "1"},
	{"darkorange", []float64{1, 0.549, 0}, "1"},
	{"darkorchid", []float64{0.6, 0.196, 0.8}, "1"},
	{"darkred", []float64{0.545, 0, 0}, "1"},
	{"darksalmon", []float64{0.914, 0.588, 0.478}, "1"},
	{"darkseagreen", []float64{0.561, 0.737, 0.561}, "1"},
	{"darkslateblue", []float64{0.282, 0.239, 0.545}, "1"},
	{"darkslategray", []float64{0.184, 0.31, 0.31}, "1"},
	{"darkslategrey", []float64{0.184, 0.31, 0.31}, "1"},
	{"darkturquoise", []float64{0, 0.808, 0.82}, "1"},
	{"darkviolet", []float64{0.58, 0, 0.827}, "1"},
	{"deeppink", []float64{1, 0.078, 0.576}, "1"},
	{"deepskyblue", []float64{0, 0.749, 1}, "1"},
	{"dimgray", []float64{0.412, 0.412, 0.412}, "1"},
	{"dimgrey", []float64{0.412, 0.412, 0.412}, "1"},
	{"dodgerblue", []float64{0.118, 0.565, 1}, "1"},
	{"firebrick", []float64{0.698, 0.133, 0.133}, "1"},
	{"floralwhite", []float64{1, 0.98, 0.941}, "1"},
	{"forestgreen", []float64{0.133, 0.545, 0.133}, "1"},
	{"fuchsia", []float64{1, 0, 1}, "1"},
	{"gainsboro", []float64{0.863, 0.863, 0.863}, "1"},
	{"ghostwhite", []float64{0.973, 0.973, 1}, "1"},
	{"gold", []float64{1, 0.843, 0}, "1"},
	{"goldenrod", []float64{0.855, 0.647, 0.125}, "1"},
	{"gray", []float64{0.502, 0.502, 0.502}, "1"},
	{"green", []float64{0, 0.502, 0}, "1"},
	{"greenyellow", []float64{0.678, 1, 0.184}, "1"},
	{"grey", []float64{0.502, 0.502, 0.502}, "1"},
	{"honeydew", []float64{0.941, 1, 0.941}, "1"},
	{"hotpink", []float64{1, 0.412, 0.706}, "1"},
	{"indianred", []float64{0.804, 0.361, 0.361}, "1"},
	{"indigo", []float64{0.294, 0, 0.51}, "1"},
	{"ivory", []float64{1, 1, 0.941}, "1"},
	{"khaki", []float64{0.941, 0.902, 0.549}, "1"},
	{"lavender", []float64{0.902, 0.902, 0.98}, "1"},
	{"lavenderblush", []float64{1, 0.941, 0.961}, "1"},
	{"lawngreen", []float64{0.486, 0.988, 0}, "1"},
	{"lemonchiffon", []float64{1, 0.98, 0.804}, "1"},
	{"lightblue", []float64{0.678, 0.847, 0.902}, "1"},
	{"lightcoral", []float64{0.941, 0.502, 0.502}, "1"},
	{"lightcyan", []float64{0.878, 1, 1}, "1"},
	{"lightgoldenrodyellow", []float64{0.98, 0.98, 0.82}, "1"},
	{"lightgray", []float64{0.827, 0.827, 0.827}, "1"},
	{"lightgreen", []float64{0.565, 0.933, 0.565}, "1"},
	{"lightgrey", []float64{0.827, 0.827, 0.827}, "1"},
	{"lightpink", []float64{1, 0.714, 0.757}, "1"},
	{"lightsalmon", []float64{1, 0.627, 0.478}, "1"},
	{"lightseagreen", []float64{0.125, 0.698, 0.667}, "1"},
	{"lightskyblue", []float64{0.529, 0.808, 0.98}, "1"},
	{"lightslategray", []float64{0.467, 0.533, 0.6}, "1"},
	{"lightslategrey", []float64{0.467, 0.533, 0.6}, "1"},
	{"lightsteelblue", []float64{0.69, 0.769, 0.871}, "1"},
	{"lightyellow", []float64{1, 1, 0.878}, "1"},
	{"lime", []float64{0, 1, 0}, "1"},
	{"limegreen", []float64{0.196, 0.804, 0.196}, "1"},
	{"linen", []float64{0.98, 0.941, 0.902}, "1"},
	{"magenta", []float64{1, 0, 1}, "1"},
	{"maroon", []float64{0.502, 0, 0}, "1"},
	{"mediumaquamarine", []float64{0.4, 0.804, 0.667}, "1"},
	{"mediumblue", []float64{0, 0, 0.804}, "1"},
	{"mediumorchid", []float64{0.729, 0.333, 0.827}, "1"},
	{"mediumpurple", []float64{0.576, 0.439, 0.859}, "1"},
	{"mediumseagreen", []float64{0.235, 0.702, 0.443}, "1"},
	{"mediumslateblue", []float64{0.482, 0.408, 0.933}, "1"},
	{"mediumspringgreen", []float64{0, 0.98, 0.604}, "1"},
	{"mediumturquoise", []float64{0.282, 0.82, 0.8}, "1"},
	{"mediumvioletred", []float64{0.78, 0.082, 0.522}, "1"},
	{"midnightblue", []float64{0.098, 0.098, 0.439}, "1"},
	{"mintcream", []float64{0.961, 1, 0.98}, "1"},
	{"mistyrose", []float64{1, 0.894, 0.882}, "1"},
	{"moccasin", []float64{1, 0.894, 0.71}, "1"},
	{"navajowhite", []float64{1, 0.871, 0.678}, "1"},
	{"navy", []float64{0, 0, 0.502}, "1"},
	{"oldlace", []float64{0.992, 0.961, 0.902}, "1"},
	{"olive", []float64{0.502, 0.502, 0}, "1"},
	{"olivedrab", []float64{0.42, 0.557, 0.137}, "1"},
	{"orange", []float64{1, 0.647, 0}, "1"},
	{"orangered", []float64{1, 0.271, 0}, "1"},
	{"orchid", []float64{0.855, 0.439, 0.839}, "1"},
	{"palegoldenrod", []float64{0.933, 0.91, 0.667}, "1"},
	{"palegreen", []float64{0.596, 0.984, 0.596}, "1"},
	{"paleturquoise", []float64{0.686, 0.933, 0.933}, "1"},
	{"palevioletred", []float64{0.859, 0.439, 0.576}, "1"},
	{"papayawhip", []float64{1, 0.937, 0.835}, "1"},
	{"peachpuff", []float64{1, 0.855, 0.725}, "1"},
	{"peru", []float64{0.804, 0.522, 0.247}, "1"},
	{"pink", []float64{1, 0.753, 0.796}, "1"},
	{"plum", []float64{0.867, 0.627, 0.867}, "1"},
	{"powderblue", []float64{0.69, 0.878, 0.902}, "1"},
	{"purple", []float64{0.502, 0, 0.502}, "1"},
	{"rebeccapurple", []float64{0.4, 0.2, 0.6}, "1"},
	{"red", []float64{1, 0, 0}, "1"},
	{"rosybrown", []float64{0.737, 0.561, 0.561}, "1"},
	{"royalblue", []float64{0.255, 0.412, 0.882}, "1"},
	{"saddlebrown", []float64{0.545, 0.271, 0.075}, "1"},
	{"salmon", []float64{0.98, 0.502, 0.447}, "1"},
	{"sandybrown", []float64{0.957, 0.643, 0.376}, "1"},
	{"seagreen", []float64{0.18, 0.545, 0.341}, "1"},
	{"seashell", []float64{1, 0.961, 0.933}, "1"},
	{"sienna", []float64{0.627, 0.322, 0.176}, "1"},
	{"silver", []float64{0.753, 0.753, 0.753}, "1"},
	{"skyblue", []float64{0.529, 0.808, 0.922}, "1"},
	{"slateblue", []float64{0.416, 0.353, 0.804}, "1"},
	{"slategray", []float64{0.439, 0.502, 0.565}, "1"},
	{"slategrey", []float64{0.439, 0.502, 0.565}, "1"},
	{"snow", []float64{1, 0.98, 0.98}, "1"},
	{"springgreen", []float64{0, 1, 0.498}, "1"},
	{"steelblue", []float64{0.275, 0.51, 0.706}, "1"},
	{"tan", []float64{0.824, 0.706, 0.549}, "1"},
	{"teal", []float64{0, 0.502, 0.502}, "1"},
	{"thistle", []float64{0.847, 0.749, 0.847}, "1"},
	{"tomato", []float64{1, 0.388, 0.278}, "1"},
	{"turquoise", []float64{0.251, 0.878, 0.816}, "1"},
	{"violet", []float64{0.933, 0.51, 0.933}, "1"},
	{"wheat", []float64{0.961, 0.871, 0.702}, "1"},
	{"white", []float64{1, 1, 1}, "1"},
	{"whitesmoke", []float64{0.961, 0.961, 0.961}, "1"},
	{"yellow", []float64{1, 1, 0}, "1"},
	{"yellowgreen", []float64{0.604, 0.804, 0.196}, "1"},
}
