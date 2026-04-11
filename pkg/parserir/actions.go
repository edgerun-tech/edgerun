package parserir

import "edgerun-reference-core/gen/go/edgerun/v0/html"

func aEmitChar() *html.TransitionAction {
	return &html.TransitionAction{Action: &html.TransitionAction_EmitCurrentChar{EmitCurrentChar: true}}
}
func aEmitEof() *html.TransitionAction {
	return &html.TransitionAction{Action: &html.TransitionAction_EmitEofToken{EmitEofToken: true}}
}
func aEmitReplacement() *html.TransitionAction {
	return &html.TransitionAction{Action: &html.TransitionAction_EmitReplacementChar{EmitReplacementChar: true}}
}
func aStartTag() *html.TransitionAction {
	return &html.TransitionAction{Action: &html.TransitionAction_CreateStartTagToken{CreateStartTagToken: true}}
}
func aEndTag() *html.TransitionAction {
	return &html.TransitionAction{Action: &html.TransitionAction_CreateEndTagToken{CreateEndTagToken: true}}
}
func aAppendTagName() *html.TransitionAction {
	return &html.TransitionAction{Action: &html.TransitionAction_AppendCurrentToTagName{AppendCurrentToTagName: true}}
}
func aAppendAttrName() *html.TransitionAction {
	return &html.TransitionAction{Action: &html.TransitionAction_AppendCurrentToAttrName{AppendCurrentToAttrName: true}}
}
func aAppendAttrValue() *html.TransitionAction {
	return &html.TransitionAction{Action: &html.TransitionAction_AppendCurrentToAttrValue{AppendCurrentToAttrValue: true}}
}
func aEmitToken(tt html.TokenType) *html.TransitionAction {
	return &html.TransitionAction{Action: &html.TransitionAction_EmitTokenType{EmitTokenType: tt}}
}
func aSetSelfClosing() *html.TransitionAction {
	return &html.TransitionAction{Action: &html.TransitionAction_SetSelfClosingFlag{SetSelfClosingFlag: true}}
}
func aErrorChar() *html.TransitionAction {
	return &html.TransitionAction{Action: &html.TransitionAction_ErrorUnexpectedChar{ErrorUnexpectedChar: true}}
}
func aErrorNull() *html.TransitionAction {
	return &html.TransitionAction{Action: &html.TransitionAction_ErrorUnexpectedNull{ErrorUnexpectedNull: true}}
}
func aSwitchState(s html.TokenizerState) *html.TransitionAction {
	return &html.TransitionAction{Action: &html.TransitionAction_SwitchTokenizerState{SwitchTokenizerState: s}}
}
func aSetReturnState(s html.TokenizerState) *html.TransitionAction {
	return &html.TransitionAction{Action: &html.TransitionAction_SetReturnState{SetReturnState: s}}
}
func aReconsume() *html.TransitionAction {
	return &html.TransitionAction{Action: &html.TransitionAction_Reconsume{Reconsume: true}}
}
func aFlushCharRef() *html.TransitionAction {
	return &html.TransitionAction{Action: &html.TransitionAction_FlushCharRef{FlushCharRef: true}}
}
func aAppendTokenData() *html.TransitionAction {
	return &html.TransitionAction{Action: &html.TransitionAction_AppendCurrentToTokenData{AppendCurrentToTokenData: true}}
}
func aAppendTempBuffer() *html.TransitionAction {
	return &html.TransitionAction{Action: &html.TransitionAction_AppendCurrentToTempBuffer{AppendCurrentToTempBuffer: true}}
}
func aCheckAppropriateEndTag() *html.TransitionAction {
	return &html.TransitionAction{Action: &html.TransitionAction_CheckAppropriateEndTag{CheckAppropriateEndTag: true}}
}
func aCheckTempBufferIsScript() *html.TransitionAction {
	return &html.TransitionAction{Action: &html.TransitionAction_CheckTempBufferIsScript{CheckTempBufferIsScript: true}}
}
func aCreateComment() *html.TransitionAction {
	return &html.TransitionAction{Action: &html.TransitionAction_CreateCommentToken{CreateCommentToken: true}}
}
func aCreateDoctype() *html.TransitionAction {
	return &html.TransitionAction{Action: &html.TransitionAction_CreateDoctypeToken{CreateDoctypeToken: true}}
}
func aSetDoctypeQuirks() *html.TransitionAction {
	return &html.TransitionAction{Action: &html.TransitionAction_SetDoctypeForceQuirks{SetDoctypeForceQuirks: true}}
}
func aErrorMissingSemi() *html.TransitionAction {
	return &html.TransitionAction{Action: &html.TransitionAction_ErrorMissingSemicolon{ErrorMissingSemicolon: true}}
}

// t is a short constructor for StateTransition.
func t(from html.TokenizerState, cc html.CharClass, to html.TokenizerState, spec string, actions ...*html.TransitionAction) *html.StateTransition {
	return &html.StateTransition{
		CurrentState: from, CharClass: cc, NextState: to,
		Actions: actions, SpecSection: spec,
	}
}

// tList emits one transition per charClass in ccs (same from→to, same actions).
func tList(from html.TokenizerState, ccs []html.CharClass, to html.TokenizerState, spec string, actions ...*html.TransitionAction) []*html.StateTransition {
	out := make([]*html.StateTransition, 0, len(ccs))
	for _, cc := range ccs {
		out = append(out, t(from, cc, to, spec, actions...))
	}
	return out
}
