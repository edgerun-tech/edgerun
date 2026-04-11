package spec

import "regexp"

var regexCache = make(map[string]*regexp.Regexp)

func mustCompile(pattern string) *regexp.Regexp {
	if re, ok := regexCache[pattern]; ok {
		return re
	}
	re := regexp.MustCompile(pattern)
	regexCache[pattern] = re
	return re
}

func regexFindAll(pattern, text string) []string {
	re := mustCompile(pattern)
	return re.FindAllString(text, -1)
}

func regexFindAllSubmatch(pattern, text string) [][]string {
	re := mustCompile(pattern)
	return re.FindAllStringSubmatch(text, -1)
}

func regexReplace(pattern, text, repl string) string {
	re := mustCompile(pattern)
	return re.ReplaceAllString(text, repl)
}
