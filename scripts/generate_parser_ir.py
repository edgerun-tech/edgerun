#!/usr/bin/env python3
"""
Generate Parser IR — encodes WHATWG HTML §13.2.5 tokenizer as data.

Reads: nothing (Phase 1: encodes a minimal subset of the spec inline)
Writes: ir/tokenizer.json

The IR is a table-driven representation of the tokenizer state machine:
  table[current_state][char_class] → (next_state, actions[])

Phase 1 covers 4 states: DATA, TAG_OPEN, END_TAG_OPEN, TAG_NAME
This is enough to tokenize <div>Hello</div> → StartTag, Text, EndTag.

Phase 2 expands to all 86 states from WHATWG §13.2.5.
Phase 3 adds entity decoding (§13.1.4.22).
Phase 4 adds attribute parsing.
"""

import json
import os

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

# ---------------------------------------------------------------------------
# Character classes — maps each input char to a class index.
# The tokenizer never matches on raw chars, only on classes.
# ---------------------------------------------------------------------------

CHAR_CLASSES = {
    "EOF": 0,
    "LT": 1,       # <
    "GT": 2,       # >
    "SLASH": 3,    # /
    "ALPHA": 4,    # A-Z a-z
    "ALNUM": 5,    # A-Z a-z 0-9 - _
    "WS": 6,       # space, tab, newline
    "AMP": 7,      # &
    "QUOT": 8,     # "
    "APOS": 9,     # '
    "EQ": 10,      # =
    "NULL": 11,    # U+0000
    "OTHER": 12,   # everything else
}

# ---------------------------------------------------------------------------
# Token types the tokenizer can emit.
# ---------------------------------------------------------------------------

TOKEN_TYPES = {
    "NONE": 0,
    "START_TAG": 1,
    "END_TAG": 2,
    "CHARACTER": 3,
    "COMMENT": 4,
    "DOCTYPE": 5,
    "EOF": 6,
}

# ---------------------------------------------------------------------------
# Tokenizer states (Phase 1: 4 states, full spec has 86).
# ---------------------------------------------------------------------------

STATES = {
    "DATA": 0,
    "TAG_OPEN": 1,
    "END_TAG_OPEN": 2,
    "TAG_NAME": 3,
    "BEFORE_ATTRIBUTE_NAME": 4,
    "ATTRIBUTE_NAME": 5,
    "AFTER_ATTRIBUTE_NAME": 6,
    "BEFORE_ATTRIBUTE_VALUE": 7,
    "ATTRIBUTE_VALUE_DOUBLE_QUOTED": 8,
    "ATTRIBUTE_VALUE_SINGLE_QUOTED": 9,
    "ATTRIBUTE_VALUE_UNQUOTED": 10,
    "SELF_CLOSING_START_TAG": 11,
    "EOF": 12,
}

# ---------------------------------------------------------------------------
# Actions the tokenizer can take during a transition.
# ---------------------------------------------------------------------------

ACTIONS = {
    "NONE": 0,
    "EMIT_CHAR": 1,           # emit current char as character token
    "EMIT_EOF": 2,            # emit EOF token
    "START_TAG": 3,           # begin a new start tag token
    "END_TAG": 4,             # begin a new end tag token
    "APPEND_TAG_NAME": 5,     # append current char to current tag name
    "APPEND_ATTR_NAME": 6,    # append current char to current attr name
    "APPEND_ATTR_VALUE": 7,   # append current char to current attr value
    "EMIT_TOKEN": 8,          # emit the current tag token
    "PARSE_ERROR": 9,         # record a parse error
    "SET_SELF_CLOSING": 10,   # set the self-closing flag on current tag
    "SWITCH_TO_RAWTEXT": 11,  # switch tokenizer to RAWTEXT mode (for <script>/<style>)
    "SWITCH_TO_RCDATA": 12,   # switch tokenizer to RCDATA mode (for <title>/<textarea>)
}

# ---------------------------------------------------------------------------
# Transition table — the heart of the tokenizer.
#
# Each entry: (state, char_class, next_state, [actions])
#
# These are encoded from WHATWG §13.2.5. The spec defines each state as
# "Consume the next input character" followed by a list of character-specific
# behaviors. We translate those behaviors into table entries.
# ---------------------------------------------------------------------------

def build_transitions():
    transitions = []

    # ── DATA state (§13.2.5.1) ──
    # "<" → TAG_OPEN (no action — just switch state)
    transitions.append({
        "state": "DATA",
        "char_class": "LT",
        "next": "TAG_OPEN",
        "actions": [],
        "spec": "13.2.5.1"
    })
    # "&" → character reference (Phase 3 — for now, emit as char)
    transitions.append({
        "state": "DATA",
        "char_class": "AMP",
        "next": "DATA",
        "actions": ["EMIT_CHAR"],
        "spec": "13.2.5.1"
    })
    # EOF → emit EOF
    transitions.append({
        "state": "DATA",
        "char_class": "EOF",
        "next": "EOF",
        "actions": ["EMIT_EOF"],
        "spec": "13.2.5.1"
    })
    # Anything else → emit as character token
    transitions.append({
        "state": "DATA",
        "char_class": "ALPHA",
        "next": "DATA",
        "actions": ["EMIT_CHAR"],
        "spec": "13.2.5.1"
    })
    transitions.append({
        "state": "DATA",
        "char_class": "ALNUM",
        "next": "DATA",
        "actions": ["EMIT_CHAR"],
        "spec": "13.2.5.1"
    })
    transitions.append({
        "state": "DATA",
        "char_class": "WS",
        "next": "DATA",
        "actions": ["EMIT_CHAR"],
        "spec": "13.2.5.1"
    })
    transitions.append({
        "state": "DATA",
        "char_class": "OTHER",
        "next": "DATA",
        "actions": ["EMIT_CHAR"],
        "spec": "13.2.5.1"
    })

    # ── TAG_OPEN state (§13.2.5.6) ──
    # "!" → bogus comment (Phase 4)
    # "/" → END_TAG_OPEN
    transitions.append({
        "state": "TAG_OPEN",
        "char_class": "SLASH",
        "next": "END_TAG_OPEN",
        "actions": [],
        "spec": "13.2.5.6"
    })
    # ALPHA → start new start tag, append name, switch to TAG_NAME
    transitions.append({
        "state": "TAG_OPEN",
        "char_class": "ALPHA",
        "next": "TAG_NAME",
        "actions": ["START_TAG", "APPEND_TAG_NAME"],
        "spec": "13.2.5.6"
    })
    # Anything else → parse error, emit char, stay in DATA
    transitions.append({
        "state": "TAG_OPEN",
        "char_class": "OTHER",
        "next": "DATA",
        "actions": ["PARSE_ERROR", "EMIT_CHAR"],
        "spec": "13.2.5.6"
    })

    # ── END_TAG_OPEN state (§13.2.5.7) ──
    # ALPHA → start new end tag, append name, switch to TAG_NAME
    transitions.append({
        "state": "END_TAG_OPEN",
        "char_class": "ALPHA",
        "next": "TAG_NAME",
        "actions": ["END_TAG", "APPEND_TAG_NAME"],
        "spec": "13.2.5.7"
    })
    # ">" → parse error, switch to DATA
    transitions.append({
        "state": "END_TAG_OPEN",
        "char_class": "GT",
        "next": "DATA",
        "actions": ["PARSE_ERROR"],
        "spec": "13.2.5.7"
    })
    # EOF → parse error, switch to DATA
    transitions.append({
        "state": "END_TAG_OPEN",
        "char_class": "EOF",
        "next": "DATA",
        "actions": ["PARSE_ERROR"],
        "spec": "13.2.5.7"
    })
    # Anything else → parse error, bogus comment (Phase 4)
    transitions.append({
        "state": "END_TAG_OPEN",
        "char_class": "OTHER",
        "next": "DATA",
        "actions": ["PARSE_ERROR", "EMIT_CHAR"],
        "spec": "13.2.5.7"
    })

    # ── TAG_NAME state (§13.2.5.8) ──
    # TAB/LF/FF/SPACE → BEFORE_ATTRIBUTE_NAME
    transitions.append({
        "state": "TAG_NAME",
        "char_class": "WS",
        "next": "BEFORE_ATTRIBUTE_NAME",
        "actions": [],
        "spec": "13.2.5.8"
    })
    # "/" → SELF_CLOSING_START_TAG
    transitions.append({
        "state": "TAG_NAME",
        "char_class": "SLASH",
        "next": "SELF_CLOSING_START_TAG",
        "actions": [],
        "spec": "13.2.5.8"
    })
    # ">" → emit current tag token, switch to DATA
    transitions.append({
        "state": "TAG_NAME",
        "char_class": "GT",
        "next": "DATA",
        "actions": ["EMIT_TOKEN"],
        "spec": "13.2.5.8"
    })
    # NULL → parse error, append U+FFFD
    transitions.append({
        "state": "TAG_NAME",
        "char_class": "NULL",
        "next": "TAG_NAME",
        "actions": ["PARSE_ERROR", "APPEND_TAG_NAME"],
        "spec": "13.2.5.8"
    })
    # EOF → parse error, emit current tag
    transitions.append({
        "state": "TAG_NAME",
        "char_class": "EOF",
        "next": "DATA",
        "actions": ["PARSE_ERROR", "EMIT_TOKEN"],
        "spec": "13.2.5.8"
    })
    # Anything else → append to tag name
    transitions.append({
        "state": "TAG_NAME",
        "char_class": "ALNUM",
        "next": "TAG_NAME",
        "actions": ["APPEND_TAG_NAME"],
        "spec": "13.2.5.8"
    })
    transitions.append({
        "state": "TAG_NAME",
        "char_class": "OTHER",
        "next": "TAG_NAME",
        "actions": ["APPEND_TAG_NAME"],
        "spec": "13.2.5.8"
    })

    # ── BEFORE_ATTRIBUTE_NAME state (§13.2.5.36) ──
    # WS → stay (consume whitespace)
    transitions.append({
        "state": "BEFORE_ATTRIBUTE_NAME",
        "char_class": "WS",
        "next": "BEFORE_ATTRIBUTE_NAME",
        "actions": [],
        "spec": "13.2.5.36"
    })
    # "/" → SELF_CLOSING
    transitions.append({
        "state": "BEFORE_ATTRIBUTE_NAME",
        "char_class": "SLASH",
        "next": "SELF_CLOSING_START_TAG",
        "actions": [],
        "spec": "13.2.5.36"
    })
    # ">" → emit tag, go to DATA
    transitions.append({
        "state": "BEFORE_ATTRIBUTE_NAME",
        "char_class": "GT",
        "next": "DATA",
        "actions": ["EMIT_TOKEN"],
        "spec": "13.2.5.36"
    })
    # EOF → parse error, emit tag
    transitions.append({
        "state": "BEFORE_ATTRIBUTE_NAME",
        "char_class": "EOF",
        "next": "DATA",
        "actions": ["PARSE_ERROR", "EMIT_TOKEN"],
        "spec": "13.2.5.36"
    })
    # Anything else → start new attribute name
    transitions.append({
        "state": "BEFORE_ATTRIBUTE_NAME",
        "char_class": "QUOT",
        "next": "ATTRIBUTE_NAME",
        "actions": [],
        "spec": "13.2.5.36"
    })
    transitions.append({
        "state": "BEFORE_ATTRIBUTE_NAME",
        "char_class": "APOS",
        "next": "ATTRIBUTE_NAME",
        "actions": [],
        "spec": "13.2.5.36"
    })
    transitions.append({
        "state": "BEFORE_ATTRIBUTE_NAME",
        "char_class": "EQ",
        "next": "ATTRIBUTE_NAME",
        "actions": ["APPEND_ATTR_NAME"],
        "spec": "13.2.5.36"
    })
    transitions.append({
        "state": "BEFORE_ATTRIBUTE_NAME",
        "char_class": "ALNUM",
        "next": "ATTRIBUTE_NAME",
        "actions": ["APPEND_ATTR_NAME"],
        "spec": "13.2.5.36"
    })
    transitions.append({
        "state": "BEFORE_ATTRIBUTE_NAME",
        "char_class": "OTHER",
        "next": "ATTRIBUTE_NAME",
        "actions": ["APPEND_ATTR_NAME"],
        "spec": "13.2.5.36"
    })

    # ── ATTRIBUTE_NAME state (§13.2.5.40) ──
    # WS → AFTER_ATTRIBUTE_NAME
    transitions.append({
        "state": "ATTRIBUTE_NAME",
        "char_class": "WS",
        "next": "AFTER_ATTRIBUTE_NAME",
        "actions": [],
        "spec": "13.2.5.40"
    })
    # "/" → SELF_CLOSING
    transitions.append({
        "state": "ATTRIBUTE_NAME",
        "char_class": "SLASH",
        "next": "SELF_CLOSING_START_TAG",
        "actions": [],
        "spec": "13.2.5.40"
    })
    # "=" → BEFORE_ATTRIBUTE_VALUE
    transitions.append({
        "state": "ATTRIBUTE_NAME",
        "char_class": "EQ",
        "next": "BEFORE_ATTRIBUTE_VALUE",
        "actions": [],
        "spec": "13.2.5.40"
    })
    # ">" → emit tag, DATA
    transitions.append({
        "state": "ATTRIBUTE_NAME",
        "char_class": "GT",
        "next": "DATA",
        "actions": ["EMIT_TOKEN"],
        "spec": "13.2.5.40"
    })
    # EOF → parse error, emit tag
    transitions.append({
        "state": "ATTRIBUTE_NAME",
        "char_class": "EOF",
        "next": "DATA",
        "actions": ["PARSE_ERROR", "EMIT_TOKEN"],
        "spec": "13.2.5.40"
    })
    # QUOT → append (unquoted attribute name with quote — parse error)
    transitions.append({
        "state": "ATTRIBUTE_NAME",
        "char_class": "QUOT",
        "next": "ATTRIBUTE_NAME",
        "actions": ["APPEND_ATTR_NAME"],
        "spec": "13.2.5.40"
    })
    transitions.append({
        "state": "ATTRIBUTE_NAME",
        "char_class": "APOS",
        "next": "ATTRIBUTE_NAME",
        "actions": ["APPEND_ATTR_NAME"],
        "spec": "13.2.5.40"
    })
    transitions.append({
        "state": "ATTRIBUTE_NAME",
        "char_class": "ALNUM",
        "next": "ATTRIBUTE_NAME",
        "actions": ["APPEND_ATTR_NAME"],
        "spec": "13.2.5.40"
    })
    transitions.append({
        "state": "ATTRIBUTE_NAME",
        "char_class": "OTHER",
        "next": "ATTRIBUTE_NAME",
        "actions": ["APPEND_ATTR_NAME"],
        "spec": "13.2.5.40"
    })

    # ── AFTER_ATTRIBUTE_NAME state (§13.2.5.41) ──
    transitions.append({
        "state": "AFTER_ATTRIBUTE_NAME",
        "char_class": "WS",
        "next": "AFTER_ATTRIBUTE_NAME",
        "actions": [],
        "spec": "13.2.5.41"
    })
    transitions.append({
        "state": "AFTER_ATTRIBUTE_NAME",
        "char_class": "SLASH",
        "next": "SELF_CLOSING_START_TAG",
        "actions": [],
        "spec": "13.2.5.41"
    })
    transitions.append({
        "state": "AFTER_ATTRIBUTE_NAME",
        "char_class": "GT",
        "next": "DATA",
        "actions": ["EMIT_TOKEN"],
        "spec": "13.2.5.41"
    })
    transitions.append({
        "state": "AFTER_ATTRIBUTE_NAME",
        "char_class": "EOF",
        "next": "DATA",
        "actions": ["PARSE_ERROR", "EMIT_TOKEN"],
        "spec": "13.2.5.41"
    })
    transitions.append({
        "state": "AFTER_ATTRIBUTE_NAME",
        "char_class": "ALNUM",
        "next": "ATTRIBUTE_NAME",
        "actions": ["APPEND_ATTR_NAME"],
        "spec": "13.2.5.41"
    })
    transitions.append({
        "state": "AFTER_ATTRIBUTE_NAME",
        "char_class": "QUOT",
        "next": "ATTRIBUTE_NAME",
        "actions": [],
        "spec": "13.2.5.41"
    })
    transitions.append({
        "state": "AFTER_ATTRIBUTE_NAME",
        "char_class": "APOS",
        "next": "ATTRIBUTE_NAME",
        "actions": [],
        "spec": "13.2.5.41"
    })
    transitions.append({
        "state": "AFTER_ATTRIBUTE_NAME",
        "char_class": "EQ",
        "next": "ATTRIBUTE_NAME",
        "actions": ["APPEND_ATTR_NAME"],
        "spec": "13.2.5.41"
    })
    transitions.append({
        "state": "AFTER_ATTRIBUTE_NAME",
        "char_class": "OTHER",
        "next": "ATTRIBUTE_NAME",
        "actions": ["APPEND_ATTR_NAME"],
        "spec": "13.2.5.41"
    })

    # ── BEFORE_ATTRIBUTE_VALUE state (§13.2.5.42) ──
    transitions.append({
        "state": "BEFORE_ATTRIBUTE_VALUE",
        "char_class": "WS",
        "next": "BEFORE_ATTRIBUTE_VALUE",
        "actions": [],
        "spec": "13.2.5.42"
    })
    transitions.append({
        "state": "BEFORE_ATTRIBUTE_VALUE",
        "char_class": "QUOT",
        "next": "ATTRIBUTE_VALUE_DOUBLE_QUOTED",
        "actions": [],
        "spec": "13.2.5.42"
    })
    transitions.append({
        "state": "BEFORE_ATTRIBUTE_VALUE",
        "char_class": "APOS",
        "next": "ATTRIBUTE_VALUE_SINGLE_QUOTED",
        "actions": [],
        "spec": "13.2.5.42"
    })
    transitions.append({
        "state": "BEFORE_ATTRIBUTE_VALUE",
        "char_class": "GT",
        "next": "DATA",
        "actions": ["EMIT_TOKEN"],
        "spec": "13.2.5.42"
    })
    transitions.append({
        "state": "BEFORE_ATTRIBUTE_VALUE",
        "char_class": "ALNUM",
        "next": "ATTRIBUTE_VALUE_UNQUOTED",
        "actions": ["APPEND_ATTR_VALUE"],
        "spec": "13.2.5.42"
    })
    transitions.append({
        "state": "BEFORE_ATTRIBUTE_VALUE",
        "char_class": "OTHER",
        "next": "ATTRIBUTE_VALUE_UNQUOTED",
        "actions": ["APPEND_ATTR_VALUE"],
        "spec": "13.2.5.42"
    })

    # ── ATTRIBUTE_VALUE_DOUBLE_QUOTED state (§13.2.5.43) ──
    transitions.append({
        "state": "ATTRIBUTE_VALUE_DOUBLE_QUOTED",
        "char_class": "QUOT",
        "next": "AFTER_ATTRIBUTE_NAME",
        "actions": [],
        "spec": "13.2.5.43"
    })
    transitions.append({
        "state": "ATTRIBUTE_VALUE_DOUBLE_QUOTED",
        "char_class": "AMP",
        "next": "ATTRIBUTE_VALUE_DOUBLE_QUOTED",
        "actions": ["APPEND_ATTR_VALUE"],
        "spec": "13.2.5.43"
    })
    transitions.append({
        "state": "ATTRIBUTE_VALUE_DOUBLE_QUOTED",
        "char_class": "EOF",
        "next": "DATA",
        "actions": ["PARSE_ERROR", "EMIT_TOKEN"],
        "spec": "13.2.5.43"
    })
    transitions.append({
        "state": "ATTRIBUTE_VALUE_DOUBLE_QUOTED",
        "char_class": "ALNUM",
        "next": "ATTRIBUTE_VALUE_DOUBLE_QUOTED",
        "actions": ["APPEND_ATTR_VALUE"],
        "spec": "13.2.5.43"
    })
    transitions.append({
        "state": "ATTRIBUTE_VALUE_DOUBLE_QUOTED",
        "char_class": "OTHER",
        "next": "ATTRIBUTE_VALUE_DOUBLE_QUOTED",
        "actions": ["APPEND_ATTR_VALUE"],
        "spec": "13.2.5.43"
    })

    # ── ATTRIBUTE_VALUE_SINGLE_QUOTED state (§13.2.5.44) ──
    transitions.append({
        "state": "ATTRIBUTE_VALUE_SINGLE_QUOTED",
        "char_class": "APOS",
        "next": "AFTER_ATTRIBUTE_NAME",
        "actions": [],
        "spec": "13.2.5.44"
    })
    transitions.append({
        "state": "ATTRIBUTE_VALUE_SINGLE_QUOTED",
        "char_class": "AMP",
        "next": "ATTRIBUTE_VALUE_SINGLE_QUOTED",
        "actions": ["APPEND_ATTR_VALUE"],
        "spec": "13.2.5.44"
    })
    transitions.append({
        "state": "ATTRIBUTE_VALUE_SINGLE_QUOTED",
        "char_class": "EOF",
        "next": "DATA",
        "actions": ["PARSE_ERROR", "EMIT_TOKEN"],
        "spec": "13.2.5.44"
    })
    transitions.append({
        "state": "ATTRIBUTE_VALUE_SINGLE_QUOTED",
        "char_class": "ALNUM",
        "next": "ATTRIBUTE_VALUE_SINGLE_QUOTED",
        "actions": ["APPEND_ATTR_VALUE"],
        "spec": "13.2.5.44"
    })
    transitions.append({
        "state": "ATTRIBUTE_VALUE_SINGLE_QUOTED",
        "char_class": "OTHER",
        "next": "ATTRIBUTE_VALUE_SINGLE_QUOTED",
        "actions": ["APPEND_ATTR_VALUE"],
        "spec": "13.2.5.44"
    })

    # ── ATTRIBUTE_VALUE_UNQUOTED state (§13.2.5.45) ──
    transitions.append({
        "state": "ATTRIBUTE_VALUE_UNQUOTED",
        "char_class": "WS",
        "next": "BEFORE_ATTRIBUTE_NAME",
        "actions": [],
        "spec": "13.2.5.45"
    })
    transitions.append({
        "state": "ATTRIBUTE_VALUE_UNQUOTED",
        "char_class": "GT",
        "next": "DATA",
        "actions": ["EMIT_TOKEN"],
        "spec": "13.2.5.45"
    })
    transitions.append({
        "state": "ATTRIBUTE_VALUE_UNQUOTED",
        "char_class": "AMP",
        "next": "ATTRIBUTE_VALUE_UNQUOTED",
        "actions": ["APPEND_ATTR_VALUE"],
        "spec": "13.2.5.45"
    })
    transitions.append({
        "state": "ATTRIBUTE_VALUE_UNQUOTED",
        "char_class": "EOF",
        "next": "DATA",
        "actions": ["PARSE_ERROR", "EMIT_TOKEN"],
        "spec": "13.2.5.45"
    })
    transitions.append({
        "state": "ATTRIBUTE_VALUE_UNQUOTED",
        "char_class": "ALNUM",
        "next": "ATTRIBUTE_VALUE_UNQUOTED",
        "actions": ["APPEND_ATTR_VALUE"],
        "spec": "13.2.5.45"
    })
    transitions.append({
        "state": "ATTRIBUTE_VALUE_UNQUOTED",
        "char_class": "OTHER",
        "next": "ATTRIBUTE_VALUE_UNQUOTED",
        "actions": ["APPEND_ATTR_VALUE"],
        "spec": "13.2.5.45"
    })

    # ── SELF_CLOSING_START_TAG state (§13.2.5.38) ──
    transitions.append({
        "state": "SELF_CLOSING_START_TAG",
        "char_class": "GT",
        "next": "DATA",
        "actions": ["SET_SELF_CLOSING", "EMIT_TOKEN"],
        "spec": "13.2.5.38"
    })
    transitions.append({
        "state": "SELF_CLOSING_START_TAG",
        "char_class": "EOF",
        "next": "DATA",
        "actions": ["PARSE_ERROR", "EMIT_TOKEN"],
        "spec": "13.2.5.38"
    })
    transitions.append({
        "state": "SELF_CLOSING_START_TAG",
        "char_class": "OTHER",
        "next": "DATA",
        "actions": ["PARSE_ERROR"],
        "spec": "13.2.5.38"
    })

    return transitions


# ---------------------------------------------------------------------------
# Tree builder rules — Phase 1: IN_BODY mode only.
# These handle what the tree builder does when it receives tokens.
# ---------------------------------------------------------------------------

def build_tree_rules():
    rules = []

    # IN_BODY mode rules (§13.2.6.4.16.7 — "In body" mode):
    rules.append({
        "mode": "IN_BODY",
        "trigger": {"start_tag": "div"},
        "actions": ["INSERT"],
        "next_mode": "IN_BODY",
        "spec": "13.2.6.4.16.7"
    })
    rules.append({
        "mode": "IN_BODY",
        "trigger": {"start_tag": "span"},
        "actions": ["INSERT"],
        "next_mode": "IN_BODY",
        "spec": "13.2.6.4.16.7"
    })
    rules.append({
        "mode": "IN_BODY",
        "trigger": {"start_tag": "p"},
        "actions": ["INSERT"],
        "next_mode": "IN_BODY",
        "spec": "13.2.6.4.16.7"
    })
    rules.append({
        "mode": "IN_BODY",
        "trigger": {"start_tag": "a"},
        "actions": ["INSERT"],
        "next_mode": "IN_BODY",
        "spec": "13.2.6.4.16.7"
    })
    rules.append({
        "mode": "IN_BODY",
        "trigger": {"start_tag": "h1"},
        "actions": ["INSERT"],
        "next_mode": "IN_BODY",
        "spec": "13.2.6.4.16.7"
    })
    rules.append({
        "mode": "IN_BODY",
        "trigger": {"start_tag": "h2"},
        "actions": ["INSERT"],
        "next_mode": "IN_BODY",
        "spec": "13.2.6.4.16.7"
    })
    rules.append({
        "mode": "IN_BODY",
        "trigger": {"start_tag": "h3"},
        "actions": ["INSERT"],
        "next_mode": "IN_BODY",
        "spec": "13.2.6.4.16.7"
    })
    rules.append({
        "mode": "IN_BODY",
        "trigger": {"start_tag": "ul"},
        "actions": ["INSERT"],
        "next_mode": "IN_BODY",
        "spec": "13.2.6.4.16.7"
    })
    rules.append({
        "mode": "IN_BODY",
        "trigger": {"start_tag": "li"},
        "actions": ["INSERT"],
        "next_mode": "IN_BODY",
        "spec": "13.2.6.4.16.7"
    })
    rules.append({
        "mode": "IN_BODY",
        "trigger": {"start_tag": "any"},
        "actions": ["INSERT"],
        "next_mode": "IN_BODY",
        "spec": "13.2.6.4.16.7"
    })
    rules.append({
        "mode": "IN_BODY",
        "trigger": {"end_tag": "div"},
        "actions": ["POP_UNTIL"],
        "pop_until": "div",
        "next_mode": "IN_BODY",
        "spec": "13.2.6.4.16.7"
    })
    rules.append({
        "mode": "IN_BODY",
        "trigger": {"end_tag": "span"},
        "actions": ["POP_UNTIL"],
        "pop_until": "span",
        "next_mode": "IN_BODY",
        "spec": "13.2.6.4.16.7"
    })
    rules.append({
        "mode": "IN_BODY",
        "trigger": {"end_tag": "p"},
        "actions": ["POP_UNTIL"],
        "pop_until": "p",
        "next_mode": "IN_BODY",
        "spec": "13.2.6.4.16.7"
    })
    rules.append({
        "mode": "IN_BODY",
        "trigger": {"end_tag": "any"},
        "actions": ["POP_UNTIL"],
        "pop_until": None,  # generic: pop until matching
        "next_mode": "IN_BODY",
        "spec": "13.2.6.4.16.7"
    })
    rules.append({
        "mode": "IN_BODY",
        "trigger": {"character": True},
        "actions": ["APPEND_CHARACTER"],
        "next_mode": "IN_BODY",
        "spec": "13.2.6.4.16.7"
    })
    rules.append({
        "mode": "IN_BODY",
        "trigger": {"eof": True},
        "actions": [],
        "next_mode": "AFTER_BODY",
        "spec": "13.2.6.4.16.7"
    })

    return rules


# ---------------------------------------------------------------------------
# Entity catalog — Phase 1: minimal set, Phase 3: full 2,231.
# ---------------------------------------------------------------------------

def build_entities():
    return [
        {"name": "amp", "code_point_1": 0x0026, "code_point_2": 0, "semicolon_required": True},
        {"name": "lt", "code_point_1": 0x003C, "code_point_2": 0, "semicolon_required": True},
        {"name": "gt", "code_point_1": 0x003E, "code_point_2": 0, "semicolon_required": True},
        {"name": "quot", "code_point_1": 0x0022, "code_point_2": 0, "semicolon_required": True},
        {"name": "apos", "code_point_1": 0x0027, "code_point_2": 0, "semicolon_required": True},
        {"name": "nbsp", "code_point_1": 0x00A0, "code_point_2": 0, "semicolon_required": True},
    ]


# ---------------------------------------------------------------------------
# Void elements — elements that never have children.
# ---------------------------------------------------------------------------

VOID_ELEMENTS = [
    "area", "base", "br", "col", "embed", "hr", "img", "input",
    "link", "meta", "source", "track", "wbr",
]

# Raw text elements — content is text, not parsed as HTML.
RAW_TEXT_ELEMENTS = ["script", "style"]

# Escapable raw text elements.
ESCAPABLE_RAW_TEXT_ELEMENTS = ["textarea", "title"]


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def main():
    ir = {
        "char_classes": CHAR_CLASSES,
        "token_types": TOKEN_TYPES,
        "states": STATES,
        "actions": ACTIONS,
        "transitions": build_transitions(),
        "tree_rules": build_tree_rules(),
        "entities": build_entities(),
        "void_elements": VOID_ELEMENTS,
        "raw_text_elements": RAW_TEXT_ELEMENTS,
        "escapable_raw_text_elements": ESCAPABLE_RAW_TEXT_ELEMENTS,
    }

    ir_dir = os.path.join(ROOT, "ir")
    os.makedirs(ir_dir, exist_ok=True)
    ir_path = os.path.join(ir_dir, "tokenizer.json")
    with open(ir_path, "w") as f:
        json.dump(ir, f, indent=2)

    print(f"IR built: {ir_path}")
    print(f"  States: {len(STATES)}")
    print(f"  Transitions: {len(ir['transitions'])}")
    print(f"  Tree rules: {len(ir['tree_rules'])}")
    print(f"  Entities: {len(ir['entities'])}")


if __name__ == "__main__":
    main()
