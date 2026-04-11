// Package htmlcodegen reads proto IR text files and generates Rust parser source.
package htmlcodegen

// CharClassPattern maps a proto CharClass enum value to a Rust match pattern.
func CharClassPattern(cc int32) string {
	switch cc {
	case 1: // CHAR_CLASS_EOF
		return "" // handled at input level
	case 17: // CHAR_CLASS_LT
		return "'<'"
	case 15: // CHAR_CLASS_GT
		return "'>'"
	case 11: // CHAR_CLASS_SLASH
		return "'/'"
	case 8: // CHAR_CLASS_AMP
		return "'&'"
	case 7: // CHAR_CLASS_QUOT
		return "'\"'"
	case 9: // CHAR_CLASS_APOS
		return "'\\''"
	case 16: // CHAR_CLASS_EQ
		return "'='"
	case 19: // CHAR_CLASS_NULL
		return "'\\0'"
	case 6, 2, 3, 5, 4: // SPACE, TAB, LF, CR, FF
		return "' ' | '\\t' | '\\n' | '\\r' | '\\x0C'"
	case 14: // LOWER_ALPHA
		return "'a'..='z'"
	case 13: // UPPER_ALPHA
		return "'A'..='Z'"
	case 12: // DIGIT
		return "'0'..='9'"
	case 10: // HYPHEN
		return "'-'"
	case 18: // QUESTION
		return "'?'"
	case 20: // SEMICOLON
		return "';'"
	case 21: // EXCLAMATION
		return "'!'"
	case 22: // SQUARE_CLOSE
		return "']'"
	case 23: // PERCENT
		return "'%'"
	default: // OTHER or unspecified
		return "_"
	}
}

// CcSortKey determines match arm ordering — specific patterns first, wildcard last.
func CcSortKey(cc int32) int {
	switch cc {
	case 14: // LOWER_ALPHA
		return 0
	case 13: // UPPER_ALPHA
		return 1
	case 12: // DIGIT
		return 2
	case 10: // HYPHEN
		return 3
	case 24: // OTHER
		return 99
	default:
		return 10
	}
}

// IsWhitespaceCharClass returns true if the char class represents whitespace.
func IsWhitespaceCharClass(cc int32) bool {
	return cc == 6 || cc == 2 || cc == 3 || cc == 5 || cc == 4 // SPACE, TAB, LF, CR, FF
}
