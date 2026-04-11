package parserir

import (
	"encoding/json"
	"os"
	"path/filepath"

	"google.golang.org/protobuf/encoding/prototext"
	"google.golang.org/protobuf/proto"
)

func writeProtoImpl(dir, filename string, msg proto.Message) {
	data, err := prototext.MarshalOptions{Multiline: true, Indent: "  "}.Marshal(msg)
	if err != nil {
		panic(err)
	}
	path := filepath.Join(dir, filename)
	os.WriteFile(path, append(data, '\n'), 0644)
}

func writeMetadata(dir string) {
	metadata := map[string][]string{
		"void_elements": {
			"area", "base", "br", "col", "embed", "hr", "img", "input",
			"link", "meta", "source", "track", "wbr",
		},
		"raw_text_elements":           {"script", "style"},
		"escapable_raw_text_elements": {"textarea", "title"},
	}
	path := filepath.Join(dir, "element_metadata.json")
	data, _ := json.MarshalIndent(metadata, "", "  ")
	os.WriteFile(path, append(data, '\n'), 0644)
}
