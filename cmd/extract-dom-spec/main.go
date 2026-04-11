// Extract DOM Living Standard: interfaces, methods, attributes, events.
package main

import (
	"encoding/json"
	"fmt"
	"os"
	"regexp"
	"strings"
)

type IDLAttr struct {
	Name     string `json:"name"`
	Type     string `json:"type"`
	Readonly bool   `json:"readonly"`
}

type IDLMethod struct {
	Name       string     `json:"name"`
	ReturnType string     `json:"return_type"`
	Parameters []IDLParam `json:"parameters"`
}

type IDLParam struct {
	Type string `json:"type"`
	Name string `json:"name"`
}

type DOMInterface struct {
	Name       string     `json:"name"`
	Parent     string     `json:"parent"`
	Attributes []IDLAttr  `json:"attributes"`
	Methods    []IDLMethod `json:"methods"`
}

type DOMEvent struct {
	Name string `json:"name"`
}

type DOMConstant struct {
	Kind   string `json:"kind"`
	Alias  string `json:"alias,omitempty"`
	Target string `json:"target,omitempty"`
}

type DOMCatalog struct {
	Spec           string         `json:"spec"`
	SpecDate       string         `json:"spec_date"`
	TotalInterfaces int           `json:"total_interfaces"`
	TotalEvents    int            `json:"total_events"`
	Interfaces     []DOMInterface `json:"interfaces"`
	Events         []DOMEvent     `json:"events"`
}

var rustTypeMap = map[string]string{
	"DOMString": "String", "USVString": "String", "ByteString": "String",
	"boolean": "bool", "unsigned short": "u16", "unsigned long": "u32",
	"unsigned long long": "u64", "short": "i16", "long": "i32", "long long": "i64",
	"float": "f32", "double": "f64", "any": "JsValue",
	"Node": "Node", "Element": "Element", "Event": "Event", "EventTarget": "EventTarget",
	"Document": "Document", "HTMLCollection": "HtmlCollection", "NodeList": "NodeList",
	"HTMLCollectionBase": "HtmlCollection",
}

func rustType(idlType string) string {
	if strings.HasPrefix(idlType, "sequence<") {
		inner := idlType[9 : len(idlType)-1]
		return "Vec<" + rustType(inner) + ">"
	}
	if strings.HasPrefix(idlType, "FrozenArray<") {
		inner := idlType[12 : len(idlType)-1]
		return "Vec<" + rustType(inner) + ">"
	}
	if t, ok := rustTypeMap[idlType]; ok {
		return t
	}
	return idlType
}

func parseIDLParams(s string) []IDLParam {
	s = strings.TrimSpace(s)
	if s == "" {
		return nil
	}
	var params []IDLParam
	for _, p := range strings.Split(s, ",") {
		p = strings.TrimSpace(p)
		p = strings.Split(p, "=")[0]
		p = strings.ReplaceAll(p, "[", "")
		p = strings.ReplaceAll(p, "]", "")
		p = strings.TrimSpace(p)
		if p == "" {
			continue
		}
		parts := strings.Fields(p)
		if len(parts) >= 2 {
			params = append(params, IDLParam{Type: rustType(parts[len(parts)-2]), Name: parts[len(parts)-1]})
		} else if len(parts) == 1 {
			params = append(params, IDLParam{Type: "unknown", Name: parts[0]})
		}
	}
	return params
}

func main() {
	if len(os.Args) < 2 {
		fmt.Fprintln(os.Stderr, "Usage: extract-dom-spec <dom-spec.md>")
		os.Exit(1)
	}
	data, err := os.ReadFile(os.Args[1])
	if err != nil {
		fmt.Fprintf(os.Stderr, "error: %v\n", err)
		os.Exit(1)
	}
	md := string(data)

	var interfaces []DOMInterface
	ifaceRe := regexp.MustCompile(`(interface\s+(\w+)(?:\s*:\s*(\w+))?\s*\{)(.*?)\};`)
	for _, m := range ifaceRe.FindAllStringSubmatch(md, -1) {
		ifaceName, parent, body := m[2], m[3], m[4]

		var attrs []IDLAttr
		attrRe := regexp.MustCompile(`((?:readonly\s+)?attribute\s+(\w+(?:<[^>]+>)?)\s+(\w+))`)
		attrMatches := attrRe.FindAllStringSubmatchIndex(body, -1)
		for _, amIdx := range attrMatches {
			_ = amIdx[0] // used below
			attrSub := body[amIdx[0]:amIdx[1]]
			am2 := regexp.MustCompile(`(?:readonly\s+)?attribute\s+(\w+(?:<[^>]+>)?)\s+(\w+)`).FindStringSubmatch(attrSub)
			if len(am2) < 3 {
				continue
			}
			before := body[max(0, amIdx[0]-20):amIdx[0]]
			attrs = append(attrs, IDLAttr{
				Name:     am2[2],
				Type:     rustType(am2[1]),
				Readonly: strings.Contains(before, "readonly"),
			})
		}

		var methods []IDLMethod
		methodRe := regexp.MustCompile(`(\w+(?:<[^>]+>)?)\s+(\w+)\s*\(([^)]*)\)`)
		for _, mm := range methodRe.FindAllStringSubmatch(body, -1) {
			mname := mm[2]
			if mname == "interface" || mname == "callback" || mname == "extends" {
				continue
			}
			methods = append(methods, IDLMethod{
				Name: mname, ReturnType: rustType(mm[1]),
				Parameters: parseIDLParams(mm[3]),
			})
		}

		interfaces = append(interfaces, DOMInterface{
			Name: ifaceName, Parent: parent,
			Attributes: attrs, Methods: methods,
		})
	}

	// Well-known events
	knownEvents := []string{
		"abort", "blur", "focus", "click", "dblclick", "mousedown", "mouseup",
		"mousemove", "mouseenter", "mouseleave", "keydown", "keyup", "keypress",
		"load", "unload", "beforeunload", "resize", "scroll", "error",
		"input", "change", "submit", "reset", "DOMContentLoaded", "readystatechange",
		"hashchange", "popstate", "pageshow", "pagehide",
		"touchstart", "touchend", "touchmove", "touchcancel",
		"wheel", "contextmenu", "animationstart", "animationend", "animationiteration",
		"transitionend", "pointerdown", "pointerup", "pointermove",
		"drag", "dragstart", "dragend", "dragover", "drop",
		"copy", "cut", "paste", "message", "open", "close",
		"compositionstart", "compositionend", "compositionupdate",
		"beforeinput", "select", "selectstart", "selectionchange",
		"visibilitychange", "fullscreenchange", "fullscreenerror",
		"pointerenter", "pointerleave", "gotpointercapture", "lostpointercapture",
	}
	var events []DOMEvent
	for _, e := range knownEvents {
		events = append(events, DOMEvent{Name: e})
	}

	catalog := DOMCatalog{
		Spec: "DOM Living Standard", SpecDate: "2026-04-10",
		TotalInterfaces: len(interfaces), TotalEvents: len(events),
		Interfaces: interfaces, Events: events,
	}
	enc := json.NewEncoder(os.Stdout)
	enc.SetIndent("", "  ")
	enc.Encode(catalog)
}
