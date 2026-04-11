package spec

import (
	"os"
	"strings"

	"github.com/spf13/cobra"

	pb "edgerunrefcore/proto/go/spec"
)

func runDOM(cmd *cobra.Command, args []string) error {
	data, err := os.ReadFile(args[0])
	if err != nil {
		return err
	}
	md := string(data)
	cat := &pb.Catalog{SpecName: "DOM Living Standard", SpecDate: "2026-04-10"}

	// Extract IDL interface blocks
	matches := regexFindAllSubmatch(`(interface\s+(\w+)(?:\s*:\s*(\w+))?\s*\{)(.*?)\};`, md)
	for _, m := range matches {
		iface := &pb.DomInterface{Name: m[2], Parent: m[3]}
		body := m[4]

		// Extract attributes
		attrMatches := regexFindAllSubmatch(`(?:readonly\s+)?attribute\s+(\w+(?:<[^>]+>)?)\s+(\w+)`, body)
		for _, am := range attrMatches {
			a := &pb.DomAttribute{Name: am[2], Type: rustType(am[1])}
			a.Readonly = strings.Contains(body[max(0, indexOf(body, am[0])-20):indexOf(body, am[0])], "readonly")
			iface.Attributes = append(iface.Attributes, a)
		}

		// Extract methods
		methodMatches := regexFindAllSubmatch(`(\w+(?:<[^>]+>)?)\s+(\w+)\s*\(([^)]*)\)`, body)
		for _, mm := range methodMatches {
			if mm[2] == "interface" || mm[2] == "callback" || mm[2] == "extends" {
				continue
			}
			meth := &pb.DomMethod{
				Name:       mm[2],
				ReturnType: rustType(mm[1]),
				Parameters: parseIDLParams(mm[3]),
			}
			iface.Methods = append(iface.Methods, meth)
		}
		cat.DomInterfaces = append(cat.DomInterfaces, iface)
	}

	// Extract events from markdown
	evtMatches := regexFindAllSubmatch(`^(?:event-type|event|eventname):\s*(\w+)`, md)
	seenEvents := make(map[string]bool)
	for _, m := range evtMatches {
		evt := m[1]
		if len(evt) > 2 && !seenEvents[evt] {
			seenEvents[evt] = true
			cat.DomEvents = append(cat.DomEvents, &pb.DomEvent{Name: evt})
		}
	}

	// Add well-known events
	for _, evt := range wellKnownEvents {
		if !seenEvents[evt] {
			seenEvents[evt] = true
			cat.DomEvents = append(cat.DomEvents, &pb.DomEvent{Name: evt})
		}
	}

	return writeProto(cat)
}

var wellKnownEvents = []string{
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

var idlTypeMap = map[string]string{
	"DOMString":      "String",
	"USVString":      "String",
	"ByteString":     "String",
	"boolean":        "bool",
	"unsigned short": "u16",
	"unsigned long":  "u32",
	"short":          "i16",
	"long":           "i32",
	"long long":      "i64",
	"float":          "f32",
	"double":         "f64",
	"any":            "JsValue",
}

func rustType(idlType string) string {
	idlType = strings.TrimSpace(idlType)
	if strings.HasPrefix(idlType, "sequence<") {
		inner := idlType[9 : len(idlType)-1]
		return "Vec<" + rustType(inner) + ">"
	}
	if strings.HasPrefix(idlType, "FrozenArray<") {
		inner := idlType[12 : len(idlType)-1]
		return "Vec<" + rustType(inner) + ">"
	}
	if t, ok := idlTypeMap[idlType]; ok {
		return t
	}
	return idlType
}

func parseIDLParams(paramStr string) []*pb.DomParameter {
	paramStr = strings.TrimSpace(paramStr)
	if paramStr == "" {
		return nil
	}
	var params []*pb.DomParameter
	for _, p := range strings.Split(paramStr, ",") {
		p = strings.TrimSpace(p)
		if p == "" {
			continue
		}
		p = strings.Split(p, "=")[0]
		p = strings.ReplaceAll(p, "[", "")
		p = strings.ReplaceAll(p, "]", "")
		p = strings.TrimSpace(p)
		parts := strings.Fields(p)
		if len(parts) >= 2 {
			params = append(params, &pb.DomParameter{
				Type: rustType(parts[len(parts)-2]),
				Name: parts[len(parts)-1],
			})
		} else if len(parts) == 1 {
			params = append(params, &pb.DomParameter{Type: "unknown", Name: parts[0]})
		}
	}
	return params
}

func max(a, b int) int {
	if a > b {
		return a
	}
	return b
}

func indexOf(s, substr string) int {
	return strings.Index(s, substr)
}
