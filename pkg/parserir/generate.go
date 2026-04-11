package parserir

import (
	"edgerun-reference-core/gen/go/edgerun/v0/html"
	"google.golang.org/protobuf/proto"
)

// Generate writes all IR data files to dir.
func Generate(dir string) {
	ts := TokenizerTransitions()
	machine := &html.TokenizerStateMachine{
		SpecSection:    "13.2.5",
		SpecUrl:        "https://html.spec.whatwg.org/multipage/parsing.html#tokenization",
		SpecDate:       "2024-01-01",
		InitialState:   html.TokenizerState_DATA_STATE,
		TotalStates:    int32(len(html.TokenizerState_value)),
		Transitions:    ts,
		TotalTransitions: int32(len(ts)),
	}
	writeProto(dir, "tokenizer.textproto", machine)

	rules := TreeRules()
	ruleSet := &html.TreeBuilderRuleSet{
		SpecSection:     "13.2.6",
		SpecDate:        "2024-01-01",
		Rules:           rules,
		TotalRules:      int32(len(rules)),
		TotalInsertionModes: int32(len(html.InsertionMode_value)),
	}
	writeProto(dir, "tree_builder.textproto", ruleSet)

	catalog := Entities()
	writeProto(dir, "entities.textproto", catalog)

	writeMetadata(dir)
}

func writeProto(dir, filename string, msg proto.Message) {
	// delegated to util.go
	writeProtoImpl(dir, filename, msg)
}
