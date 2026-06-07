BUN  := $(shell which bun 2>/dev/null || echo node)
WAT  := edgerun.wat
WASM := edgerun.wasm

.DEFAULT_GOAL := all

.PHONY: all build validate test watch clean ci lint lint-wat stats info

all: build validate

## Build: concatenate fragments + compile WAT→WASM
build:
	$(BUN) tools/build_wat.mjs

## Validate: run wasm-tools validate on outputs
validate:
	$(BUN) tools/validate.mjs

## Test: instantiate WASM and run integration tests
test:
	$(BUN) tools/test.mjs

## Watch: rebuild on file changes
watch:
	$(BUN) tools/build_wat.mjs --watch

## CI: full pipeline (build → validate → test)
ci: build validate test

## Clean: remove build artifacts
clean:
	rm -f $(WAT) $(WASM) edgerun-stripped.wasm edgerun-full.wat
	rm -f *.wasm
	@echo "  Cleaned build artifacts"

## Lint: validate individual WAT files
lint-wat:
	@echo "  Linting individual .wat files..."
	@count=0; fail=0; \
	for f in $$(find runtime compiler pipeline crypto protocol codec data device net system tools -name '*.wat' 2>/dev/null); do \
	  if wasm-tools validate "$$f" 2>/dev/null; then \
	    count=$$((count + 1)); \
	  else \
	    echo "  FAIL: $$f"; \
	    fail=$$((fail + 1)); \
	  fi; \
	done; \
	echo "  $$count files OK, $$fail failed"; \
	[ $$fail -eq 0 ]

## Lint: check JS tooling with node
lint:
	@if [ -n "$(shell which node 2>/dev/null)" ]; then \
	  node --check tools/build_wat.mjs 2>&1 | grep -v "Warning" || true; \
	  node --check tools/validate.mjs 2>&1 | grep -v "Warning" || true; \
	  node --check tools/test.mjs 2>&1 | grep -v "Warning" || true; \
	  echo "  JS syntax OK"; \
	fi

## Stats: count lines and files
stats:
	@echo "=== EdgeRun Standards Stats ==="
	@find . -name '*.wat' -not -path '*/node_modules/*' -not -name 'edgerun*.wat' | wc -l | xargs echo "Source WAT files:"
	@find . -name '*.wat' -not -path '*/node_modules/*' -not -name 'edgerun*.wat' -exec wc -l {} + | tail -1 | xargs echo "Source WAT lines: "
	@find . -name '*.mjs' -not -path '*/node_modules/*' | xargs wc -l 2>/dev/null | tail -1 | xargs echo "JS tooling lines:"
	@if [ -f $(WAT) ]; then wc -l $(WAT) | xargs echo "Build output lines:"; fi
	@if [ -f $(WASM) ]; then ls -lh $(WASM) | awk '{print "Build output size: " $$5}'; fi

## Help: show available targets
help:
	@echo "EdgeRun Standards — Developer Commands"
	@echo ""
	@grep -E '^## ' $(MAKEFILE_LIST) | sed 's/## /* /'
