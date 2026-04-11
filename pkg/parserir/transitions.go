package parserir

import "edgerun-reference-core/gen/go/edgerun/v0/html"

// TokenizerTransitions builds the complete state machine for WHATWG §13.2.5.
func TokenizerTransitions() []*html.StateTransition {
	var ts []*html.StateTransition

	// ── DATA (§13.2.5.1) ──
	ts = append(ts, t(stData, ccLT, stTagOpen, "13.2.5.1"))
	ts = append(ts, t(stData, ccAmp, stCharRef, "13.2.5.1", aSetReturnState(stData)))
	ts = append(ts, t(stData, ccNull, stData, "13.2.5.1", aErrorNull(), aEmitReplacement()))
	ts = append(ts, t(stData, ccEOF, stEOF, "13.2.5.1", aEmitEof()))
	ts = append(ts, tList(stData, everythingElse, stData, "13.2.5.1", aEmitChar())...)

	// ── RCDATA (§13.2.5.2) ──
	ts = append(ts, t(stRCDATA, ccAmp, stCharRef, "13.2.5.2", aSetReturnState(stRCDATA)))
	ts = append(ts, t(stRCDATA, ccLT, stRCDATALessThan, "13.2.5.2"))
	ts = append(ts, t(stRCDATA, ccNull, stRCDATA, "13.2.5.2", aErrorNull(), aEmitReplacement()))
	ts = append(ts, t(stRCDATA, ccEOF, stEOF, "13.2.5.2", aEmitEof()))
	ts = append(ts, tList(stRCDATA, everythingElse, stRCDATA, "13.2.5.2", aEmitChar())...)

	// ── RCDATA LESS-THAN SIGN (§13.2.5.10) ──
	ts = append(ts, t(stRCDATALessThan, ccSlash, stRCDATAEndTagOpen, "13.2.5.10", aAppendTempBuffer()))
	ts = append(ts, t(stRCDATALessThan, ccEOF, stRCDATA, "13.2.5.10", aReconsume(), aEmitChar()))
	ts = append(ts, t(stRCDATALessThan, ccOther, stRCDATA, "13.2.5.10", aReconsume(), aEmitChar()))

	// ── RCDATA END TAG OPEN (§13.2.5.11) ──
	ts = append(ts, t(stRCDATAEndTagOpen, ccLower, stRCDATAEndTagName, "13.2.5.11", aEndTag(), aAppendTagName(), aAppendTempBuffer()))
	ts = append(ts, t(stRCDATAEndTagOpen, ccUpper, stRCDATAEndTagName, "13.2.5.11", aEndTag(), aAppendTagName(), aAppendTempBuffer()))
	ts = append(ts, t(stRCDATAEndTagOpen, ccEOF, stRCDATA, "13.2.5.11", aReconsume(), aEmitChar()))
	ts = append(ts, t(stRCDATAEndTagOpen, ccOther, stRCDATA, "13.2.5.11", aReconsume(), aEmitChar()))

	// ── RCDATA END TAG NAME (§13.2.5.12) ──
	ts = append(ts, tList(stRCDATAEndTagName, whitespace(), stRCDATAEndTagNameAfter, "13.2.5.12", aAppendTempBuffer(), aCheckAppropriateEndTag())...)
	ts = append(ts, t(stRCDATAEndTagName, ccSlash, stRCDATAEndTagNameAfter, "13.2.5.12", aAppendTempBuffer(), aCheckAppropriateEndTag()))
	ts = append(ts, t(stRCDATAEndTagName, ccGT, stRCDATA, "13.2.5.12", aCheckAppropriateEndTag()))
	ts = append(ts, tList(stRCDATAEndTagName, []html.CharClass{ccLower, ccUpper}, stRCDATAEndTagName, "13.2.5.12", aAppendTagName(), aAppendTempBuffer())...)
	ts = append(ts, t(stRCDATAEndTagName, ccEOF, stRCDATA, "13.2.5.12", aReconsume(), aEmitChar()))
	ts = append(ts, t(stRCDATAEndTagName, ccOther, stRCDATA, "13.2.5.12", aReconsume(), aEmitChar()))

	// ── RCDATA END TAG NAME AFTER (§13.2.5.12a) ──
	ts = append(ts, t(stRCDATAEndTagNameAfter, ccSlash, stBeforeAttrName, "13.2.5.12a", aAppendTempBuffer()))
	ts = append(ts, t(stRCDATAEndTagNameAfter, ccGT, stRCDATA, "13.2.5.12a", aEmitToken(ttStartTag), aAppendTempBuffer()))
	ts = append(ts, tList(stRCDATAEndTagNameAfter, append(whitespace(), ccLower, ccUpper), stBeforeAttrName, "13.2.5.12a", aAppendTempBuffer())...)
	ts = append(ts, t(stRCDATAEndTagNameAfter, ccEOF, stRCDATA, "13.2.5.12a", aReconsume(), aEmitChar()))
	ts = append(ts, t(stRCDATAEndTagNameAfter, ccOther, stRCDATA, "13.2.5.12a", aReconsume(), aEmitChar()))

	// ── RAWTEXT (§13.2.5.3) ──
	ts = append(ts, t(stRAWTEXT, ccLT, stRAWTEXTLessThan, "13.2.5.3"))
	ts = append(ts, t(stRAWTEXT, ccNull, stRAWTEXT, "13.2.5.3", aErrorNull(), aEmitReplacement()))
	ts = append(ts, t(stRAWTEXT, ccEOF, stEOF, "13.2.5.3", aEmitEof()))
	rawtextChars := append([]html.CharClass{ccAmp}, everythingElse...)
	ts = append(ts, tList(stRAWTEXT, rawtextChars, stRAWTEXT, "13.2.5.3", aEmitChar())...)

	// ── RAWTEXT LESS-THAN SIGN (§13.2.5.13) ──
	ts = append(ts, t(stRAWTEXTLessThan, ccSlash, stRAWTEXTEndTagOpen, "13.2.5.13", aAppendTempBuffer()))
	ts = append(ts, t(stRAWTEXTLessThan, ccEOF, stRAWTEXT, "13.2.5.13", aReconsume(), aEmitChar()))
	ts = append(ts, t(stRAWTEXTLessThan, ccOther, stRAWTEXT, "13.2.5.13", aReconsume(), aEmitChar()))

	// ── RAWTEXT END TAG OPEN (§13.2.5.14) ──
	ts = append(ts, t(stRAWTEXTEndTagOpen, ccLower, stRAWTEXTEndTagName, "13.2.5.14", aEndTag(), aAppendTagName(), aAppendTempBuffer()))
	ts = append(ts, t(stRAWTEXTEndTagOpen, ccUpper, stRAWTEXTEndTagName, "13.2.5.14", aEndTag(), aAppendTagName(), aAppendTempBuffer()))
	ts = append(ts, t(stRAWTEXTEndTagOpen, ccEOF, stRAWTEXT, "13.2.5.14", aReconsume(), aEmitChar()))
	ts = append(ts, t(stRAWTEXTEndTagOpen, ccOther, stRAWTEXT, "13.2.5.14", aReconsume(), aEmitChar()))

	// ── RAWTEXT END TAG NAME (§13.2.5.15) ──
	ts = append(ts, tList(stRAWTEXTEndTagName, whitespace(), stRAWTEXTEndTagNameAfter, "13.2.5.15", aAppendTempBuffer(), aCheckAppropriateEndTag())...)
	ts = append(ts, t(stRAWTEXTEndTagName, ccSlash, stRAWTEXTEndTagNameAfter, "13.2.5.15", aAppendTempBuffer(), aCheckAppropriateEndTag()))
	ts = append(ts, t(stRAWTEXTEndTagName, ccGT, stRAWTEXT, "13.2.5.15", aCheckAppropriateEndTag()))
	ts = append(ts, tList(stRAWTEXTEndTagName, []html.CharClass{ccLower, ccUpper}, stRAWTEXTEndTagName, "13.2.5.15", aAppendTagName(), aAppendTempBuffer())...)
	ts = append(ts, t(stRAWTEXTEndTagName, ccEOF, stRAWTEXT, "13.2.5.15", aReconsume(), aEmitChar()))
	ts = append(ts, t(stRAWTEXTEndTagName, ccOther, stRAWTEXT, "13.2.5.15", aReconsume(), aEmitChar()))

	// ── RAWTEXT END TAG NAME AFTER (§13.2.5.15a) ──
	ts = append(ts, t(stRAWTEXTEndTagNameAfter, ccSlash, stBeforeAttrName, "13.2.5.15a", aAppendTempBuffer()))
	ts = append(ts, t(stRAWTEXTEndTagNameAfter, ccGT, stRAWTEXT, "13.2.5.15a", aEmitToken(ttStartTag), aAppendTempBuffer()))
	ts = append(ts, tList(stRAWTEXTEndTagNameAfter, append(whitespace(), ccLower, ccUpper), stBeforeAttrName, "13.2.5.15a", aAppendTempBuffer())...)
	ts = append(ts, t(stRAWTEXTEndTagNameAfter, ccEOF, stRAWTEXT, "13.2.5.15a", aReconsume(), aEmitChar()))
	ts = append(ts, t(stRAWTEXTEndTagNameAfter, ccOther, stRAWTEXT, "13.2.5.15a", aReconsume(), aEmitChar()))

	// ── SCRIPT DATA (§13.2.5.4) ──
	ts = append(ts, t(stScriptData, ccLT, stScriptDataLessThan, "13.2.5.4"))
	ts = append(ts, t(stScriptData, ccNull, stScriptData, "13.2.5.4", aErrorNull(), aEmitReplacement()))
	ts = append(ts, t(stScriptData, ccEOF, stEOF, "13.2.5.4", aEmitEof()))
	ts = append(ts, tList(stScriptData, everythingElse, stScriptData, "13.2.5.4", aEmitChar())...)

	// ── SCRIPT DATA LESS-THAN SIGN (§13.2.5.17) ──
	ts = append(ts, t(stScriptDataLessThan, ccSlash, stScriptDataEndTagOpen, "13.2.5.17", aAppendTempBuffer()))
	ts = append(ts, t(stScriptDataLessThan, ccBang, stScriptDataEscapeStart, "13.2.5.17", aAppendTempBuffer()))
	ts = append(ts, t(stScriptDataLessThan, ccEOF, stScriptData, "13.2.5.17", aReconsume(), aEmitChar()))
	ts = append(ts, t(stScriptDataLessThan, ccOther, stScriptData, "13.2.5.17", aReconsume(), aEmitChar()))

	// ── SCRIPT DATA END TAG OPEN (§13.2.5.18) ──
	ts = append(ts, t(stScriptDataEndTagOpen, ccLower, stScriptDataEndTagName, "13.2.5.18", aEndTag(), aAppendTagName(), aAppendTempBuffer()))
	ts = append(ts, t(stScriptDataEndTagOpen, ccUpper, stScriptDataEndTagName, "13.2.5.18", aEndTag(), aAppendTagName(), aAppendTempBuffer()))
	ts = append(ts, t(stScriptDataEndTagOpen, ccEOF, stScriptData, "13.2.5.18", aReconsume(), aEmitChar()))
	ts = append(ts, t(stScriptDataEndTagOpen, ccOther, stScriptData, "13.2.5.18", aReconsume(), aEmitChar()))

	// ── SCRIPT DATA END TAG NAME (§13.2.5.19) ──
	ts = append(ts, tList(stScriptDataEndTagName, whitespace(), stScriptDataEndTagNameAfter, "13.2.5.19", aAppendTempBuffer(), aCheckAppropriateEndTag())...)
	ts = append(ts, t(stScriptDataEndTagName, ccSlash, stScriptDataEndTagNameAfter, "13.2.5.19", aAppendTempBuffer(), aCheckAppropriateEndTag()))
	ts = append(ts, t(stScriptDataEndTagName, ccGT, stScriptData, "13.2.5.19", aCheckAppropriateEndTag()))
	ts = append(ts, tList(stScriptDataEndTagName, []html.CharClass{ccLower, ccUpper}, stScriptDataEndTagName, "13.2.5.19", aAppendTagName(), aAppendTempBuffer())...)
	ts = append(ts, t(stScriptDataEndTagName, ccEOF, stScriptData, "13.2.5.19", aReconsume(), aEmitChar()))
	ts = append(ts, t(stScriptDataEndTagName, ccOther, stScriptData, "13.2.5.19", aReconsume(), aEmitChar()))

	// ── SCRIPT DATA END TAG NAME AFTER (§13.2.5.19a) ──
	ts = append(ts, t(stScriptDataEndTagNameAfter, ccSlash, stBeforeAttrName, "13.2.5.19a", aAppendTempBuffer()))
	ts = append(ts, t(stScriptDataEndTagNameAfter, ccGT, stScriptData, "13.2.5.19a", aEmitToken(ttStartTag), aAppendTempBuffer()))
	ts = append(ts, tList(stScriptDataEndTagNameAfter, append(whitespace(), ccLower, ccUpper), stBeforeAttrName, "13.2.5.19a", aAppendTempBuffer())...)
	ts = append(ts, t(stScriptDataEndTagNameAfter, ccEOF, stScriptData, "13.2.5.19a", aReconsume(), aEmitChar()))
	ts = append(ts, t(stScriptDataEndTagNameAfter, ccOther, stScriptData, "13.2.5.19a", aReconsume(), aEmitChar()))

	// ── SCRIPT DATA ESCAPE START (§13.2.5.21) ──
	ts = append(ts, t(stScriptDataEscapeStart, ccHyphen, stScriptDataEscapeStartDash, "13.2.5.21", aEmitChar(), aAppendTempBuffer()))
	ts = append(ts, t(stScriptDataEscapeStart, ccLT, stScriptDataEscaped, "13.2.5.21", aEmitChar(), aAppendTempBuffer()))
	ts = append(ts, t(stScriptDataEscapeStart, ccEOF, stScriptData, "13.2.5.21", aReconsume()))
	ts = append(ts, t(stScriptDataEscapeStart, ccOther, stScriptData, "13.2.5.21", aReconsume(), aEmitChar()))

	// ── SCRIPT DATA ESCAPE START DASH (§13.2.5.22) ──
	ts = append(ts, t(stScriptDataEscapeStartDash, ccHyphen, stScriptDataEscapedDashDash, "13.2.5.22", aEmitChar(), aAppendTempBuffer()))
	ts = append(ts, t(stScriptDataEscapeStartDash, ccLT, stScriptDataEscaped, "13.2.5.22", aEmitChar(), aAppendTempBuffer()))
	ts = append(ts, t(stScriptDataEscapeStartDash, ccEOF, stScriptData, "13.2.5.22", aReconsume()))
	ts = append(ts, t(stScriptDataEscapeStartDash, ccOther, stScriptData, "13.2.5.22", aReconsume(), aEmitChar()))

	// ── SCRIPT DATA ESCAPED (§13.2.5.23) ──
	ts = append(ts, t(stScriptDataEscaped, ccHyphen, stScriptDataEscapedDash, "13.2.5.23", aEmitChar(), aAppendTempBuffer()))
	ts = append(ts, t(stScriptDataEscaped, ccLT, stScriptDataEscapedLessThan, "13.2.5.23", aEmitChar(), aAppendTempBuffer()))
	ts = append(ts, t(stScriptDataEscaped, ccNull, stScriptDataEscaped, "13.2.5.23", aErrorNull(), aEmitReplacement(), aAppendTempBuffer()))
	ts = append(ts, t(stScriptDataEscaped, ccEOF, stEOF, "13.2.5.23", aEmitEof()))
	ts = append(ts, tList(stScriptDataEscaped, everythingElse, stScriptDataEscaped, "13.2.5.23", aEmitChar(), aAppendTempBuffer())...)

	// ── SCRIPT DATA ESCAPED DASH (§13.2.5.24) ──
	ts = append(ts, t(stScriptDataEscapedDash, ccHyphen, stScriptDataEscapedDashDash, "13.2.5.24", aEmitChar(), aAppendTempBuffer()))
	ts = append(ts, t(stScriptDataEscapedDash, ccLT, stScriptDataEscapedLessThan, "13.2.5.24", aEmitChar(), aAppendTempBuffer()))
	ts = append(ts, t(stScriptDataEscapedDash, ccNull, stScriptDataEscaped, "13.2.5.24", aErrorNull(), aEmitReplacement(), aAppendTempBuffer()))
	ts = append(ts, t(stScriptDataEscapedDash, ccEOF, stEOF, "13.2.5.24", aEmitEof()))
	ts = append(ts, tList(stScriptDataEscapedDash, everythingElse, stScriptDataEscaped, "13.2.5.24", aEmitChar(), aAppendTempBuffer())...)

	// ── SCRIPT DATA ESCAPED DASH DASH (§13.2.5.25) ──
	ts = append(ts, t(stScriptDataEscapedDashDash, ccHyphen, stScriptDataEscapedDashDash, "13.2.5.25", aEmitChar(), aAppendTempBuffer()))
	ts = append(ts, t(stScriptDataEscapedDashDash, ccLT, stScriptDataEscapedLessThan, "13.2.5.25", aEmitChar(), aAppendTempBuffer()))
	ts = append(ts, t(stScriptDataEscapedDashDash, ccGT, stScriptData, "13.2.5.25", aEmitChar()))
	ts = append(ts, t(stScriptDataEscapedDashDash, ccNull, stScriptDataEscaped, "13.2.5.25", aErrorNull(), aEmitReplacement(), aAppendTempBuffer()))
	ts = append(ts, t(stScriptDataEscapedDashDash, ccEOF, stEOF, "13.2.5.25", aEmitEof()))
	escapedDD := append([]html.CharClass{ccTab, ccLF, ccFF, ccCR, ccSpace, ccQuot, ccApos, ccSlash, ccAmp, ccDigit, ccUpper, ccLower, ccEQ, ccQuest, ccSemi, ccBang, ccSqClose, ccPct, ccOther})
	ts = append(ts, tList(stScriptDataEscapedDashDash, escapedDD, stScriptDataEscaped, "13.2.5.25", aEmitChar(), aAppendTempBuffer())...)

	// ── SCRIPT DATA ESCAPED LESS-THAN SIGN (§13.2.5.26) ──
	ts = append(ts, t(stScriptDataEscapedLessThan, ccSlash, stScriptDataEscapedEndTagOpen, "13.2.5.26", aAppendTempBuffer()))
	ts = append(ts, t(stScriptDataEscapedLessThan, ccLower, stScriptDataDoubleEscapeStart, "13.2.5.26", aEmitChar(), aAppendTempBuffer()))
	ts = append(ts, t(stScriptDataEscapedLessThan, ccUpper, stScriptDataDoubleEscapeStart, "13.2.5.26", aEmitChar(), aAppendTempBuffer()))
	ts = append(ts, t(stScriptDataEscapedLessThan, ccEOF, stScriptDataEscaped, "13.2.5.26", aReconsume()))
	ts = append(ts, t(stScriptDataEscapedLessThan, ccOther, stScriptDataEscaped, "13.2.5.26", aReconsume(), aEmitChar()))

	// ── SCRIPT DATA ESCAPED END TAG OPEN (§13.2.5.27) ──
	ts = append(ts, t(stScriptDataEscapedEndTagOpen, ccLower, stScriptDataEscapedEndTagName, "13.2.5.27", aEndTag(), aAppendTagName(), aAppendTempBuffer()))
	ts = append(ts, t(stScriptDataEscapedEndTagOpen, ccUpper, stScriptDataEscapedEndTagName, "13.2.5.27", aEndTag(), aAppendTagName(), aAppendTempBuffer()))
	ts = append(ts, t(stScriptDataEscapedEndTagOpen, ccEOF, stScriptDataEscaped, "13.2.5.27", aReconsume()))
	ts = append(ts, t(stScriptDataEscapedEndTagOpen, ccOther, stScriptDataEscaped, "13.2.5.27", aReconsume(), aEmitChar()))

	// ── SCRIPT DATA ESCAPED END TAG NAME (§13.2.5.28) ──
	ts = append(ts, tList(stScriptDataEscapedEndTagName, whitespace(), stScriptDataEscapedEndTagNameAfter, "13.2.5.28", aAppendTempBuffer(), aCheckAppropriateEndTag())...)
	ts = append(ts, t(stScriptDataEscapedEndTagName, ccSlash, stScriptDataEscapedEndTagNameAfter, "13.2.5.28", aAppendTempBuffer(), aCheckAppropriateEndTag()))
	ts = append(ts, t(stScriptDataEscapedEndTagName, ccGT, stScriptData, "13.2.5.28", aCheckAppropriateEndTag()))
	ts = append(ts, tList(stScriptDataEscapedEndTagName, []html.CharClass{ccLower, ccUpper}, stScriptDataEscapedEndTagName, "13.2.5.28", aAppendTagName(), aAppendTempBuffer())...)
	ts = append(ts, t(stScriptDataEscapedEndTagName, ccEOF, stScriptDataEscaped, "13.2.5.28", aReconsume()))
	ts = append(ts, t(stScriptDataEscapedEndTagName, ccOther, stScriptDataEscaped, "13.2.5.28", aReconsume(), aEmitChar()))

	// ── SCRIPT DATA ESCAPED END TAG NAME AFTER (§13.2.5.28a) ──
	ts = append(ts, t(stScriptDataEscapedEndTagNameAfter, ccSlash, stBeforeAttrName, "13.2.5.28a", aAppendTempBuffer()))
	ts = append(ts, t(stScriptDataEscapedEndTagNameAfter, ccGT, stScriptData, "13.2.5.28a", aEmitToken(ttStartTag), aAppendTempBuffer()))
	ts = append(ts, tList(stScriptDataEscapedEndTagNameAfter, append(whitespace(), ccLower, ccUpper), stBeforeAttrName, "13.2.5.28a", aAppendTempBuffer())...)
	ts = append(ts, t(stScriptDataEscapedEndTagNameAfter, ccEOF, stScriptDataEscaped, "13.2.5.28a", aReconsume()))
	ts = append(ts, t(stScriptDataEscapedEndTagNameAfter, ccOther, stScriptDataEscaped, "13.2.5.28a", aReconsume(), aEmitChar()))

	// ── SCRIPT DATA DOUBLE ESCAPE START (§13.2.5.30) ──
	doubleStartChars := []html.CharClass{ccTab, ccLF, ccFF, ccCR, ccSpace, ccSlash, ccGT}
	ts = append(ts, tList(stScriptDataDoubleEscapeStart, doubleStartChars, stScriptDataDoubleEscaped, "13.2.5.30", aEmitChar(), aAppendTempBuffer(), aCheckTempBufferIsScript())...)
	ts = append(ts, tList(stScriptDataDoubleEscapeStart, []html.CharClass{ccLower, ccUpper}, stScriptDataDoubleEscapeStart, "13.2.5.30", aEmitChar(), aAppendTempBuffer())...)
	ts = append(ts, t(stScriptDataDoubleEscapeStart, ccOther, stScriptDataEscaped, "13.2.5.30", aReconsume()))

	// ── SCRIPT DATA DOUBLE ESCAPED (§13.2.5.31) ──
	ts = append(ts, t(stScriptDataDoubleEscaped, ccHyphen, stScriptDataDoubleEscapedDash, "13.2.5.31", aEmitChar()))
	ts = append(ts, t(stScriptDataDoubleEscaped, ccLT, stScriptDataDoubleEscapedLessThan, "13.2.5.31", aEmitChar()))
	ts = append(ts, t(stScriptDataDoubleEscaped, ccNull, stScriptDataDoubleEscaped, "13.2.5.31", aErrorNull(), aEmitReplacement()))
	ts = append(ts, t(stScriptDataDoubleEscaped, ccEOF, stEOF, "13.2.5.31", aEmitEof()))
	ts = append(ts, tList(stScriptDataDoubleEscaped, everythingElse, stScriptDataDoubleEscaped, "13.2.5.31", aEmitChar())...)

	// ── SCRIPT DATA DOUBLE ESCAPED DASH (§13.2.5.32) ──
	ts = append(ts, t(stScriptDataDoubleEscapedDash, ccHyphen, stScriptDataDoubleEscapedDashDash, "13.2.5.32", aEmitChar()))
	ts = append(ts, t(stScriptDataDoubleEscapedDash, ccLT, stScriptDataDoubleEscapedLessThan, "13.2.5.32", aEmitChar()))
	ts = append(ts, t(stScriptDataDoubleEscapedDash, ccNull, stScriptDataDoubleEscaped, "13.2.5.32", aErrorNull(), aEmitReplacement()))
	ts = append(ts, t(stScriptDataDoubleEscapedDash, ccEOF, stEOF, "13.2.5.32", aEmitEof()))
	ts = append(ts, tList(stScriptDataDoubleEscapedDash, everythingElse, stScriptDataDoubleEscaped, "13.2.5.32", aEmitChar())...)

	// ── SCRIPT DATA DOUBLE ESCAPED DASH DASH (§13.2.5.33) ──
	ts = append(ts, t(stScriptDataDoubleEscapedDashDash, ccHyphen, stScriptDataDoubleEscapedDashDash, "13.2.5.33", aEmitChar()))
	ts = append(ts, t(stScriptDataDoubleEscapedDashDash, ccLT, stScriptDataDoubleEscapedLessThan, "13.2.5.33", aEmitChar()))
	ts = append(ts, t(stScriptDataDoubleEscapedDashDash, ccGT, stScriptData, "13.2.5.33", aEmitChar()))
	ts = append(ts, t(stScriptDataDoubleEscapedDashDash, ccNull, stScriptDataDoubleEscaped, "13.2.5.33", aErrorNull(), aEmitReplacement()))
	ts = append(ts, t(stScriptDataDoubleEscapedDashDash, ccEOF, stEOF, "13.2.5.33", aEmitEof()))
	ts = append(ts, tList(stScriptDataDoubleEscapedDashDash, escapedDD, stScriptDataDoubleEscaped, "13.2.5.33", aEmitChar())...)

	// ── SCRIPT DATA DOUBLE ESCAPED LESS-THAN SIGN (§13.2.5.34) ──
	ts = append(ts, t(stScriptDataDoubleEscapedLessThan, ccSlash, stScriptDataDoubleEscapeEnd, "13.2.5.34", aEmitChar(), aAppendTempBuffer()))
	ts = append(ts, t(stScriptDataDoubleEscapedLessThan, ccLower, stScriptDataDoubleEscapeEnd, "13.2.5.34", aEmitChar(), aAppendTempBuffer()))
	ts = append(ts, t(stScriptDataDoubleEscapedLessThan, ccUpper, stScriptDataDoubleEscapeEnd, "13.2.5.34", aEmitChar(), aAppendTempBuffer()))
	ts = append(ts, t(stScriptDataDoubleEscapedLessThan, ccEOF, stScriptDataDoubleEscaped, "13.2.5.34", aReconsume()))
	ts = append(ts, t(stScriptDataDoubleEscapedLessThan, ccOther, stScriptDataDoubleEscaped, "13.2.5.34", aReconsume(), aEmitChar()))

	// ── SCRIPT DATA DOUBLE ESCAPE END (§13.2.5.35) ──
	ts = append(ts, tList(stScriptDataDoubleEscapeEnd, doubleStartChars, stScriptDataEscaped, "13.2.5.35", aEmitChar(), aAppendTempBuffer(), aCheckTempBufferIsScript())...)
	ts = append(ts, tList(stScriptDataDoubleEscapeEnd, []html.CharClass{ccLower, ccUpper}, stScriptDataDoubleEscapeEnd, "13.2.5.35", aEmitChar(), aAppendTempBuffer())...)
	ts = append(ts, t(stScriptDataDoubleEscapeEnd, ccOther, stScriptDataDoubleEscaped, "13.2.5.35", aReconsume()))

	// ── PLAINTEXT (§13.2.5.5) ──
	ts = append(ts, t(stPlaintext, ccNull, stPlaintext, "13.2.5.5", aErrorNull(), aEmitReplacement()))
	ts = append(ts, t(stPlaintext, ccEOF, stEOF, "13.2.5.5", aEmitEof()))
	plaintextChars := append([]html.CharClass{ccAmp, ccLT}, everythingElse...)
	ts = append(ts, tList(stPlaintext, plaintextChars, stPlaintext, "13.2.5.5", aEmitChar())...)

	// ── TAG OPEN (§13.2.5.6) ──
	ts = append(ts, t(stTagOpen, ccBang, stMarkupDeclOpen, "13.2.5.6"))
	ts = append(ts, t(stTagOpen, ccSlash, stEndTagOpen, "13.2.5.6"))
	ts = append(ts, t(stTagOpen, ccLower, stTagName, "13.2.5.6", aStartTag(), aAppendTagName()))
	ts = append(ts, t(stTagOpen, ccUpper, stTagName, "13.2.5.6", aStartTag(), aAppendTagName()))
	ts = append(ts, t(stTagOpen, ccQuest, stBogusComment, "13.2.5.6", aErrorChar(), aCreateComment()))
	ts = append(ts, t(stTagOpen, ccEOF, stData, "13.2.5.6", aErrorChar(), aEmitChar()))
	ts = append(ts, t(stTagOpen, ccOther, stData, "13.2.5.6", aErrorChar(), aEmitChar()))

	// ── END TAG OPEN (§13.2.5.7) ──
	ts = append(ts, t(stEndTagOpen, ccLower, stTagName, "13.2.5.7", aEndTag(), aAppendTagName()))
	ts = append(ts, t(stEndTagOpen, ccUpper, stTagName, "13.2.5.7", aEndTag(), aAppendTagName()))
	ts = append(ts, t(stEndTagOpen, ccGT, stData, "13.2.5.7", aErrorChar()))
	ts = append(ts, t(stEndTagOpen, ccEOF, stData, "13.2.5.7", aErrorChar(), aEmitChar()))
	ts = append(ts, t(stEndTagOpen, ccOther, stBogusComment, "13.2.5.7", aErrorChar(), aCreateComment()))

	// ── TAG NAME (§13.2.5.8) ──
	ts = append(ts, tList(stTagName, whitespace(), stBeforeAttrName, "13.2.5.8")...)
	ts = append(ts, t(stTagName, ccSlash, stAfterTagName, "13.2.5.8"))
	ts = append(ts, t(stTagName, ccGT, stData, "13.2.5.8", aEmitToken(ttStartTag)))
	ts = append(ts, t(stTagName, ccNull, stTagName, "13.2.5.8", aErrorNull(), aAppendTagName()))
	ts = append(ts, t(stTagName, ccEOF, stData, "13.2.5.8", aErrorChar(), aEmitToken(ttStartTag)))
	tagNameChars := []html.CharClass{ccQuot, ccApos, ccHyphen, ccDigit, ccUpper, ccLower, ccEQ, ccQuest, ccSemi, ccBang, ccSqClose, ccPct, ccOther}
	ts = append(ts, tList(stTagName, tagNameChars, stTagName, "13.2.5.8", aAppendTagName())...)

	// ── BEFORE TAG NAME (§13.2.5.36) ──
	ts = append(ts, tList(stBeforeTagName, whitespace(), stBeforeTagName, "13.2.5.36")...)
	ts = append(ts, t(stBeforeTagName, ccSlash, stSelfClosingStartTag, "13.2.5.36"))
	ts = append(ts, t(stBeforeTagName, ccGT, stData, "13.2.5.36", aErrorChar(), aEmitToken(ttStartTag)))
	ts = append(ts, t(stBeforeTagName, ccEOF, stData, "13.2.5.36", aErrorChar(), aEmitToken(ttStartTag)))
	beforeTagChars := []html.CharClass{ccQuot, ccApos, ccHyphen, ccLower, ccUpper, ccDigit, ccEQ, ccQuest, ccBang, ccSqClose, ccPct, ccOther}
	ts = append(ts, tList(stBeforeTagName, beforeTagChars, stTagName, "13.2.5.36", aStartTag(), aAppendTagName())...)
	ts = append(ts, t(stBeforeTagName, ccNull, stTagName, "13.2.5.36", aErrorNull(), aStartTag(), aAppendTagName()))

	// ── AFTER TAG NAME (§13.2.5.37) ──
	ts = append(ts, tList(stAfterTagName, whitespace(), stAfterTagName, "13.2.5.37")...)
	ts = append(ts, t(stAfterTagName, ccSlash, stSelfClosingStartTag, "13.2.5.37"))
	ts = append(ts, t(stAfterTagName, ccGT, stData, "13.2.5.37", aEmitToken(ttStartTag)))
	ts = append(ts, t(stAfterTagName, ccEOF, stData, "13.2.5.37", aErrorChar(), aEmitToken(ttStartTag)))
	afterTagChars := []html.CharClass{ccQuot, ccApos, ccLower, ccUpper, ccDigit, ccHyphen, ccEQ, ccNull, ccQuest, ccBang, ccSqClose, ccPct, ccOther}
	ts = append(ts, tList(stAfterTagName, afterTagChars, stBeforeAttrName, "13.2.5.37", aErrorChar(), aAppendAttrName())...)
	ts = append(ts, t(stAfterTagName, ccNull, stBeforeAttrName, "13.2.5.37", aErrorNull(), aAppendAttrName()))

	// ── SELF-CLOSING START TAG (§13.2.5.38) ──
	ts = append(ts, t(stSelfClosingStartTag, ccGT, stData, "13.2.5.38", aSetSelfClosing(), aEmitToken(ttStartTag)))
	ts = append(ts, t(stSelfClosingStartTag, ccEOF, stData, "13.2.5.38", aErrorChar(), aEmitToken(ttStartTag)))
	selfCloseChars := []html.CharClass{ccTab, ccLF, ccFF, ccCR, ccSpace, ccQuot, ccApos, ccHyphen, ccSlash, ccAmp, ccDigit, ccUpper, ccLower, ccEQ, ccQuest, ccSemi, ccBang, ccSqClose, ccPct, ccNull, ccOther}
	ts = append(ts, tList(stSelfClosingStartTag, selfCloseChars, stBeforeAttrName, "13.2.5.38", aErrorChar(), aAppendAttrName())...)

	// ── BEFORE ATTRIBUTE_NAME (§13.2.5.39) ──
	ts = append(ts, tList(stBeforeAttrName, whitespace(), stBeforeAttrName, "13.2.5.39")...)
	ts = append(ts, t(stBeforeAttrName, ccSlash, stSelfClosingStartTag, "13.2.5.39"))
	ts = append(ts, t(stBeforeAttrName, ccEQ, stAttrName, "13.2.5.39", aErrorChar(), aStartTag(), aAppendAttrName()))
	ts = append(ts, t(stBeforeAttrName, ccGT, stData, "13.2.5.39", aEmitToken(ttStartTag)))
	ts = append(ts, t(stBeforeAttrName, ccEOF, stData, "13.2.5.39", aErrorChar(), aEmitToken(ttStartTag)))
	beforeAttrChars := []html.CharClass{ccQuot, ccApos, ccLower, ccUpper, ccDigit, ccHyphen, ccNull, ccQuest, ccBang, ccSqClose, ccPct, ccOther}
	ts = append(ts, tList(stBeforeAttrName, beforeAttrChars, stAttrName, "13.2.5.39", aStartTag(), aAppendAttrName())...)
	ts = append(ts, t(stBeforeAttrName, ccNull, stAttrName, "13.2.5.39", aErrorNull(), aStartTag(), aAppendAttrName()))

	// ── ATTRIBUTE_NAME (§13.2.5.40) ──
	ts = append(ts, tList(stAttrName, whitespace(), stAfterAttrName, "13.2.5.40")...)
	ts = append(ts, t(stAttrName, ccSlash, stAfterAttrName, "13.2.5.40"))
	ts = append(ts, t(stAttrName, ccEQ, stBeforeAttrValue, "13.2.5.40"))
	ts = append(ts, t(stAttrName, ccGT, stData, "13.2.5.40", aEmitToken(ttStartTag)))
	ts = append(ts, t(stAttrName, ccEOF, stData, "13.2.5.40", aErrorChar(), aEmitToken(ttStartTag)))
	attrNameChars := []html.CharClass{ccQuot, ccApos, ccLower, ccUpper, ccDigit, ccHyphen, ccNull, ccQuest, ccBang, ccSqClose, ccPct, ccOther}
	ts = append(ts, tList(stAttrName, attrNameChars, stAttrName, "13.2.5.40", aAppendAttrName())...)
	ts = append(ts, t(stAttrName, ccNull, stAttrName, "13.2.5.40", aErrorNull(), aAppendAttrName()))

	// ── AFTER ATTRIBUTE NAME (§13.2.5.41) ──
	ts = append(ts, tList(stAfterAttrName, whitespace(), stAfterAttrName, "13.2.5.41")...)
	ts = append(ts, t(stAfterAttrName, ccSlash, stSelfClosingStartTag, "13.2.5.41"))
	ts = append(ts, t(stAfterAttrName, ccEQ, stBeforeAttrValue, "13.2.5.41"))
	ts = append(ts, t(stAfterAttrName, ccGT, stData, "13.2.5.41", aEmitToken(ttStartTag)))
	ts = append(ts, t(stAfterAttrName, ccEOF, stData, "13.2.5.41", aErrorChar(), aEmitToken(ttStartTag)))
	afterAttrChars := []html.CharClass{ccQuot, ccApos, ccLower, ccUpper, ccDigit, ccHyphen, ccNull, ccQuest, ccBang, ccSqClose, ccPct, ccOther}
	ts = append(ts, tList(stAfterAttrName, afterAttrChars, stAttrName, "13.2.5.41", aErrorChar(), aAppendAttrName())...)
	ts = append(ts, t(stAfterAttrName, ccNull, stAttrName, "13.2.5.41", aErrorNull(), aAppendAttrName()))

	// ── BEFORE ATTRIBUTE VALUE (§13.2.5.42) ──
	ts = append(ts, tList(stBeforeAttrValue, whitespace(), stBeforeAttrValue, "13.2.5.42")...)
	ts = append(ts, t(stBeforeAttrValue, ccQuot, stAttrValueDoubleQuoted, "13.2.5.42"))
	ts = append(ts, t(stBeforeAttrValue, ccApos, stAttrValueSingleQuoted, "13.2.5.42"))
	ts = append(ts, t(stBeforeAttrValue, ccGT, stData, "13.2.5.42", aErrorChar(), aEmitToken(ttStartTag)))
	ts = append(ts, t(stBeforeAttrValue, ccEOF, stData, "13.2.5.42", aErrorChar(), aEmitToken(ttStartTag)))
	beforeValChars := []html.CharClass{ccLower, ccUpper, ccDigit, ccHyphen, ccEQ, ccQuest, ccBang, ccPct, ccOther, ccNull, ccSlash, ccSemi, ccSqClose}
	ts = append(ts, tList(stBeforeAttrValue, beforeValChars, stAttrValueUnquoted, "13.2.5.42", aAppendAttrValue())...)

	// ── ATTRIBUTE VALUE DOUBLE QUOTED (§13.2.5.43) ──
	ts = append(ts, t(stAttrValueDoubleQuoted, ccQuot, stAfterAttrName, "13.2.5.43"))
	ts = append(ts, t(stAttrValueDoubleQuoted, ccAmp, stCharRef, "13.2.5.43", aSetReturnState(stAttrValueDoubleQuoted)))
	ts = append(ts, t(stAttrValueDoubleQuoted, ccNull, stAttrValueDoubleQuoted, "13.2.5.43", aErrorNull(), aAppendAttrValue()))
	ts = append(ts, t(stAttrValueDoubleQuoted, ccEOF, stData, "13.2.5.43", aErrorChar(), aEmitToken(ttStartTag)))
	dqChars := []html.CharClass{ccTab, ccLF, ccFF, ccCR, ccSpace, ccApos, ccHyphen, ccSlash, ccLT, ccGT, ccEQ, ccQuest, ccSemi, ccBang, ccSqClose, ccPct, ccDigit, ccUpper, ccLower, ccOther}
	ts = append(ts, tList(stAttrValueDoubleQuoted, dqChars, stAttrValueDoubleQuoted, "13.2.5.43", aAppendAttrValue())...)

	// ── ATTRIBUTE VALUE SINGLE QUOTED (§13.2.5.44) ──
	ts = append(ts, t(stAttrValueSingleQuoted, ccApos, stAfterAttrName, "13.2.5.44"))
	ts = append(ts, t(stAttrValueSingleQuoted, ccAmp, stCharRef, "13.2.5.44", aSetReturnState(stAttrValueSingleQuoted)))
	ts = append(ts, t(stAttrValueSingleQuoted, ccNull, stAttrValueSingleQuoted, "13.2.5.44", aErrorNull(), aAppendAttrValue()))
	ts = append(ts, t(stAttrValueSingleQuoted, ccEOF, stData, "13.2.5.44", aErrorChar(), aEmitToken(ttStartTag)))
	sqChars := []html.CharClass{ccTab, ccLF, ccFF, ccCR, ccSpace, ccQuot, ccHyphen, ccSlash, ccLT, ccGT, ccEQ, ccQuest, ccSemi, ccBang, ccSqClose, ccPct, ccDigit, ccUpper, ccLower, ccOther}
	ts = append(ts, tList(stAttrValueSingleQuoted, sqChars, stAttrValueSingleQuoted, "13.2.5.44", aAppendAttrValue())...)

	// ── ATTRIBUTE VALUE UNQUOTED (§13.2.5.45) ──
	ts = append(ts, tList(stAttrValueUnquoted, whitespace(), stBeforeAttrName, "13.2.5.45")...)
	ts = append(ts, t(stAttrValueUnquoted, ccAmp, stCharRef, "13.2.5.45", aSetReturnState(stAttrValueUnquoted)))
	ts = append(ts, t(stAttrValueUnquoted, ccGT, stData, "13.2.5.45", aEmitToken(ttStartTag)))
	ts = append(ts, t(stAttrValueUnquoted, ccNull, stAttrValueUnquoted, "13.2.5.45", aErrorNull(), aAppendAttrValue()))
	ts = append(ts, t(stAttrValueUnquoted, ccEOF, stData, "13.2.5.45", aErrorChar(), aEmitToken(ttStartTag)))
	unqChars := []html.CharClass{ccApos, ccHyphen, ccSlash, ccEQ, ccQuest, ccBang, ccPct, ccDigit, ccUpper, ccLower, ccOther}
	ts = append(ts, tList(stAttrValueUnquoted, unqChars, stAttrValueUnquoted, "13.2.5.45", aAppendAttrValue())...)
	unqErrChars := []html.CharClass{ccQuot, ccSemi, ccSqClose}
	ts = append(ts, tList(stAttrValueUnquoted, unqErrChars, stAttrValueUnquoted, "13.2.5.45", aErrorChar(), aAppendAttrValue())...)

	// ── AFTER ATTRIBUTE VALUE QUOTED (§13.2.5.46) ──
	ts = append(ts, tList(stAfterAttrValueQuoted, whitespace(), stAfterAttrName, "13.2.5.46")...)
	ts = append(ts, t(stAfterAttrValueQuoted, ccSlash, stSelfClosingStartTag, "13.2.5.46"))
	ts = append(ts, t(stAfterAttrValueQuoted, ccGT, stData, "13.2.5.46", aEmitToken(ttStartTag)))
	ts = append(ts, t(stAfterAttrValueQuoted, ccEOF, stData, "13.2.5.46", aErrorChar(), aEmitToken(ttStartTag)))
	afterValChars := []html.CharClass{ccQuot, ccApos, ccHyphen, ccLower, ccUpper, ccDigit, ccEQ, ccQuest, ccBang, ccSqClose, ccPct, ccNull, ccOther}
	ts = append(ts, tList(stAfterAttrValueQuoted, afterValChars, stBeforeAttrName, "13.2.5.46", aErrorChar(), aAppendAttrName())...)

	// ── CHARACTER REFERENCE (§13.2.5.47) ──
	ts = append(ts, t(stCharRef, ccLower, stNamedCharRef, "13.2.5.47", aAppendTokenData()))
	ts = append(ts, t(stCharRef, ccUpper, stNamedCharRef, "13.2.5.47", aAppendTokenData()))
	ts = append(ts, t(stCharRef, ccDigit, stNumericCharRef, "13.2.5.47", aAppendTokenData()))
	ts = append(ts, t(stCharRef, ccOther, stAmbiguousAmpersand, "13.2.5.47"))

	// ── NAMED CHARACTER REFERENCE (§13.2.5.48) ──
	ts = append(ts, t(stNamedCharRef, ccSemi, stData, "13.2.5.48", aFlushCharRef()))
	ts = append(ts, tList(stNamedCharRef, []html.CharClass{ccDigit, ccLower, ccUpper}, stNamedCharRef, "13.2.5.48", aAppendTokenData())...)
	ts = append(ts, t(stNamedCharRef, ccEQ, stAmbiguousAmpersand, "13.2.5.48"))
	namedFallback := []html.CharClass{ccTab, ccLF, ccFF, ccCR, ccSpace, ccGT, ccOther}
	ts = append(ts, tList(stNamedCharRef, namedFallback, stData, "13.2.5.48", aErrorMissingSemi(), aFlushCharRef())...)
	ts = append(ts, t(stNamedCharRef, ccEOF, stData, "13.2.5.48", aFlushCharRef()))
	ts = append(ts, t(stNamedCharRef, ccNull, stAmbiguousAmpersand, "13.2.5.48"))
	namedChars := []html.CharClass{ccQuot, ccApos, ccHyphen, ccSlash, ccLT, ccQuest, ccBang, ccSqClose, ccPct}
	ts = append(ts, tList(stNamedCharRef, namedChars, stData, "13.2.5.48", aErrorMissingSemi(), aFlushCharRef())...)

	// ── AMBIGUOUS AMPERSAND (§13.2.5.49) ──
	ts = append(ts, tList(stAmbiguousAmpersand, []html.CharClass{ccLower, ccUpper, ccDigit}, stAmbiguousAmpersand, "13.2.5.49", aAppendTokenData())...)
	ts = append(ts, t(stAmbiguousAmpersand, ccSemi, stAmbiguousAmpersand, "13.2.5.49", aFlushCharRef()))
	ambigChars := []html.CharClass{ccGT, ccTab, ccLF, ccFF, ccCR, ccSpace, ccQuot, ccApos, ccHyphen, ccSlash, ccLT, ccEQ, ccQuest, ccBang, ccSqClose, ccPct, ccNull, ccOther}
	ts = append(ts, tList(stAmbiguousAmpersand, ambigChars, stData, "13.2.5.49", aFlushCharRef())...)
	ts = append(ts, t(stAmbiguousAmpersand, ccEOF, stData, "13.2.5.49", aFlushCharRef()))

	// ── NUMERIC CHARACTER REFERENCE (§13.2.5.50) ──
	ts = append(ts, t(stNumericCharRef, ccLower, stHexCharRef, "13.2.5.50", aAppendTokenData()))
	ts = append(ts, t(stNumericCharRef, ccUpper, stHexCharRef, "13.2.5.50", aAppendTokenData()))
	ts = append(ts, t(stNumericCharRef, ccDigit, stDecCharRef, "13.2.5.50", aAppendTokenData()))
	ts = append(ts, t(stNumericCharRef, ccOther, stData, "13.2.5.50", aErrorChar(), aFlushCharRef()))

	// ── HEXADECIMAL CHARACTER REFERENCE (§13.2.5.53) ──
	ts = append(ts, tList(stHexCharRef, []html.CharClass{ccDigit, ccLower, ccUpper}, stHexCharRef, "13.2.5.53", aAppendTokenData())...)
	ts = append(ts, t(stHexCharRef, ccSemi, stNumericCharRefEnd, "13.2.5.53"))
	hexEnd := []html.CharClass{ccTab, ccLF, ccFF, ccCR, ccSpace, ccQuot, ccApos, ccHyphen, ccSlash, ccLT, ccGT, ccEQ, ccQuest, ccBang, ccSqClose, ccPct, ccNull, ccOther}
	ts = append(ts, tList(stHexCharRef, hexEnd, stData, "13.2.5.53", aErrorChar(), aFlushCharRef())...)
	ts = append(ts, t(stHexCharRef, ccEOF, stData, "13.2.5.53", aErrorChar(), aFlushCharRef()))

	// ── DECIMAL CHARACTER REFERENCE (§13.2.5.54) ──
	ts = append(ts, t(stDecCharRef, ccDigit, stDecCharRef, "13.2.5.54", aAppendTokenData()))
	ts = append(ts, t(stDecCharRef, ccSemi, stNumericCharRefEnd, "13.2.5.54"))
	decEnd := []html.CharClass{ccTab, ccLF, ccFF, ccCR, ccSpace, ccQuot, ccApos, ccHyphen, ccSlash, ccLT, ccGT, ccEQ, ccQuest, ccBang, ccSqClose, ccPct, ccNull, ccUpper, ccLower, ccOther}
	ts = append(ts, tList(stDecCharRef, decEnd, stData, "13.2.5.54", aErrorChar(), aFlushCharRef())...)
	ts = append(ts, t(stDecCharRef, ccEOF, stData, "13.2.5.54", aErrorChar(), aFlushCharRef()))

	// ── NUMERIC CHARACTER REFERENCE END (§13.2.5.55) ──
	ts = append(ts, t(stNumericCharRefEnd, ccNull, stData, "13.2.5.55", aErrorChar(), aEmitReplacement(), aFlushCharRef()))
	numEnd := []html.CharClass{ccTab, ccLF, ccFF, ccCR, ccSpace, ccQuot, ccApos, ccHyphen, ccSlash, ccLT, ccGT, ccEQ, ccQuest, ccSemi, ccBang, ccSqClose, ccPct, ccDigit, ccUpper, ccLower, ccOther}
	ts = append(ts, tList(stNumericCharRefEnd, numEnd, stData, "13.2.5.55", aFlushCharRef())...)
	ts = append(ts, t(stNumericCharRefEnd, ccEOF, stData, "13.2.5.55", aErrorChar(), aFlushCharRef()))

	// ── MARKUP DECLARATION OPEN (§13.2.5.71) ──
	ts = append(ts, t(stMarkupDeclOpen, ccHyphen, stCommentStart, "13.2.5.71", aCreateComment(), aAppendTokenData()))
	ts = append(ts, t(stMarkupDeclOpen, ccLower, stDoctype, "13.2.5.71", aCreateDoctype(), aAppendTokenData()))
	ts = append(ts, t(stMarkupDeclOpen, ccUpper, stDoctype, "13.2.5.71", aCreateDoctype(), aAppendTokenData()))
	ts = append(ts, t(stMarkupDeclOpen, ccSqClose, stCDATA, "13.2.5.71", aAppendTokenData()))
	ts = append(ts, t(stMarkupDeclOpen, ccOther, stBogusComment, "13.2.5.71", aErrorChar(), aCreateComment()))
	ts = append(ts, t(stMarkupDeclOpen, ccEOF, stBogusComment, "13.2.5.71", aErrorChar(), aCreateComment()))

	// ── COMMENT (§13.2.5.74) ──
	ts = append(ts, t(stComment, ccLT, stCommentLessThanSign, "13.2.5.74", aEmitChar()))
	ts = append(ts, t(stComment, ccHyphen, stCommentEndDash, "13.2.5.74"))
	ts = append(ts, t(stComment, ccNull, stComment, "13.2.5.74", aErrorNull(), aEmitReplacement()))
	ts = append(ts, t(stComment, ccEOF, stData, "13.2.5.74", aErrorChar(), aEmitToken(ttStartTag)))
	commentChars := []html.CharClass{ccTab, ccLF, ccFF, ccCR, ccSpace, ccQuot, ccApos, ccSlash, ccAmp, ccGT, ccDigit, ccUpper, ccLower, ccEQ, ccQuest, ccSemi, ccBang, ccSqClose, ccPct, ccOther}
	ts = append(ts, tList(stComment, commentChars, stComment, "13.2.5.74", aEmitChar())...)

	// ── COMMENT START (§13.2.5.72) ──
	ts = append(ts, t(stCommentStart, ccHyphen, stCommentStartDash, "13.2.5.72"))
	ts = append(ts, t(stCommentStart, ccGT, stData, "13.2.5.72", aErrorChar(), aEmitToken(ttStartTag)))
	ts = append(ts, t(stCommentStart, ccNull, stComment, "13.2.5.72", aErrorNull(), aEmitReplacement()))
	ts = append(ts, t(stCommentStart, ccEOF, stData, "13.2.5.72", aErrorChar(), aEmitToken(ttStartTag)))
	csChars := []html.CharClass{ccTab, ccLF, ccFF, ccCR, ccSpace, ccQuot, ccApos, ccSlash, ccAmp, ccLT, ccDigit, ccUpper, ccLower, ccEQ, ccQuest, ccSemi, ccBang, ccSqClose, ccPct, ccOther}
	ts = append(ts, tList(stCommentStart, csChars, stComment, "13.2.5.72", aEmitChar())...)

	// ── COMMENT START DASH (§13.2.5.73) ──
	ts = append(ts, t(stCommentStartDash, ccHyphen, stCommentEnd, "13.2.5.73"))
	ts = append(ts, t(stCommentStartDash, ccGT, stData, "13.2.5.73", aErrorChar(), aEmitToken(ttStartTag)))
	ts = append(ts, t(stCommentStartDash, ccNull, stComment, "13.2.5.73", aErrorNull(), aEmitReplacement()))
	ts = append(ts, t(stCommentStartDash, ccEOF, stData, "13.2.5.73", aErrorChar(), aEmitToken(ttStartTag)))
	csdChars := []html.CharClass{ccTab, ccLF, ccFF, ccCR, ccSpace, ccQuot, ccApos, ccSlash, ccAmp, ccLT, ccDigit, ccUpper, ccLower, ccEQ, ccQuest, ccSemi, ccBang, ccSqClose, ccPct, ccOther}
	ts = append(ts, tList(stCommentStartDash, csdChars, stComment, "13.2.5.73", aEmitChar())...)

	// ── COMMENT LESS-THAN SIGN (§13.2.5.75) ──
	ts = append(ts, t(stCommentLessThanSign, ccBang, stCommentLessThanSignBang, "13.2.5.75", aAppendTokenData()))
	ts = append(ts, t(stCommentLessThanSign, ccLT, stCommentLessThanSign, "13.2.5.75", aEmitChar()))
	ts = append(ts, t(stCommentLessThanSign, ccOther, stComment, "13.2.5.75", aReconsume(), aEmitChar()))
	ts = append(ts, t(stCommentLessThanSign, ccEOF, stComment, "13.2.5.75", aReconsume()))

	// ── COMMENT LESS-THAN SIGN BANG (§13.2.5.76) ──
	ts = append(ts, t(stCommentLessThanSignBang, ccHyphen, stCommentLessThanSignBangDash, "13.2.5.76"))
	ts = append(ts, t(stCommentLessThanSignBang, ccOther, stComment, "13.2.5.76", aReconsume()))
	ts = append(ts, t(stCommentLessThanSignBang, ccEOF, stCommentEnd, "13.2.5.76"))

	// ── COMMENT LESS-THAN SIGN BANG DASH (§13.2.5.77) ──
	ts = append(ts, t(stCommentLessThanSignBangDash, ccHyphen, stCommentLessThanSignBangDashDash, "13.2.5.77"))
	ts = append(ts, t(stCommentLessThanSignBangDash, ccOther, stCommentEndDash, "13.2.5.77", aReconsume()))
	ts = append(ts, t(stCommentLessThanSignBangDash, ccEOF, stCommentEnd, "13.2.5.77"))

	// ── COMMENT LESS-THAN SIGN BANG DASH DASH (§13.2.5.78) ──
	ts = append(ts, t(stCommentLessThanSignBangDashDash, ccGT, stCommentEnd, "13.2.5.78"))
	ts = append(ts, t(stCommentLessThanSignBangDashDash, ccOther, stCommentEnd, "13.2.5.78", aReconsume()))
	ts = append(ts, t(stCommentLessThanSignBangDashDash, ccEOF, stCommentEnd, "13.2.5.78"))

	// ── COMMENT END DASH (§13.2.5.79) ──
	ts = append(ts, t(stCommentEndDash, ccHyphen, stCommentEnd, "13.2.5.79"))
	ts = append(ts, t(stCommentEndDash, ccOther, stComment, "13.2.5.79", aReconsume(), aEmitChar()))
	ts = append(ts, t(stCommentEndDash, ccEOF, stData, "13.2.5.79", aErrorChar(), aEmitToken(ttStartTag)))

	// ── COMMENT END (§13.2.5.80) ──
	ts = append(ts, t(stCommentEnd, ccGT, stData, "13.2.5.80", aEmitToken(ttStartTag)))
	ts = append(ts, t(stCommentEnd, ccBang, stCommentEndBang, "13.2.5.80"))
	ts = append(ts, t(stCommentEnd, ccHyphen, stCommentEnd, "13.2.5.80", aEmitChar()))
	ts = append(ts, t(stCommentEnd, ccOther, stComment, "13.2.5.80", aErrorChar(), aReconsume()))
	ts = append(ts, t(stCommentEnd, ccEOF, stData, "13.2.5.80", aErrorChar(), aEmitToken(ttStartTag)))

	// ── COMMENT END BANG (§13.2.5.81) ──
	ts = append(ts, t(stCommentEndBang, ccHyphen, stCommentEndDash, "13.2.5.81", aEmitChar()))
	ts = append(ts, t(stCommentEndBang, ccGT, stData, "13.2.5.81", aErrorChar(), aEmitToken(ttStartTag)))
	ts = append(ts, t(stCommentEndBang, ccOther, stComment, "13.2.5.81", aErrorChar(), aReconsume(), aEmitChar()))
	ts = append(ts, t(stCommentEndBang, ccEOF, stData, "13.2.5.81", aErrorChar(), aEmitToken(ttStartTag)))

	// ── BOGUS COMMENT (§13.2.5.82) ──
	ts = append(ts, t(stBogusComment, ccGT, stData, "13.2.5.82", aEmitToken(ttStartTag)))
	ts = append(ts, t(stBogusComment, ccEOF, stData, "13.2.5.82", aEmitToken(ttStartTag)))
	bogusChars := []html.CharClass{ccTab, ccLF, ccFF, ccCR, ccSpace, ccQuot, ccApos, ccHyphen, ccSlash, ccAmp, ccLT, ccDigit, ccUpper, ccLower, ccEQ, ccQuest, ccSemi, ccBang, ccSqClose, ccPct, ccNull, ccOther}
	ts = append(ts, tList(stBogusComment, bogusChars, stBogusComment, "13.2.5.82", aEmitChar())...)

	// ── CDATA (§13.2.5.83) ──
	ts = append(ts, t(stCDATA, ccSqClose, stCDATABracket, "13.2.5.83", aEmitChar()))
	ts = append(ts, t(stCDATA, ccEOF, stData, "13.2.5.83", aEmitToken(ttStartTag)))
	cdataChars := []html.CharClass{ccTab, ccLF, ccFF, ccCR, ccSpace, ccQuot, ccApos, ccHyphen, ccSlash, ccAmp, ccLT, ccGT, ccDigit, ccUpper, ccLower, ccEQ, ccQuest, ccSemi, ccBang, ccPct, ccNull, ccOther}
	ts = append(ts, tList(stCDATA, cdataChars, stCDATA, "13.2.5.83", aEmitChar())...)

	// ── CDATA BRACKET (§13.2.5.84) ──
	ts = append(ts, t(stCDATABracket, ccSqClose, stCDATAEnd, "13.2.5.84", aEmitChar()))
	ts = append(ts, t(stCDATABracket, ccOther, stCDATA, "13.2.5.84", aReconsume(), aEmitChar()))
	ts = append(ts, t(stCDATABracket, ccEOF, stData, "13.2.5.84", aEmitToken(ttStartTag)))

	// ── CDATA END (§13.2.5.85) ──
	ts = append(ts, t(stCDATAEnd, ccGT, stData, "13.2.5.85"))
	ts = append(ts, t(stCDATAEnd, ccOther, stCDATA, "13.2.5.85", aReconsume(), aEmitChar(), aEmitChar(), aEmitChar()))
	ts = append(ts, t(stCDATAEnd, ccEOF, stData, "13.2.5.85", aEmitToken(ttStartTag)))

	// ── DOCTYPE (§13.2.5.56) ──
	ts = append(ts, tList(stDoctype, whitespace(), stBeforeDoctypeName, "13.2.5.56")...)
	ts = append(ts, t(stDoctype, ccGT, stData, "13.2.5.56", aErrorChar(), aSetDoctypeQuirks(), aEmitToken(ttStartTag)))
	ts = append(ts, t(stDoctype, ccEOF, stData, "13.2.5.56", aEmitToken(ttStartTag)))
	ts = append(ts, t(stDoctype, ccLower, stDoctypeName, "13.2.5.56", aCreateDoctype(), aAppendTokenData()))
	ts = append(ts, t(stDoctype, ccUpper, stDoctypeName, "13.2.5.56", aCreateDoctype(), aAppendTokenData()))
	ts = append(ts, t(stDoctype, ccNull, stDoctypeName, "13.2.5.56", aErrorNull(), aCreateDoctype(), aAppendTokenData()))
	ts = append(ts, t(stDoctype, ccOther, stDoctypeName, "13.2.5.56", aCreateDoctype(), aAppendTokenData()))

	// ── BEFORE DOCTYPE NAME (§13.2.5.57) ──
	ts = append(ts, tList(stBeforeDoctypeName, whitespace(), stBeforeDoctypeName, "13.2.5.57")...)
	ts = append(ts, t(stBeforeDoctypeName, ccNull, stDoctypeName, "13.2.5.57", aErrorNull(), aAppendTokenData()))
	ts = append(ts, t(stBeforeDoctypeName, ccGT, stData, "13.2.5.57", aErrorChar(), aSetDoctypeQuirks(), aEmitToken(ttStartTag)))
	ts = append(ts, t(stBeforeDoctypeName, ccEOF, stData, "13.2.5.57", aEmitToken(ttStartTag)))
	bdnChars := []html.CharClass{ccLower, ccUpper, ccHyphen, ccOther}
	ts = append(ts, tList(stBeforeDoctypeName, bdnChars, stDoctypeName, "13.2.5.57", aAppendTokenData())...)

	// ── DOCTYPE NAME (§13.2.5.58) ──
	ts = append(ts, tList(stDoctypeName, whitespace(), stAfterDoctypeName, "13.2.5.58")...)
	ts = append(ts, t(stDoctypeName, ccGT, stData, "13.2.5.58", aEmitToken(ttStartTag)))
	ts = append(ts, t(stDoctypeName, ccNull, stDoctypeName, "13.2.5.58", aErrorNull(), aAppendTokenData()))
	ts = append(ts, t(stDoctypeName, ccEOF, stData, "13.2.5.58", aEmitToken(ttStartTag)))
	dnChars := []html.CharClass{ccLower, ccUpper, ccHyphen, ccOther}
	ts = append(ts, tList(stDoctypeName, dnChars, stDoctypeName, "13.2.5.58", aAppendTokenData())...)

	// ── AFTER DOCTYPE NAME (§13.2.5.59) ──
	ts = append(ts, tList(stAfterDoctypeName, whitespace(), stAfterDoctypeName, "13.2.5.59")...)
	ts = append(ts, t(stAfterDoctypeName, ccGT, stData, "13.2.5.59", aEmitToken(ttStartTag)))
	ts = append(ts, t(stAfterDoctypeName, ccEOF, stData, "13.2.5.59", aEmitToken(ttStartTag)))
	ts = append(ts, t(stAfterDoctypeName, ccLower, stAfterDoctypePublicKeyword, "13.2.5.59", aAppendTokenData()))
	ts = append(ts, t(stAfterDoctypeName, ccUpper, stAfterDoctypePublicKeyword, "13.2.5.59", aAppendTokenData()))
	adnErr := []html.CharClass{ccQuot, ccApos, ccHyphen, ccSlash, ccAmp, ccLT, ccDigit, ccEQ, ccQuest, ccSemi, ccBang, ccSqClose, ccPct, ccNull, ccOther}
	ts = append(ts, tList(stAfterDoctypeName, adnErr, stBogusDoctype, "13.2.5.59", aErrorChar(), aSetDoctypeQuirks())...)

	// ── AFTER DOCTYPE PUBLIC KEYWORD (§13.2.5.60) ──
	ts = append(ts, t(stAfterDoctypePublicKeyword, ccGT, stData, "13.2.5.60", aErrorChar(), aSetDoctypeQuirks(), aEmitToken(ttStartTag)))
	ts = append(ts, t(stAfterDoctypePublicKeyword, ccEOF, stData, "13.2.5.60", aEmitToken(ttStartTag)))
	adpkChars := []html.CharClass{ccTab, ccLF, ccFF, ccCR, ccSpace, ccLower, ccUpper, ccNull, ccOther}
	ts = append(ts, tList(stAfterDoctypePublicKeyword, adpkChars, stBogusDoctype, "13.2.5.60", aErrorChar(), aSetDoctypeQuirks())...)

	// ── BOGUS DOCTYPE (§13.2.5.70) ──
	ts = append(ts, t(stBogusDoctype, ccGT, stData, "13.2.5.70", aEmitToken(ttStartTag)))
	ts = append(ts, t(stBogusDoctype, ccEOF, stData, "13.2.5.70", aEmitToken(ttStartTag)))
	bdChars := []html.CharClass{ccTab, ccLF, ccFF, ccCR, ccSpace, ccQuot, ccApos, ccHyphen, ccSlash, ccAmp, ccLT, ccDigit, ccUpper, ccLower, ccEQ, ccQuest, ccSemi, ccBang, ccSqClose, ccPct, ccNull, ccOther}
	ts = append(ts, tList(stBogusDoctype, bdChars, stBogusDoctype, "13.2.5.70")...)

	// ── Placeholder DOCTYPE sub-states (Phase 4) ──
	placeholderStates := []html.TokenizerState{
		stBeforeDoctypePublicID, stDoctypePublicIDDouble, stDoctypePublicIDSingle,
		stAfterDoctypePublicID, stBetweenDoctypePublicSystem, stAfterDoctypeSystemKeyword,
		stBeforeDoctypeSystemID, stDoctypeSystemIDDouble, stDoctypeSystemIDSingle,
	}
	for _, st := range placeholderStates {
		ts = append(ts, t(st, ccGT, stData, "13.2.5", aErrorChar(), aSetDoctypeQuirks(), aEmitToken(ttStartTag)))
		ts = append(ts, t(st, ccEOF, stData, "13.2.5", aEmitToken(ttStartTag)))
		psChars := []html.CharClass{ccTab, ccLF, ccFF, ccCR, ccSpace, ccLower, ccUpper, ccNull, ccOther}
		ts = append(ts, tList(st, psChars, st, "13.2.5")...)
	}

	return ts
}

// whitespace returns the five whitespace char classes.
func whitespace() []html.CharClass {
	return []html.CharClass{ccTab, ccLF, ccFF, ccCR, ccSpace}
}
