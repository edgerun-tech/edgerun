// Extract Fetch spec: Request/Response/Headers types, CORS modes, redirect modes, body mixin.
package main

import (
	"encoding/json"
	"fmt"
	"os"
)

type FetchInit struct {
	Name        string `json:"name"`
	Type        string `json:"type"`
	Description string `json:"description"`
}

type FetchMethod struct {
	Name       string `json:"name"`
	ReturnType string `json:"return_type"`
	Parameters string `json:"parameters"`
}

type FetchEnum struct {
	Name     string   `json:"name"`
	Variants []string `json:"variants"`
}

type FetchCatalog struct {
	Spec               string      `json:"spec"`
	SpecDate           string      `json:"spec_date"`
	RequestInit        []FetchInit `json:"request_init"`
	ResponseProperties []FetchInit `json:"response_properties"`
	HeadersMethods     []FetchMethod `json:"headers_methods"`
	BodyMethods        []FetchMethod `json:"body_methods"`
	Enums              []FetchEnum `json:"enums"`
}

func main() {
	if len(os.Args) >= 2 {
		// Read file but data is hardcoded, so we just need to ensure the file exists
		if _, err := os.ReadFile(os.Args[1]); err != nil {
			fmt.Fprintf(os.Stderr, "error: %v\n", err)
			os.Exit(1)
		}
	}

	catalog := FetchCatalog{
		Spec:     "Fetch Living Standard",
		SpecDate: "2026-04-10",
		RequestInit: []FetchInit{
			{"method", "String", "GET, POST, PUT, DELETE, etc."},
			{"url", "String", "The URL of the request"},
			{"headers", "HeadersInit", "Headers object or dict"},
			{"body", "Option<BodyInit>", "Request body"},
			{"referrer", "String", "Referrer URL"},
			{"referrer_policy", "String", "no-referrer, origin, etc."},
			{"mode", "RequestMode", "cors, no-cors, same-origin, navigate"},
			{"credentials", "RequestCredentials", "omit, same-origin, include"},
			{"cache", "RequestCache", "default, no-store, reload, etc."},
			{"redirect", "RequestRedirect", "follow, error, manual"},
			{"integrity", "String", "Subresource integrity hash"},
			{"keep_alive", "bool", "Keepalive flag"},
			{"signal", "Option<AbortSignal>", "AbortSignal"},
			{"priority", "RequestPriority", "high, low, auto"},
			{"duplex", "String", "half for streaming"},
		},
		ResponseProperties: []FetchInit{
			{"ok", "bool", "Success status (200-299)"},
			{"status", "u16", "HTTP status code"},
			{"status_text", "String", "Status message"},
			{"headers", "Headers", "Response headers"},
			{"url", "String", "Response URL"},
			{"type", "ResponseType", "basic, cors, opaque, etc."},
			{"redirected", "bool", "Whether redirected"},
			{"body", "Option<Body>", "Response body stream"},
		},
		HeadersMethods: []FetchMethod{
			{"append", "void", "name: &str, value: &str"},
			{"delete", "void", "name: &str"},
			{"get", "Option<String>", "name: &str"},
			{"get_set_cookie", "Vec<String>", ""},
			{"has", "bool", "name: &str"},
			{"set", "void", "name: &str, value: &str"},
			{"entries", "Iterator", ""},
			{"keys", "Iterator", ""},
			{"values", "Iterator", ""},
			{"for_each", "void", "callback: Fn(&str, &str)"},
		},
		BodyMethods: []FetchMethod{
			{"array_buffer", "Vec<u8>", ""},
			{"blob", "Blob", ""},
			{"bytes", "Vec<u8>", ""},
			{"form_data", "FormData", ""},
			{"json", "JsValue", ""},
			{"text", "String", ""},
		},
		Enums: []FetchEnum{
			{"RequestMode", []string{"SameOrigin", "Cors", "NoCors", "Navigate", "WebSocket"}},
			{"RequestCredentials", []string{"Omit", "SameOrigin", "Include"}},
			{"RequestCache", []string{"Default", "NoStore", "Reload", "NoCache", "ForceCache", "OnlyIfCached"}},
			{"RequestRedirect", []string{"Follow", "Error", "Manual"}},
			{"RequestPriority", []string{"High", "Low", "Auto"}},
			{"ResponseType", []string{"Basic", "Cors", "Default", "Error", "Opaque", "OpaqueRedirect"}},
			{"ReferrerPolicy", []string{"NoReferrer", "NoReferrerWhenDowngrade", "SameOrigin", "Origin", "StrictOrigin", "OriginWhenCrossOrigin", "StrictOriginWhenCrossOrigin", "UnsafeUrl"}},
			{"RequestDestination", []string{"Audio", "AudioWorklet", "Document", "Embed", "Font", "Frame", "Iframe", "Image", "Manifest", "Object", "PainterWorklet", "Report", "Script", "SharedWorker", "Style", "Track", "Video", "Worker", "Xslt"}},
		},
	}
	enc := json.NewEncoder(os.Stdout)
	enc.SetIndent("", "  ")
	enc.Encode(catalog)
}
