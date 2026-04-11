// Package parserir encodes WHATWG HTML §13.2.5 as proto IR data.
package parserir

import "edgerun-reference-core/gen/go/edgerun/v0/html"

// Char class aliases
const (
	ccEOF     = html.CharClass_CHAR_CLASS_EOF
	ccTab     = html.CharClass_CHAR_CLASS_TAB
	ccLF      = html.CharClass_CHAR_CLASS_LF
	ccFF      = html.CharClass_CHAR_CLASS_FF
	ccCR      = html.CharClass_CHAR_CLASS_CR
	ccSpace   = html.CharClass_CHAR_CLASS_SPACE
	ccQuot    = html.CharClass_CHAR_CLASS_QUOT
	ccAmp     = html.CharClass_CHAR_CLASS_AMP
	ccApos    = html.CharClass_CHAR_CLASS_APOS
	ccHyphen  = html.CharClass_CHAR_CLASS_HYPHEN
	ccSlash   = html.CharClass_CHAR_CLASS_SLASH
	ccDigit   = html.CharClass_CHAR_CLASS_DIGIT
	ccUpper   = html.CharClass_CHAR_CLASS_UPPER_ALPHA
	ccLower   = html.CharClass_CHAR_CLASS_LOWER_ALPHA
	ccGT      = html.CharClass_CHAR_CLASS_GT
	ccEQ      = html.CharClass_CHAR_CLASS_EQ
	ccLT      = html.CharClass_CHAR_CLASS_LT
	ccQuest   = html.CharClass_CHAR_CLASS_QUESTION
	ccNull    = html.CharClass_CHAR_CLASS_NULL
	ccSemi    = html.CharClass_CHAR_CLASS_SEMICOLON
	ccBang    = html.CharClass_CHAR_CLASS_EXCLAMATION
	ccSqClose = html.CharClass_CHAR_CLASS_SQUARE_CLOSE
	ccPct     = html.CharClass_CHAR_CLASS_PERCENT
	ccOther   = html.CharClass_CHAR_CLASS_OTHER
)

// State aliases
const (
	stData                    = html.TokenizerState_DATA_STATE
	stRCDATA                  = html.TokenizerState_RCDATA_STATE
	stRAWTEXT                 = html.TokenizerState_RAWTEXT_STATE
	stScriptData              = html.TokenizerState_SCRIPT_DATA_STATE
	stPlaintext               = html.TokenizerState_PLAINTEXT_STATE
	stTagOpen                 = html.TokenizerState_TAG_OPEN_STATE
	stEndTagOpen              = html.TokenizerState_END_TAG_OPEN_STATE
	stTagName                 = html.TokenizerState_TAG_NAME_STATE
	stRCDATALessThan          = html.TokenizerState_RCDATA_LESS_THAN_SIGN_STATE
	stRCDATAEndTagOpen        = html.TokenizerState_RCDATA_END_TAG_OPEN_STATE
	stRCDATAEndTagName        = html.TokenizerState_RCDATA_END_TAG_NAME_STATE
	stRCDATAEndTagNameAfter   = html.TokenizerState_RCDATA_END_TAG_NAME_STATE_AFTER
	stRAWTEXTLessThan         = html.TokenizerState_RAWTEXT_LESS_THAN_SIGN_STATE
	stRAWTEXTEndTagOpen       = html.TokenizerState_RAWTEXT_END_TAG_OPEN_STATE
	stRAWTEXTEndTagName       = html.TokenizerState_RAWTEXT_END_TAG_NAME_STATE
	stRAWTEXTEndTagNameAfter  = html.TokenizerState_RAWTEXT_END_TAG_NAME_STATE_AFTER
	stScriptDataLessThan      = html.TokenizerState_SCRIPT_DATA_LESS_THAN_SIGN_STATE
	stScriptDataEndTagOpen    = html.TokenizerState_SCRIPT_DATA_END_TAG_OPEN_STATE
	stScriptDataEndTagName    = html.TokenizerState_SCRIPT_DATA_END_TAG_NAME_STATE
	stScriptDataEndTagNameAfter = html.TokenizerState_SCRIPT_DATA_END_TAG_NAME_STATE_AFTER
	stScriptDataEscapeStart   = html.TokenizerState_SCRIPT_DATA_ESCAPE_START_STATE
	stScriptDataEscapeStartDash = html.TokenizerState_SCRIPT_DATA_ESCAPE_START_DASH_STATE
	stScriptDataEscaped       = html.TokenizerState_SCRIPT_DATA_ESCAPED_STATE
	stScriptDataEscapedDash   = html.TokenizerState_SCRIPT_DATA_ESCAPED_DASH_STATE
	stScriptDataEscapedDashDash = html.TokenizerState_SCRIPT_DATA_ESCAPED_DASH_DASH_STATE
	stScriptDataEscapedLessThan = html.TokenizerState_SCRIPT_DATA_ESCAPED_LESS_THAN_SIGN_STATE
	stScriptDataEscapedEndTagOpen = html.TokenizerState_SCRIPT_DATA_ESCAPED_END_TAG_OPEN_STATE
	stScriptDataEscapedEndTagName = html.TokenizerState_SCRIPT_DATA_ESCAPED_END_TAG_NAME_STATE
	stScriptDataEscapedEndTagNameAfter = html.TokenizerState_SCRIPT_DATA_ESCAPED_END_TAG_NAME_STATE_AFTER
	stScriptDataDoubleEscapeStart = html.TokenizerState_SCRIPT_DATA_DOUBLE_ESCAPE_START_STATE
	stScriptDataDoubleEscaped = html.TokenizerState_SCRIPT_DATA_DOUBLE_ESCAPED_STATE
	stScriptDataDoubleEscapedDash = html.TokenizerState_SCRIPT_DATA_DOUBLE_ESCAPED_DASH_STATE
	stScriptDataDoubleEscapedDashDash = html.TokenizerState_SCRIPT_DATA_DOUBLE_ESCAPED_DASH_DASH_STATE
	stScriptDataDoubleEscapedLessThan = html.TokenizerState_SCRIPT_DATA_DOUBLE_ESCAPED_LESS_THAN_SIGN_STATE
	stScriptDataDoubleEscapeEnd = html.TokenizerState_SCRIPT_DATA_DOUBLE_ESCAPE_END_STATE
	stBeforeTagName             = html.TokenizerState_BEFORE_TAG_NAME_STATE
	stAfterTagName              = html.TokenizerState_AFTER_TAG_NAME_STATE
	stSelfClosingStartTag       = html.TokenizerState_SELF_CLOSING_START_TAG_STATE
	stBeforeAttrName            = html.TokenizerState_BEFORE_ATTRIBUTE_NAME_STATE
	stAttrName                  = html.TokenizerState_ATTRIBUTE_NAME_STATE
	stAfterAttrName             = html.TokenizerState_AFTER_ATTRIBUTE_NAME_STATE
	stBeforeAttrValue           = html.TokenizerState_BEFORE_ATTRIBUTE_VALUE_STATE
	stAttrValueDoubleQuoted     = html.TokenizerState_ATTRIBUTE_VALUE_DOUBLE_QUOTED_STATE
	stAttrValueSingleQuoted     = html.TokenizerState_ATTRIBUTE_VALUE_SINGLE_QUOTED_STATE
	stAttrValueUnquoted         = html.TokenizerState_ATTRIBUTE_VALUE_UNQUOTED_STATE
	stAfterAttrValueQuoted      = html.TokenizerState_AFTER_ATTRIBUTE_VALUE_QUOTED_STATE
	stCharRef                   = html.TokenizerState_CHARACTER_REFERENCE_STATE
	stNamedCharRef              = html.TokenizerState_NAMED_CHARACTER_REFERENCE_STATE
	stAmbiguousAmpersand        = html.TokenizerState_AMBIGUOUS_AMPERSAND_STATE
	stNumericCharRef            = html.TokenizerState_NUMERIC_CHARACTER_REFERENCE_STATE
	stHexCharRef                = html.TokenizerState_HEXADEMICAL_CHARACTER_REFERENCE_STATE
	stDecCharRef                = html.TokenizerState_DECIMAL_CHARACTER_REFERENCE_STATE
	stNumericCharRefEnd         = html.TokenizerState_NUMERIC_CHARACTER_REFERENCE_END_STATE
	stDoctype                   = html.TokenizerState_DOCTYPE_STATE
	stBeforeDoctypeName         = html.TokenizerState_BEFORE_DOCTYPE_NAME_STATE
	stDoctypeName               = html.TokenizerState_DOCTYPE_NAME_STATE
	stAfterDoctypeName          = html.TokenizerState_AFTER_DOCTYPE_NAME_STATE
	stAfterDoctypePublicKeyword = html.TokenizerState_AFTER_DOCTYPE_PUBLIC_KEYWORD_STATE
	stBeforeDoctypePublicID     = html.TokenizerState_BEFORE_DOCTYPE_PUBLIC_IDENTIFIER_STATE
	stDoctypePublicIDDouble     = html.TokenizerState_DOCTYPE_PUBLIC_IDENTIFIER_DOUBLE_QUOTED_STATE
	stDoctypePublicIDSingle     = html.TokenizerState_DOCTYPE_PUBLIC_IDENTIFIER_SINGLE_QUOTED_STATE
	stAfterDoctypePublicID      = html.TokenizerState_AFTER_DOCTYPE_PUBLIC_IDENTIFIER_STATE
	stBetweenDoctypePublicSystem = html.TokenizerState_BETWEEN_DOCTYPE_PUBLIC_AND_SYSTEM_IDENTIFIERS_STATE
	stAfterDoctypeSystemKeyword = html.TokenizerState_AFTER_DOCTYPE_SYSTEM_KEYWORD_STATE
	stBeforeDoctypeSystemID     = html.TokenizerState_BEFORE_DOCTYPE_SYSTEM_IDENTIFIER_STATE
	stDoctypeSystemIDDouble     = html.TokenizerState_DOCTYPE_SYSTEM_IDENTIFIER_DOUBLE_QUOTED_STATE
	stDoctypeSystemIDSingle     = html.TokenizerState_DOCTYPE_SYSTEM_IDENTIFIER_SINGLE_QUOTED_STATE
	stBogusDoctype              = html.TokenizerState_BOGUS_DOCTYPE_STATE
	stMarkupDeclOpen            = html.TokenizerState_MARKUP_DECLARATION_OPEN_STATE
	stCommentStart              = html.TokenizerState_COMMENT_START_STATE
	stCommentStartDash          = html.TokenizerState_COMMENT_START_DASH_STATE
	stComment                   = html.TokenizerState_COMMENT_STATE
	stCommentLessThanSign       = html.TokenizerState_COMMENT_LESS_THAN_SIGN_STATE
	stCommentLessThanSignBang   = html.TokenizerState_COMMENT_LESS_THAN_SIGN_BANG_STATE
	stCommentLessThanSignBangDash = html.TokenizerState_COMMENT_LESS_THAN_SIGN_BANG_DASH_STATE
	stCommentLessThanSignBangDashDash = html.TokenizerState_COMMENT_LESS_THAN_SIGN_BANG_DASH_DASH_STATE
	stCommentEndDash            = html.TokenizerState_COMMENT_END_DASH_STATE
	stCommentEnd                = html.TokenizerState_COMMENT_END_STATE
	stCommentEndBang            = html.TokenizerState_COMMENT_END_BANG_STATE
	stBogusComment              = html.TokenizerState_BOGUS_COMMENT_STATE
	stCDATA                     = html.TokenizerState_CDATA_SECTION_STATE
	stCDATABracket              = html.TokenizerState_CDATA_SECTION_BRACKET_STATE
	stCDATAEnd                  = html.TokenizerState_CDATA_SECTION_END_STATE
	stEOF                       = html.TokenizerState_EOF_STATE
)

const (
	ttStartTag      = html.TokenType_TOKEN_TYPE_START_TAG
	initial         = html.InsertionMode_INITIAL_MODE
	beforeHTML      = html.InsertionMode_BEFORE_HTML_MODE
	beforeHead      = html.InsertionMode_BEFORE_HEAD_MODE
	inHead          = html.InsertionMode_IN_HEAD_MODE
	afterHead       = html.InsertionMode_AFTER_HEAD_MODE
	inBody          = html.InsertionMode_IN_BODY_MODE
	afterBody       = html.InsertionMode_AFTER_BODY_MODE
	textMode        = html.InsertionMode_TEXT_MODE
	inTable         = html.InsertionMode_IN_TABLE_MODE
	inTableText     = html.InsertionMode_IN_TABLE_TEXT_MODE
	inTableBody     = html.InsertionMode_IN_TABLE_BODY_MODE
	inRow           = html.InsertionMode_IN_ROW_MODE
	inCell          = html.InsertionMode_IN_CELL_MODE
	inCaption       = html.InsertionMode_IN_CAPTION_MODE
	inColumnGroup   = html.InsertionMode_IN_COLUMN_GROUP_MODE
	inFrameset      = html.InsertionMode_IN_FRAMESET_MODE
	afterFrameset   = html.InsertionMode_AFTER_FRAMESET_MODE
	afterAfterBody  = html.InsertionMode_AFTER_AFTER_BODY_MODE
	afterAfterFS    = html.InsertionMode_AFTER_AFTER_FRAMESET_MODE
	inHeadNoscript  = html.InsertionMode_IN_HEAD_NOSCRIPT_MODE
	inSelect        = html.InsertionMode_IN_SELECT_MODE
	inSelectInTable = html.InsertionMode_IN_SELECT_IN_TABLE_MODE
	inTemplate      = html.InsertionMode_IN_TEMPLATE_MODE
)

var (
	everythingElse = []html.CharClass{ccTab, ccLF, ccFF, ccCR, ccSpace, ccQuot, ccApos, ccHyphen, ccSlash, ccDigit, ccUpper, ccLower, ccGT, ccEQ, ccQuest, ccSemi, ccBang, ccSqClose, ccPct, ccOther}
)
