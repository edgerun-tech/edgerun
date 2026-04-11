// Extract URL spec: URL parser states, URLSearchParams, origin, etc.
package main

import (
	"encoding/json"
	"fmt"
	"os"
)

type URLProp struct {
	Name        string `json:"name"`
	Type        string `json:"type"`
	Description string `json:"description"`
}

type URLSPMethod struct {
	Name       string `json:"name"`
	ReturnType string `json:"return_type"`
	Parameters string `json:"parameters"`
}

type URLCatalog struct {
	Spec             string       `json:"spec"`
	SpecDate         string       `json:"spec_date"`
	URLParserStates  []string     `json:"url_parser_states"`
	URLProperties    []URLProp    `json:"url_properties"`
	URLSPMethods     []URLSPMethod `json:"urlsearchparams_methods"`
	SpecialSchemes   []string     `json:"special_schemes"`
	DefaultPorts     map[string]*int `json:"default_ports"`
}

func main() {
	if len(os.Args) >= 2 {
		if _, err := os.ReadFile(os.Args[1]); err != nil {
			fmt.Fprintf(os.Stderr, "error: %v\n", err)
			os.Exit(1)
		}
	}

	none := (*int)(nil)
	filePort := (*int)(nil)

	catalog := URLCatalog{
		Spec:     "URL Living Standard",
		SpecDate: "2026-04-10",
		URLParserStates: []string{
			"SchemeStart", "Scheme", "NoScheme", "SpecialRelativeOrAuthority",
			"PathOrAuthority", "Relative", "RelativeSlash", "SpecialAuthoritySlashes",
			"SpecialAuthorityIgnoreSlashes", "Query", "Host", "Hostname", "Port",
			"PathStart", "Path", "CannotBeABaseUrlPath", "File", "FileHost",
			"FileSlash", "Fragment",
		},
		URLProperties: []URLProp{
			{"href", "String", "The full URL"},
			{"origin", "String", "The origin (read-only)"},
			{"protocol", "String", "Scheme with trailing ':'"},
			{"username", "String", "Username component"},
			{"password", "String", "Password component"},
			{"host", "String", "Host with port"},
			{"hostname", "String", "Host without port"},
			{"port", "Option<String>", "Port number"},
			{"pathname", "String", "Path component"},
			{"search", "String", "Query string with '?'"},
			{"hash", "String", "Fragment with '#'"},
		},
		URLSPMethods: []URLSPMethod{
			{"append", "void", "name: &str, value: &str"},
			{"delete", "void", "name: &str"},
			{"delete_with_value", "void", "name: &str, value: &str"},
			{"get", "Option<String>", "name: &str"},
			{"get_all", "Vec<String>", "name: &str"},
			{"has", "bool", "name: &str"},
			{"has_with_value", "bool", "name: &str, value: &str"},
			{"set", "void", "name: &str, value: &str"},
			{"sort", "void", ""},
			{"to_string", "String", ""},
		},
		SpecialSchemes: []string{"ftp", "file", "http", "https", "ws", "wss"},
		DefaultPorts: map[string]*int{
			"ftp":    ptr(21),
			"file":   filePort,
			"http":   ptr(80),
			"https":  ptr(443),
			"ws":     ptr(80),
			"wss":    ptr(443),
		},
	}
	_ = none
	enc := json.NewEncoder(os.Stdout)
	enc.SetIndent("", "  ")
	enc.Encode(catalog)
}

func ptr(i int) *int { return &i }
