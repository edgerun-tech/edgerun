package parserir

import "edgerun-reference-core/gen/go/edgerun/v0/html"

// TreeRules builds the complete tree builder rule set for all insertion modes.
func TreeRules() []*html.TreeRule {
	var rules []*html.TreeRule

	// ── INITIAL mode (§13.2.6.4.1) ──
	// DOCTYPE → append, switch to BEFORE_HTML
	rules = append(rules, &html.TreeRule{
		Mode:          initial,
		Trigger:       &html.TokenTrigger{Trigger: &html.TokenTrigger_TokenType{TokenType: html.TokenType_TOKEN_TYPE_DOCTYPE}},
		Actions:       []html.TreeAction{html.TreeAction_TREE_ACTION_APPEND_DOCTYPE},
		NextMode:      beforeHTML,
		SpecParagraph: "13.2.6.4.1",
	})
	// Comment
	rules = append(rules, &html.TreeRule{
		Mode:          initial,
		Trigger:       &html.TokenTrigger{Trigger: &html.TokenTrigger_TokenType{TokenType: html.TokenType_TOKEN_TYPE_COMMENT}},
		Actions:       []html.TreeAction{html.TreeAction_TREE_ACTION_APPEND_COMMENT},
		NextMode:      initial,
		SpecParagraph: "13.2.6.4.1",
	})
	// EOF
	rules = append(rules, &html.TreeRule{
		Mode:          initial,
		Trigger:       &html.TokenTrigger{Trigger: &html.TokenTrigger_TokenType{TokenType: html.TokenType_TOKEN_TYPE_EOF}},
		NextMode:      afterAfterBody,
		SpecParagraph: "13.2.6.4.1",
	})
	// Start tag (not DOCTYPE) → reprocess in BEFORE_HTML (implicitly create html)
	rules = append(rules, &html.TreeRule{
		Mode:    initial,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_AnyStartTag{AnyStartTag: true}},
		Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_PARSE_ERROR},
		NextMode: beforeHTML,
		Otherwise: true,
		SpecParagraph: "13.2.6.4.1",
	})
	rules = append(rules, &html.TreeRule{
		Mode:    initial,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_CharacterToken{CharacterToken: true}},
		Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_PARSE_ERROR},
		NextMode: beforeHTML,
		Otherwise: true,
		SpecParagraph: "13.2.6.4.1",
	})

	// ── BEFORE_HTML mode (§13.2.6.4.2) ──
	// "html" start tag → insert, go to BEFORE_HEAD
	rules = append(rules, &html.TreeRule{
		Mode:    beforeHTML,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_StartTag{StartTag: "html"}},
		Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_INSERT},
		NextMode: beforeHead,
		SpecParagraph: "13.2.6.4.2",
	})
	// Any other start tag → implicitly create html, reprocess in BEFORE_HEAD
	rules = append(rules, &html.TreeRule{
		Mode:    beforeHTML,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_AnyStartTag{AnyStartTag: true}},
		Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_PARSE_ERROR},
		NextMode: beforeHead,
		Otherwise: true,
		SpecParagraph: "13.2.6.4.2",
	})
	rules = append(rules, &html.TreeRule{
		Mode:    beforeHTML,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_CharacterToken{CharacterToken: true}},
		Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_PARSE_ERROR},
		NextMode: beforeHead,
		Otherwise: true,
		SpecParagraph: "13.2.6.4.2",
	})
	rules = append(rules, &html.TreeRule{
		Mode:    beforeHTML,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_TokenType{TokenType: html.TokenType_TOKEN_TYPE_COMMENT}},
		Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_APPEND_COMMENT},
		NextMode: beforeHTML,
		SpecParagraph: "13.2.6.4.2",
	})
	rules = append(rules, &html.TreeRule{
		Mode:    beforeHTML,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_TokenType{TokenType: html.TokenType_TOKEN_TYPE_DOCTYPE}},
		Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_PARSE_ERROR, html.TreeAction_TREE_ACTION_IGNORE},
		NextMode: beforeHTML,
		SpecParagraph: "13.2.6.4.2",
	})
	rules = append(rules, &html.TreeRule{
		Mode:    beforeHTML,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_TokenType{TokenType: html.TokenType_TOKEN_TYPE_EOF}},
		NextMode: afterAfterBody,
		SpecParagraph: "13.2.6.4.2",
	})

	// ── BEFORE_HEAD mode (§13.2.6.4.3) ──
	rules = append(rules, &html.TreeRule{
		Mode:    beforeHead,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_CharacterToken{CharacterToken: true}},
		Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_PARSE_ERROR, html.TreeAction_TREE_ACTION_IGNORE},
		NextMode: beforeHead,
		SpecParagraph: "13.2.6.4.3",
	})
	rules = append(rules, &html.TreeRule{
		Mode:    beforeHead,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_StartTag{StartTag: "html"}},
		Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_INSERT},
		NextMode: beforeHead,
		SpecParagraph: "13.2.6.4.3",
	})
	rules = append(rules, &html.TreeRule{
		Mode:    beforeHead,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_StartTag{StartTag: "head"}},
		Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_INSERT},
		NextMode: inHead,
		SpecParagraph: "13.2.6.4.3",
	})
	rules = append(rules, &html.TreeRule{
		Mode:    beforeHead,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_AnyStartTag{AnyStartTag: true}},
		Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_PARSE_ERROR},
		NextMode: inHead,
		Otherwise: true,
		SpecParagraph: "13.2.6.4.3",
	})
	rules = append(rules, &html.TreeRule{
		Mode:    beforeHead,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_EndTag{EndTag: "head"}},
		Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_POP_UNTIL},
		PopUntilTag: "head",
		NextMode: afterHead,
		SpecParagraph: "13.2.6.4.3",
	})
	rules = append(rules, &html.TreeRule{
		Mode:    beforeHead,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_TokenType{TokenType: html.TokenType_TOKEN_TYPE_COMMENT}},
		Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_APPEND_COMMENT},
		NextMode: beforeHead,
		SpecParagraph: "13.2.6.4.3",
	})
	rules = append(rules, &html.TreeRule{
		Mode:    beforeHead,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_TokenType{TokenType: html.TokenType_TOKEN_TYPE_DOCTYPE}},
		Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_PARSE_ERROR, html.TreeAction_TREE_ACTION_IGNORE},
		NextMode: beforeHead,
		SpecParagraph: "13.2.6.4.3",
	})
	rules = append(rules, &html.TreeRule{
		Mode:    beforeHead,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_TokenType{TokenType: html.TokenType_TOKEN_TYPE_EOF}},
		NextMode: afterAfterBody,
		SpecParagraph: "13.2.6.4.3",
	})

	// ── IN_HEAD mode (§13.2.6.4.4) ──
	rules = append(rules, &html.TreeRule{
		Mode:    inHead,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_StartTag{StartTag: "html"}},
		Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_INSERT},
		NextMode: inBody,
		SpecParagraph: "13.2.6.4.4",
	})
	rules = append(rules, &html.TreeRule{
		Mode:    inHead,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_StartTag{StartTag: "head"}},
		Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_PARSE_ERROR, html.TreeAction_TREE_ACTION_IGNORE},
		NextMode: inHead,
		SpecParagraph: "13.2.6.4.4",
	})
	// Elements that belong in <head>: base, basefont, bgsound, link, meta
	headOnly := []string{"base", "basefont", "bgsound", "link", "meta"}
	for _, tag := range headOnly {
		rules = append(rules, &html.TreeRule{
			Mode:    inHead,
			Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_StartTag{StartTag: tag}},
			Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_INSERT},
			NextMode: inHead,
			SpecParagraph: "13.2.6.4.4",
		})
	}
	// title, noscript, style, script, noscript, noframes
	for _, tag := range []string{"title", "noscript", "style", "noframes"} {
		rules = append(rules, &html.TreeRule{
			Mode:    inHead,
			Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_StartTag{StartTag: tag}},
			Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_INSERT, treeActionForRawText(tag)},
			NextMode: textMode,
			SpecParagraph: "13.2.6.4.4",
		})
	}
	// script
	rules = append(rules, &html.TreeRule{
		Mode:    inHead,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_StartTag{StartTag: "script"}},
		Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_INSERT, html.TreeAction_TREE_ACTION_SWITCH_TO_SCRIPT_DATA},
		NextMode: textMode,
		SpecParagraph: "13.2.6.4.4",
	})
	// </head>
	rules = append(rules, &html.TreeRule{
		Mode:    inHead,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_EndTag{EndTag: "head"}},
		Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_POP_UNTIL},
		PopUntilTag: "head",
		NextMode: afterHead,
		SpecParagraph: "13.2.6.4.4",
	})
	// </body>, </html>, </br> — special handling
	rules = append(rules, &html.TreeRule{
		Mode:    inHead,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_EndTag{EndTag: "body"}},
		Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_PARSE_ERROR, html.TreeAction_TREE_ACTION_IGNORE},
		NextMode: inHead,
		SpecParagraph: "13.2.6.4.4",
	})
	rules = append(rules, &html.TreeRule{
		Mode:    inHead,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_EndTag{EndTag: "html"}},
		Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_PARSE_ERROR, html.TreeAction_TREE_ACTION_IGNORE},
		NextMode: inHead,
		SpecParagraph: "13.2.6.4.4",
	})
	// Anything else → </head> implied, reprocess
	rules = append(rules, &html.TreeRule{
		Mode:    inHead,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_AnyStartTag{AnyStartTag: true}},
		Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_POP_UNTIL},
		PopUntilTag: "head",
		NextMode: afterHead,
		Otherwise: true,
		SpecParagraph: "13.2.6.4.4",
	})
	rules = append(rules, &html.TreeRule{
		Mode:    inHead,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_CharacterToken{CharacterToken: true}},
		Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_PARSE_ERROR, html.TreeAction_TREE_ACTION_IGNORE},
		NextMode: inHead,
		SpecParagraph: "13.2.6.4.4",
	})
	rules = append(rules, &html.TreeRule{
		Mode:    inHead,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_TokenType{TokenType: html.TokenType_TOKEN_TYPE_COMMENT}},
		Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_APPEND_COMMENT},
		NextMode: inHead,
		SpecParagraph: "13.2.6.4.4",
	})
	rules = append(rules, &html.TreeRule{
		Mode:    inHead,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_TokenType{TokenType: html.TokenType_TOKEN_TYPE_DOCTYPE}},
		Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_PARSE_ERROR, html.TreeAction_TREE_ACTION_IGNORE},
		NextMode: inHead,
		SpecParagraph: "13.2.6.4.4",
	})
	rules = append(rules, &html.TreeRule{
		Mode:    inHead,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_TokenType{TokenType: html.TokenType_TOKEN_TYPE_EOF}},
		NextMode: afterAfterBody,
		SpecParagraph: "13.2.6.4.4",
	})

	// ── AFTER_HEAD mode (§13.2.6.4.5) ──
	rules = append(rules, &html.TreeRule{
		Mode:    afterHead,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_CharacterToken{CharacterToken: true}},
		Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_APPEND_CHARACTER},
		NextMode: inBody,
		SpecParagraph: "13.2.6.4.5",
	})
	rules = append(rules, &html.TreeRule{
		Mode:    afterHead,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_StartTag{StartTag: "html"}},
		Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_INSERT},
		NextMode: inBody,
		SpecParagraph: "13.2.6.4.5",
	})
	rules = append(rules, &html.TreeRule{
		Mode:    afterHead,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_StartTag{StartTag: "body"}},
		Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_INSERT},
		NextMode: inBody,
		SpecParagraph: "13.2.6.4.5",
	})
	rules = append(rules, &html.TreeRule{
		Mode:    afterHead,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_StartTag{StartTag: "frameset"}},
		Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_INSERT},
		NextMode: inFrameset,
		SpecParagraph: "13.2.6.4.5",
	})
	rules = append(rules, &html.TreeRule{
		Mode:    afterHead,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_StartTag{StartTag: "head"}},
		Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_PARSE_ERROR, html.TreeAction_TREE_ACTION_IGNORE},
		NextMode: inBody,
		SpecParagraph: "13.2.6.4.5",
	})
	// Anything else → <body> implied, reprocess
	rules = append(rules, &html.TreeRule{
		Mode:    afterHead,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_AnyStartTag{AnyStartTag: true}},
		Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_PARSE_ERROR},
		NextMode: inBody,
		Otherwise: true,
		SpecParagraph: "13.2.6.4.5",
	})
	rules = append(rules, &html.TreeRule{
		Mode:    afterHead,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_EndTag{EndTag: "body"}},
		Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_PARSE_ERROR, html.TreeAction_TREE_ACTION_IGNORE},
		NextMode: inBody,
		SpecParagraph: "13.2.6.4.5",
	})
	rules = append(rules, &html.TreeRule{
		Mode:    afterHead,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_EndTag{EndTag: "html"}},
		Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_PARSE_ERROR, html.TreeAction_TREE_ACTION_IGNORE},
		NextMode: inBody,
		SpecParagraph: "13.2.6.4.5",
	})
	rules = append(rules, &html.TreeRule{
		Mode:    afterHead,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_TokenType{TokenType: html.TokenType_TOKEN_TYPE_COMMENT}},
		Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_APPEND_COMMENT},
		NextMode: inBody,
		SpecParagraph: "13.2.6.4.5",
	})
	rules = append(rules, &html.TreeRule{
		Mode:    afterHead,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_TokenType{TokenType: html.TokenType_TOKEN_TYPE_DOCTYPE}},
		Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_PARSE_ERROR, html.TreeAction_TREE_ACTION_IGNORE},
		NextMode: inBody,
		SpecParagraph: "13.2.6.4.5",
	})
	rules = append(rules, &html.TreeRule{
		Mode:    afterHead,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_TokenType{TokenType: html.TokenType_TOKEN_TYPE_EOF}},
		NextMode: afterAfterBody,
		SpecParagraph: "13.2.6.4.5",
	})

	// ── IN_BODY mode (§13.2.6.4.16) ──
	// "html" start tag — reparent
	rules = append(rules, &html.TreeRule{
		Mode:    inBody,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_StartTag{StartTag: "html"}},
		Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_PARSE_ERROR},
		NextMode: inBody,
		SpecParagraph: "13.2.6.4.16",
	})
	// "body" start tag — ignore
	rules = append(rules, &html.TreeRule{
		Mode:    inBody,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_StartTag{StartTag: "body"}},
		Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_PARSE_ERROR, html.TreeAction_TREE_ACTION_IGNORE},
		NextMode: inBody,
		SpecParagraph: "13.2.6.4.16",
	})
	// "frameset" start tag — parse error, remove body, switch to frameset
	rules = append(rules, &html.TreeRule{
		Mode:    inBody,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_StartTag{StartTag: "frameset"}},
		Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_PARSE_ERROR, html.TreeAction_TREE_ACTION_POP_ALL},
		NextMode: inFrameset,
		SpecParagraph: "13.2.6.4.16",
	})
	// "base", "basefont", "bgsound", "link", "meta", "noframes" — ignore in body
	for _, tag := range []string{"base", "basefont", "bgsound", "link", "meta", "noframes"} {
		rules = append(rules, &html.TreeRule{
			Mode:    inBody,
			Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_StartTag{StartTag: tag}},
			Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_PARSE_ERROR, html.TreeAction_TREE_ACTION_IGNORE},
			NextMode: inBody,
			SpecParagraph: "13.2.6.4.16",
		})
	}
	// Raw text elements
	rawText := []struct{ tag string; action html.TreeAction }{
		{"script", html.TreeAction_TREE_ACTION_SWITCH_TO_SCRIPT_DATA},
		{"style", html.TreeAction_TREE_ACTION_SWITCH_TO_RAWTEXT},
		{"textarea", html.TreeAction_TREE_ACTION_SWITCH_TO_RCDATA},
		{"title", html.TreeAction_TREE_ACTION_SWITCH_TO_RCDATA},
	}
	for _, rt := range rawText {
		rules = append(rules, &html.TreeRule{
			Mode:    inBody,
			Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_StartTag{StartTag: rt.tag}},
			Actions: []html.TreeAction{
				html.TreeAction_TREE_ACTION_PARSE_ERROR,
				html.TreeAction_TREE_ACTION_INSERT,
				rt.action,
			},
			NextMode:      textMode,
			SpecParagraph: "13.2.6.4.16",
		})
	}

	// ── Implicit tag closing rules (§13.2.6.4.16) ──
	// Tags that close a <p> element (if one is in button scope)
	closesP := []string{
		"address", "article", "aside", "blockquote", "details", "dialog",
		"div", "dl", "fieldset", "figcaption", "figure", "footer", "form",
		"h1", "h2", "h3", "h4", "h5", "h6", "header", "hgroup", "hr",
		"main", "menu", "nav", "ol", "p", "pre", "section", "table", "ul",
	}
	for _, tag := range closesP {
		rules = append(rules, &html.TreeRule{
			Mode:    inBody,
			Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_StartTag{StartTag: tag}},
			Actions: []html.TreeAction{
				html.TreeAction_TREE_ACTION_POP_UNTIL,
				html.TreeAction_TREE_ACTION_INSERT,
			},
			PopUntilTag:   "p",
			NextMode:      inBody,
			SpecParagraph: "13.2.6.4.16",
			Condition:     &html.StackCondition{HasInButtonScope: []string{"p"}},
		})
	}
	// <li> closes any open <li> (if in scope)
	rules = append(rules, &html.TreeRule{
		Mode:    inBody,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_StartTag{StartTag: "li"}},
		Actions: []html.TreeAction{
			html.TreeAction_TREE_ACTION_POP_UNTIL,
			html.TreeAction_TREE_ACTION_INSERT,
		},
		PopUntilTag:   "li",
		NextMode:      inBody,
		SpecParagraph: "13.2.6.4.16",
		Condition:     &html.StackCondition{HasInListScope: []string{"li"}},
	})
	// <dt>/<dd> close each other
	rules = append(rules, &html.TreeRule{
		Mode:    inBody,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_StartTag{StartTag: "dt"}},
		Actions: []html.TreeAction{
			html.TreeAction_TREE_ACTION_POP_UNTIL,
			html.TreeAction_TREE_ACTION_INSERT,
		},
		PopUntilTag:   "dd",
		NextMode:      inBody,
		SpecParagraph: "13.2.6.4.16",
		Condition:     &html.StackCondition{HasInScope: []string{"dd"}},
	})
	rules = append(rules, &html.TreeRule{
		Mode:    inBody,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_StartTag{StartTag: "dd"}},
		Actions: []html.TreeAction{
			html.TreeAction_TREE_ACTION_POP_UNTIL,
			html.TreeAction_TREE_ACTION_INSERT,
		},
		PopUntilTag:   "dt",
		NextMode:      inBody,
		SpecParagraph: "13.2.6.4.16",
		Condition:     &html.StackCondition{HasInScope: []string{"dt"}},
	})
	// Headings auto-close each other
	headings := []string{"h1", "h2", "h3", "h4", "h5", "h6"}
	for _, tag := range headings {
		for _, other := range headings {
			if other == tag {
				continue // already covered by the p-closing rule above
			}
			rules = append(rules, &html.TreeRule{
				Mode:    inBody,
				Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_StartTag{StartTag: tag}},
				Actions: []html.TreeAction{
					html.TreeAction_TREE_ACTION_POP_UNTIL,
					html.TreeAction_TREE_ACTION_INSERT,
				},
				PopUntilTag:   other,
				NextMode:      inBody,
				SpecParagraph: "13.2.6.4.16",
				Condition:     &html.StackCondition{HasInScope: []string{other}},
			})
		}
	}

	// Normal start tags (no implicit closing triggered)
	// Note: tags that trigger implicit closing (div, p, headings, li, dt, dd, table, form, section, article, aside, header, footer, nav, main, ul, ol, dl, dt, dd) already have rules above — don't duplicate
	startTags := []string{
		"span", "a", "b", "i", "em", "strong", "u", "s", "small", "big", "sub", "sup",
		"code", "pre", "blockquote", "q", "cite", "abbr", "dfn",
		"br", "img", "hr", "input",
	}
	for _, tag := range startTags {
		rules = append(rules, &html.TreeRule{
			Mode:    inBody,
			Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_StartTag{StartTag: tag}},
			Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_INSERT},
			NextMode: inBody,
			SpecParagraph: "13.2.6.4.16",
		})
	}
	// <div> — insert fallback (when p not in scope)
	rules = append(rules, &html.TreeRule{Mode: inBody, Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_StartTag{StartTag: "div"}}, Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_INSERT}, NextMode: inBody, SpecParagraph: "13.2.6.4.16"})
	// <p> — insert fallback
	rules = append(rules, &html.TreeRule{Mode: inBody, Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_StartTag{StartTag: "p"}}, Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_INSERT}, NextMode: inBody, SpecParagraph: "13.2.6.4.16"})
	// <li> — insert fallback
	rules = append(rules, &html.TreeRule{Mode: inBody, Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_StartTag{StartTag: "li"}}, Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_INSERT}, NextMode: inBody, SpecParagraph: "13.2.6.4.16"})
	// heading fallbacks
	for _, tag := range headings {
		rules = append(rules, &html.TreeRule{Mode: inBody, Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_StartTag{StartTag: tag}}, Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_INSERT}, NextMode: inBody, SpecParagraph: "13.2.6.4.16"})
	}
	// dt/dd fallbacks
	rules = append(rules, &html.TreeRule{Mode: inBody, Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_StartTag{StartTag: "dt"}}, Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_INSERT}, NextMode: inBody, SpecParagraph: "13.2.6.4.16"})
	rules = append(rules, &html.TreeRule{Mode: inBody, Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_StartTag{StartTag: "dd"}}, Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_INSERT}, NextMode: inBody, SpecParagraph: "13.2.6.4.16"})
	// Any other start tag
	rules = append(rules, &html.TreeRule{
		Mode:    inBody,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_AnyStartTag{AnyStartTag: true}},
		Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_INSERT},
		NextMode: inBody,
		SpecParagraph: "13.2.6.4.16",
	})

	// End tags
	endTags := []string{"body", "html", "div", "span", "p", "a", "h1", "h2", "h3", "h4", "h5", "h6",
		"ul", "ol", "li", "table", "tr", "td", "th", "thead", "tbody", "tfoot",
		"form", "section", "article", "aside", "header", "footer", "nav", "main",
		"b", "i", "em", "strong", "u", "s", "small", "big", "sub", "sup",
		"code", "pre", "blockquote", "q", "cite", "abbr", "dfn",
		"dl", "dt", "dd", "script", "style", "textarea", "title"}
	for _, tag := range endTags {
		rules = append(rules, &html.TreeRule{
			Mode:    inBody,
			Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_EndTag{EndTag: tag}},
			Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_POP_UNTIL},
			PopUntilTag: tag,
			NextMode:    inBody,
			SpecParagraph: "13.2.6.4.16",
		})
	}
	rules = append(rules, &html.TreeRule{
		Mode:    inBody,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_AnyEndTag{AnyEndTag: true}},
		Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_POP_UNTIL},
		NextMode: inBody,
		SpecParagraph: "13.2.6.4.16",
	})

	// Character
	rules = append(rules, &html.TreeRule{
		Mode:    inBody,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_CharacterToken{CharacterToken: true}},
		Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_APPEND_CHARACTER},
		NextMode: inBody,
		SpecParagraph: "13.2.6.4.16",
	})

	// Comment
	rules = append(rules, &html.TreeRule{
		Mode:    inBody,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_TokenType{TokenType: html.TokenType_TOKEN_TYPE_COMMENT}},
		Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_APPEND_COMMENT},
		NextMode: inBody,
		SpecParagraph: "13.2.6.4.16",
	})

	// DOCTYPE in body
	rules = append(rules, &html.TreeRule{
		Mode:    inBody,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_TokenType{TokenType: html.TokenType_TOKEN_TYPE_DOCTYPE}},
		Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_PARSE_ERROR, html.TreeAction_TREE_ACTION_IGNORE},
		NextMode: inBody,
		SpecParagraph: "13.2.6.4.16",
	})

	// EOF
	rules = append(rules, &html.TreeRule{
		Mode:    inBody,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_TokenType{TokenType: html.TokenType_TOKEN_TYPE_EOF}},
		NextMode: afterBody,
		SpecParagraph: "13.2.6.4.16",
	})

	// ── TEXT mode (§13.2.6.4.17) ──
	rules = append(rules, &html.TreeRule{
		Mode:    textMode,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_CharacterToken{CharacterToken: true}},
		Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_APPEND_CHARACTER},
		NextMode: textMode,
		SpecParagraph: "13.2.6.4.17",
	})
	for _, tag := range []string{"script", "style", "textarea", "title", "noscript", "noframes"} {
		rules = append(rules, &html.TreeRule{
			Mode:    textMode,
			Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_EndTag{EndTag: tag}},
			Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_POP_UNTIL},
			PopUntilTag: tag,
			NextMode: afterHead,
			SpecParagraph: "13.2.6.4.17",
		})
	}
	rules = append(rules, &html.TreeRule{
		Mode:    textMode,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_TokenType{TokenType: html.TokenType_TOKEN_TYPE_EOF}},
		NextMode: afterAfterBody,
		SpecParagraph: "13.2.6.4.17",
	})

	// ── AFTER_BODY mode (§13.2.6.4.18) ──
	rules = append(rules, &html.TreeRule{
		Mode:    afterBody,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_CharacterToken{CharacterToken: true}},
		Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_REPROCESS},
		NextMode: inBody,
		SpecParagraph: "13.2.6.4.18",
	})
	rules = append(rules, &html.TreeRule{
		Mode:    afterBody,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_TokenType{TokenType: html.TokenType_TOKEN_TYPE_COMMENT}},
		Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_APPEND_COMMENT},
		NextMode: afterBody,
		SpecParagraph: "13.2.6.4.18",
	})
	rules = append(rules, &html.TreeRule{
		Mode:    afterBody,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_TokenType{TokenType: html.TokenType_TOKEN_TYPE_EOF}},
		NextMode: afterAfterBody,
		SpecParagraph: "13.2.6.4.18",
	})

	// ── IN_FRAMESET mode (§13.2.6.4.19) ──
	rules = append(rules, &html.TreeRule{
		Mode:    inFrameset,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_StartTag{StartTag: "frameset"}},
		Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_INSERT},
		NextMode: inFrameset,
		SpecParagraph: "13.2.6.4.19",
	})
	rules = append(rules, &html.TreeRule{
		Mode:    inFrameset,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_EndTag{EndTag: "frameset"}},
		Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_POP_UNTIL},
		PopUntilTag: "frameset",
		NextMode: afterFrameset,
		SpecParagraph: "13.2.6.4.19",
	})
	rules = append(rules, &html.TreeRule{
		Mode:    inFrameset,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_StartTag{StartTag: "html"}},
		Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_INSERT},
		NextMode: inFrameset,
		SpecParagraph: "13.2.6.4.19",
	})
	rules = append(rules, &html.TreeRule{
		Mode:    inFrameset,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_TokenType{TokenType: html.TokenType_TOKEN_TYPE_EOF}},
		NextMode: afterAfterBody,
		SpecParagraph: "13.2.6.4.19",
	})

	// ── AFTER_FRAMESET mode (§13.2.6.4.20) ──
	rules = append(rules, &html.TreeRule{
		Mode:    afterFrameset,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_TokenType{TokenType: html.TokenType_TOKEN_TYPE_COMMENT}},
		Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_APPEND_COMMENT},
		NextMode: afterFrameset,
		SpecParagraph: "13.2.6.4.20",
	})
	rules = append(rules, &html.TreeRule{
		Mode:    afterFrameset,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_TokenType{TokenType: html.TokenType_TOKEN_TYPE_EOF}},
		NextMode: afterAfterFS,
		SpecParagraph: "13.2.6.4.20",
	})

	// ── AFTER_AFTER_BODY mode (§13.2.6.4.21) ──
	rules = append(rules, &html.TreeRule{
		Mode:    afterAfterBody,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_TokenType{TokenType: html.TokenType_TOKEN_TYPE_COMMENT}},
		Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_APPEND_COMMENT},
		NextMode: afterAfterBody,
		SpecParagraph: "13.2.6.4.21",
	})
	rules = append(rules, &html.TreeRule{
		Mode:    afterAfterBody,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_CharacterToken{CharacterToken: true}},
		Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_REPROCESS},
		NextMode: inBody,
		SpecParagraph: "13.2.6.4.21",
	})
	rules = append(rules, &html.TreeRule{
		Mode:    afterAfterBody,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_StartTag{StartTag: "html"}},
		Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_INSERT},
		NextMode: inBody,
		SpecParagraph: "13.2.6.4.21",
	})
	rules = append(rules, &html.TreeRule{
		Mode:    afterAfterBody,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_TokenType{TokenType: html.TokenType_TOKEN_TYPE_EOF}},
		NextMode: afterAfterBody,
		SpecParagraph: "13.2.6.4.21",
	})

	// ── AFTER_AFTER_FRAMESET mode (§13.2.6.4.22) ──
	rules = append(rules, &html.TreeRule{
		Mode:    afterAfterFS,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_TokenType{TokenType: html.TokenType_TOKEN_TYPE_COMMENT}},
		Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_APPEND_COMMENT},
		NextMode: afterAfterFS,
		SpecParagraph: "13.2.6.4.22",
	})
	rules = append(rules, &html.TreeRule{
		Mode:    afterAfterFS,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_TokenType{TokenType: html.TokenType_TOKEN_TYPE_EOF}},
		NextMode: afterAfterFS,
		SpecParagraph: "13.2.6.4.22",
	})

	// ── IN_TABLE mode (§13.2.6.4.9) ──
	// "caption" → insert, switch to IN_CAPTION
	rules = append(rules, &html.TreeRule{
		Mode:    inTable,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_StartTag{StartTag: "caption"}},
		Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_INSERT},
		NextMode: inCaption,
		SpecParagraph: "13.2.6.4.9",
	})
	// "colgroup" → insert, stay in IN_TABLE
	rules = append(rules, &html.TreeRule{
		Mode:    inTable,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_StartTag{StartTag: "colgroup"}},
		Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_INSERT},
		NextMode: inColumnGroup,
		SpecParagraph: "13.2.6.4.9",
	})
	// "tbody", "thead", "tfoot" → insert, switch to IN_TABLE_BODY
	for _, tag := range []string{"tbody", "thead", "tfoot"} {
		rules = append(rules, &html.TreeRule{
			Mode:    inTable,
			Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_StartTag{StartTag: tag}},
			Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_INSERT},
			NextMode: inTableBody,
			SpecParagraph: "13.2.6.4.9",
		})
	}
	// "col" → insert, stay in IN_TABLE
	rules = append(rules, &html.TreeRule{
		Mode:    inTable,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_StartTag{StartTag: "col"}},
		Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_INSERT},
		NextMode: inTable,
		SpecParagraph: "13.2.6.4.9",
	})
	// "tr" → foster parent, switch to IN_TABLE_BODY
	rules = append(rules, &html.TreeRule{
		Mode:    inTable,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_StartTag{StartTag: "tr"}},
		Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_INSERT_FOSTER},
		NextMode: inTableBody,
		SpecParagraph: "13.2.6.4.9",
	})
	// "td", "th" → foster parent, switch to IN_ROW
	for _, tag := range []string{"td", "th"} {
		rules = append(rules, &html.TreeRule{
			Mode:    inTable,
			Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_StartTag{StartTag: tag}},
			Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_INSERT_FOSTER},
			NextMode: inRow,
			SpecParagraph: "13.2.6.4.9",
		})
	}
	// "table" → parse error, ignore (can't nest tables)
	rules = append(rules, &html.TreeRule{
		Mode:    inTable,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_StartTag{StartTag: "table"}},
		Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_PARSE_ERROR, html.TreeAction_TREE_ACTION_IGNORE},
		NextMode: inTable,
		SpecParagraph: "13.2.6.4.9",
	})
	// "style", "script" → switch to head processing
	for _, tag := range []string{"style", "script"} {
		rules = append(rules, &html.TreeRule{
			Mode:    inTable,
			Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_StartTag{StartTag: tag}},
			Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_PARSE_ERROR, html.TreeAction_TREE_ACTION_INSERT, treeActionForRawText(tag)},
			NextMode: textMode,
			SpecParagraph: "13.2.6.4.9",
		})
	}
	// "input" (type=hidden) → insert, ignore
	rules = append(rules, &html.TreeRule{
		Mode:    inTable,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_StartTag{StartTag: "input"}},
		Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_PARSE_ERROR, html.TreeAction_TREE_ACTION_IGNORE},
		NextMode: inTable,
		SpecParagraph: "13.2.6.4.9",
	})
	// "form" → parse error, insert, stay
	rules = append(rules, &html.TreeRule{
		Mode:    inTable,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_StartTag{StartTag: "form"}},
		Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_PARSE_ERROR, html.TreeAction_TREE_ACTION_INSERT},
		NextMode: inTable,
		SpecParagraph: "13.2.6.4.9",
	})
	// Character → reprocess in IN_TABLE_TEXT (foster parent text)
	rules = append(rules, &html.TreeRule{
		Mode:    inTable,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_CharacterToken{CharacterToken: true}},
		Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_REPROCESS},
		NextMode: inTableText,
		SpecParagraph: "13.2.6.4.9",
	})
	// Comment → append
	rules = append(rules, &html.TreeRule{
		Mode:    inTable,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_TokenType{TokenType: html.TokenType_TOKEN_TYPE_COMMENT}},
		Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_APPEND_COMMENT},
		NextMode: inTable,
		SpecParagraph: "13.2.6.4.9",
	})
	// DOCTYPE → parse error, ignore
	rules = append(rules, &html.TreeRule{
		Mode:    inTable,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_TokenType{TokenType: html.TokenType_TOKEN_TYPE_DOCTYPE}},
		Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_PARSE_ERROR, html.TreeAction_TREE_ACTION_IGNORE},
		NextMode: inTable,
		SpecParagraph: "13.2.6.4.9",
	})
	// EndTag "table" → pop until table
	rules = append(rules, &html.TreeRule{
		Mode:    inTable,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_EndTag{EndTag: "table"}},
		Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_POP_UNTIL},
		PopUntilTag: "table",
		NextMode: afterBody,
		SpecParagraph: "13.2.6.4.9",
	})
	// Other end tags → parse error, ignore
	for _, tag := range []string{"body", "caption", "col", "colgroup", "html", "tbody", "td", "tfoot", "th", "thead", "tr"} {
		rules = append(rules, &html.TreeRule{
			Mode:    inTable,
			Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_EndTag{EndTag: tag}},
			Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_PARSE_ERROR, html.TreeAction_TREE_ACTION_IGNORE},
			NextMode: inTable,
			SpecParagraph: "13.2.6.4.9",
		})
	}
	// Any other start tag → foster parent, stay in IN_TABLE
	rules = append(rules, &html.TreeRule{
		Mode:    inTable,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_AnyStartTag{AnyStartTag: true}},
		Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_PARSE_ERROR, html.TreeAction_TREE_ACTION_INSERT_FOSTER},
		NextMode: inTable,
		SpecParagraph: "13.2.6.4.9",
	})
	// EOF → done
	rules = append(rules, &html.TreeRule{
		Mode:    inTable,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_TokenType{TokenType: html.TokenType_TOKEN_TYPE_EOF}},
		NextMode: afterAfterBody,
		SpecParagraph: "13.2.6.4.9",
	})

	// ── IN_TABLE_TEXT mode (§13.2.6.4.10) ──
	// Character → append to pending text (TODO: foster parent on flush)
	rules = append(rules, &html.TreeRule{
		Mode:    inTableText,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_CharacterToken{CharacterToken: true}},
		Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_APPEND_CHARACTER},
		NextMode: inTableText,
		SpecParagraph: "13.2.6.4.10",
	})
	// Whitespace → same
	rules = append(rules, &html.TreeRule{
		Mode:    inTableText,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_WhitespaceCharacter{WhitespaceCharacter: true}},
		Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_APPEND_CHARACTER},
		NextMode: inTableText,
		SpecParagraph: "13.2.6.4.10",
	})
	// Anything else → reprocess in IN_TABLE
	rules = append(rules, &html.TreeRule{
		Mode:    inTableText,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_AnyStartTag{AnyStartTag: true}},
		Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_REPROCESS},
		NextMode: inTable,
		SpecParagraph: "13.2.6.4.10",
	})
	rules = append(rules, &html.TreeRule{
		Mode:    inTableText,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_EndTag{EndTag: "table"}},
		Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_REPROCESS},
		NextMode: inTable,
		SpecParagraph: "13.2.6.4.10",
	})
	rules = append(rules, &html.TreeRule{
		Mode:    inTableText,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_CharacterToken{CharacterToken: true}},
		Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_REPROCESS},
		NextMode: inBody,
		SpecParagraph: "13.2.6.4.10",
	})
	rules = append(rules, &html.TreeRule{
		Mode:    inTableText,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_TokenType{TokenType: html.TokenType_TOKEN_TYPE_EOF}},
		NextMode: afterAfterBody,
		SpecParagraph: "13.2.6.4.10",
	})

	// ── IN_TABLE_BODY mode (§13.2.6.4.12) ──
	// "tr" → foster parent, switch to IN_ROW
	rules = append(rules, &html.TreeRule{
		Mode:    inTableBody,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_StartTag{StartTag: "tr"}},
		Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_INSERT_FOSTER},
		NextMode: inRow,
		SpecParagraph: "13.2.6.4.12",
	})
	// "td", "th" → foster parent, switch to IN_ROW
	for _, tag := range []string{"td", "th"} {
		rules = append(rules, &html.TreeRule{
			Mode:    inTableBody,
			Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_StartTag{StartTag: tag}},
			Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_PARSE_ERROR, html.TreeAction_TREE_ACTION_INSERT_FOSTER},
			NextMode: inRow,
			SpecParagraph: "13.2.6.4.12",
		})
	}
	// "caption", "col", "colgroup", "tbody", "tfoot", "thead" → parse error, pop until table body group
	for _, tag := range []string{"caption", "col", "colgroup", "tbody", "tfoot", "thead"} {
		rules = append(rules, &html.TreeRule{
			Mode:    inTableBody,
			Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_StartTag{StartTag: tag}},
			Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_PARSE_ERROR, html.TreeAction_TREE_ACTION_IGNORE},
			NextMode: inTable,
			SpecParagraph: "13.2.6.4.12",
		})
	}
	// EndTag "tbody", "thead", "tfoot" → pop until matching
	for _, tag := range []string{"tbody", "thead", "tfoot"} {
		rules = append(rules, &html.TreeRule{
			Mode:    inTableBody,
			Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_EndTag{EndTag: tag}},
			Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_POP_UNTIL},
			PopUntilTag: tag,
			NextMode: inTable,
			SpecParagraph: "13.2.6.4.12",
		})
	}
	// "table" → parse error, pop until table
	rules = append(rules, &html.TreeRule{
		Mode:    inTableBody,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_StartTag{StartTag: "table"}},
		Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_PARSE_ERROR, html.TreeAction_TREE_ACTION_IGNORE},
		NextMode: inTable,
		SpecParagraph: "13.2.6.4.12",
	})
	// "style", "script" → head processing
	for _, tag := range []string{"style", "script"} {
		rules = append(rules, &html.TreeRule{
			Mode:    inTableBody,
			Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_StartTag{StartTag: tag}},
			Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_PARSE_ERROR, html.TreeAction_TREE_ACTION_INSERT, treeActionForRawText(tag)},
			NextMode: textMode,
			SpecParagraph: "13.2.6.4.12",
		})
	}
	// Character → reprocess in IN_TABLE_TEXT
	rules = append(rules, &html.TreeRule{
		Mode:    inTableBody,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_CharacterToken{CharacterToken: true}},
		Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_REPROCESS},
		NextMode: inTableText,
		SpecParagraph: "13.2.6.4.12",
	})
	// Comment → append
	rules = append(rules, &html.TreeRule{
		Mode:    inTableBody,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_TokenType{TokenType: html.TokenType_TOKEN_TYPE_COMMENT}},
		Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_APPEND_COMMENT},
		NextMode: inTableBody,
		SpecParagraph: "13.2.6.4.12",
	})
	// EOF → done
	rules = append(rules, &html.TreeRule{
		Mode:    inTableBody,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_TokenType{TokenType: html.TokenType_TOKEN_TYPE_EOF}},
		NextMode: afterAfterBody,
		SpecParagraph: "13.2.6.4.12",
	})

	// ── IN_ROW mode (§13.2.6.4.13) ──
	// "td", "th" → insert, switch to IN_CELL
	for _, tag := range []string{"td", "th"} {
		rules = append(rules, &html.TreeRule{
			Mode:    inRow,
			Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_StartTag{StartTag: tag}},
			Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_INSERT},
			NextMode: inCell,
			SpecParagraph: "13.2.6.4.13",
		})
	}
	// "tr" → parse error, pop until row group
	rules = append(rules, &html.TreeRule{
		Mode:    inRow,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_StartTag{StartTag: "tr"}},
		Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_PARSE_ERROR, html.TreeAction_TREE_ACTION_IGNORE},
		NextMode: inTableBody,
		SpecParagraph: "13.2.6.4.13",
	})
	// "tbody", "thead", "tfoot", "caption", "col", "colgroup", "table" → parse error, pop until row
	for _, tag := range []string{"tbody", "thead", "tfoot", "caption", "col", "colgroup", "table"} {
		rules = append(rules, &html.TreeRule{
			Mode:    inRow,
			Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_StartTag{StartTag: tag}},
			Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_PARSE_ERROR, html.TreeAction_TREE_ACTION_IGNORE},
			NextMode: inTableBody,
			SpecParagraph: "13.2.6.4.13",
		})
	}
	// "td", "th" end tag → parse error, ignore
	for _, tag := range []string{"td", "th"} {
		rules = append(rules, &html.TreeRule{
			Mode:    inRow,
			Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_EndTag{EndTag: tag}},
			Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_PARSE_ERROR, html.TreeAction_TREE_ACTION_IGNORE},
			NextMode: inRow,
			SpecParagraph: "13.2.6.4.13",
		})
	}
	// EndTag "tr" → pop until tr
	rules = append(rules, &html.TreeRule{
		Mode:    inRow,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_EndTag{EndTag: "tr"}},
		Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_POP_UNTIL},
		PopUntilTag: "tr",
		NextMode: inTableBody,
		SpecParagraph: "13.2.6.4.13",
	})
	// "style", "script" → head processing
	for _, tag := range []string{"style", "script"} {
		rules = append(rules, &html.TreeRule{
			Mode:    inRow,
			Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_StartTag{StartTag: tag}},
			Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_PARSE_ERROR, html.TreeAction_TREE_ACTION_INSERT, treeActionForRawText(tag)},
			NextMode: textMode,
			SpecParagraph: "13.2.6.4.13",
		})
	}
	// Character → reprocess in IN_TABLE_TEXT
	rules = append(rules, &html.TreeRule{
		Mode:    inRow,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_CharacterToken{CharacterToken: true}},
		Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_REPROCESS},
		NextMode: inTableText,
		SpecParagraph: "13.2.6.4.13",
	})
	// Comment → append
	rules = append(rules, &html.TreeRule{
		Mode:    inRow,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_TokenType{TokenType: html.TokenType_TOKEN_TYPE_COMMENT}},
		Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_APPEND_COMMENT},
		NextMode: inRow,
		SpecParagraph: "13.2.6.4.13",
	})
	// EOF → done
	rules = append(rules, &html.TreeRule{
		Mode:    inRow,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_TokenType{TokenType: html.TokenType_TOKEN_TYPE_EOF}},
		NextMode: afterAfterBody,
		SpecParagraph: "13.2.6.4.13",
	})

	// ── IN_CELL mode (§13.2.6.4.14) ──
	// EndTag "td", "th" → pop until cell
	for _, tag := range []string{"td", "th"} {
		rules = append(rules, &html.TreeRule{
			Mode:    inCell,
			Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_EndTag{EndTag: tag}},
			Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_POP_UNTIL},
			PopUntilTag: tag,
			NextMode: inRow,
			SpecParagraph: "13.2.6.4.14",
		})
	}
	// "caption", "col", "colgroup", "tbody", "td", "tfoot", "th", "thead", "tr" → parse error, close cell, reprocess
	for _, tag := range []string{"caption", "col", "colgroup", "tbody", "td", "tfoot", "th", "thead", "tr"} {
		rules = append(rules, &html.TreeRule{
			Mode:    inCell,
			Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_StartTag{StartTag: tag}},
			Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_PARSE_ERROR},
			NextMode: inRow,
			Otherwise: true,
			SpecParagraph: "13.2.6.4.14",
		})
	}
	// Any start tag → insert, stay in IN_CELL
	rules = append(rules, &html.TreeRule{
		Mode:    inCell,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_AnyStartTag{AnyStartTag: true}},
		Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_INSERT},
		NextMode: inCell,
		SpecParagraph: "13.2.6.4.14",
	})
	// Character → append
	rules = append(rules, &html.TreeRule{
		Mode:    inCell,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_CharacterToken{CharacterToken: true}},
		Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_APPEND_CHARACTER},
		NextMode: inCell,
		SpecParagraph: "13.2.6.4.14",
	})
	// Comment → append
	rules = append(rules, &html.TreeRule{
		Mode:    inCell,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_TokenType{TokenType: html.TokenType_TOKEN_TYPE_COMMENT}},
		Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_APPEND_COMMENT},
		NextMode: inCell,
		SpecParagraph: "13.2.6.4.14",
	})
	// EOF → done
	rules = append(rules, &html.TreeRule{
		Mode:    inCell,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_TokenType{TokenType: html.TokenType_TOKEN_TYPE_EOF}},
		NextMode: afterAfterBody,
		SpecParagraph: "13.2.6.4.14",
	})

	// ── IN_CAPTION mode (§13.2.6.4.11) ──
	// EndTag "caption" → pop until caption
	rules = append(rules, &html.TreeRule{
		Mode:    inCaption,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_EndTag{EndTag: "caption"}},
		Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_POP_UNTIL},
		PopUntilTag: "caption",
		NextMode: inTable,
		SpecParagraph: "13.2.6.4.11",
	})
	// "caption", "col", "colgroup", "tbody", "td", "tfoot", "th", "thead", "tr" → parse error, close caption, reprocess
	for _, tag := range []string{"caption", "col", "colgroup", "tbody", "td", "tfoot", "th", "thead", "tr"} {
		rules = append(rules, &html.TreeRule{
			Mode:    inCaption,
			Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_StartTag{StartTag: tag}},
			Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_PARSE_ERROR},
			NextMode: inTable,
			Otherwise: true,
			SpecParagraph: "13.2.6.4.11",
		})
	}
	// Any start tag → insert, stay
	rules = append(rules, &html.TreeRule{
		Mode:    inCaption,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_AnyStartTag{AnyStartTag: true}},
		Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_INSERT},
		NextMode: inCaption,
		SpecParagraph: "13.2.6.4.11",
	})
	// Character → append
	rules = append(rules, &html.TreeRule{
		Mode:    inCaption,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_CharacterToken{CharacterToken: true}},
		Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_APPEND_CHARACTER},
		NextMode: inCaption,
		SpecParagraph: "13.2.6.4.11",
	})
	// Comment → append
	rules = append(rules, &html.TreeRule{
		Mode:    inCaption,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_TokenType{TokenType: html.TokenType_TOKEN_TYPE_COMMENT}},
		Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_APPEND_COMMENT},
		NextMode: inCaption,
		SpecParagraph: "13.2.6.4.11",
	})
	// EOF → done
	rules = append(rules, &html.TreeRule{
		Mode:    inCaption,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_TokenType{TokenType: html.TokenType_TOKEN_TYPE_EOF}},
		NextMode: afterAfterBody,
		SpecParagraph: "13.2.6.4.11",
	})

	// ── IN_COLUMN_GROUP mode (§13.2.6.4.8) ──
	// "col" → insert
	rules = append(rules, &html.TreeRule{
		Mode:    inColumnGroup,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_StartTag{StartTag: "col"}},
		Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_INSERT},
		NextMode: inColumnGroup,
		SpecParagraph: "13.2.6.4.8",
	})
	// EndTag "colgroup" → pop until colgroup
	rules = append(rules, &html.TreeRule{
		Mode:    inColumnGroup,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_EndTag{EndTag: "colgroup"}},
		Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_POP_UNTIL},
		PopUntilTag: "colgroup",
		NextMode: inTable,
		SpecParagraph: "13.2.6.4.8",
	})
	// Anything else → reprocess in IN_TABLE
	rules = append(rules, &html.TreeRule{
		Mode:    inColumnGroup,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_AnyStartTag{AnyStartTag: true}},
		Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_PARSE_ERROR},
		NextMode: inTable,
		Otherwise: true,
		SpecParagraph: "13.2.6.4.8",
	})
	rules = append(rules, &html.TreeRule{
		Mode:    inColumnGroup,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_TokenType{TokenType: html.TokenType_TOKEN_TYPE_COMMENT}},
		Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_APPEND_COMMENT},
		NextMode: inColumnGroup,
		SpecParagraph: "13.2.6.4.8",
	})
	rules = append(rules, &html.TreeRule{
		Mode:    inColumnGroup,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_CharacterToken{CharacterToken: true}},
		Actions: []html.TreeAction{html.TreeAction_TREE_ACTION_PARSE_ERROR, html.TreeAction_TREE_ACTION_IGNORE},
		NextMode: inTable,
		SpecParagraph: "13.2.6.4.8",
	})
	rules = append(rules, &html.TreeRule{
		Mode:    inColumnGroup,
		Trigger: &html.TokenTrigger{Trigger: &html.TokenTrigger_TokenType{TokenType: html.TokenType_TOKEN_TYPE_EOF}},
		NextMode: afterAfterBody,
		SpecParagraph: "13.2.6.4.8",
	})

	return rules
}

// treeActionForRawText returns the appropriate switch action for a raw text element.
func treeActionForRawText(tag string) html.TreeAction {
	switch tag {
	case "script":
		return html.TreeAction_TREE_ACTION_SWITCH_TO_SCRIPT_DATA
	case "style", "xmp", "iframe", "noembed", "noframes", "noscript":
		return html.TreeAction_TREE_ACTION_SWITCH_TO_RAWTEXT
	case "textarea", "title":
		return html.TreeAction_TREE_ACTION_SWITCH_TO_RCDATA
	default:
		return html.TreeAction_TREE_ACTION_SWITCH_TO_RAWTEXT
	}
}
