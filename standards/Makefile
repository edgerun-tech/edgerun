WAT  := edgerun.wat
WASM := edgerun.wasm
STRIPPED := edgerun-stripped.wasm
ELF  := edgerun.elf

.DEFAULT_GOAL := all

.PHONY: all build validate watch clean ci stats help

all: build validate

## Build: concatenate fragments + compile WAT→WASM
build:
	sh build/build.sh

## Validate: run wasm-tools validate on outputs
validate:
	@echo "Validating $(WASM)..."
	wasm-tools validate "$(WASM)" && echo "  ✓ $(WASM) valid"
	@if [ -f "$(STRIPPED)" ]; then \
	  wasm-tools validate "$(STRIPPED)" && echo "  ✓ $(STRIPPED) valid"; \
	fi

## Watch: rebuild on file changes
watch:
	@echo "Watching for changes in source directories..."
	@while true; do \
	  find runtime compiler pipeline crypto protocol codec data device net system lang app ui -name '*.wat' -newer "$(WAT)" 2>/dev/null | grep -q . && { \
	    echo "  Change detected, rebuilding..."; \
	    sh build/build.sh; \
	    echo ""; \
	  }; \
	  sleep 1; \
	done

## CI: full pipeline (build → validate)
ci: build validate

## Clean: remove build artifacts
clean:
	rm -f $(WAT) $(WASM) $(STRIPPED) edgerun-full.wat
	rm -f *.wasm $(ELF)
	@echo "  Cleaned build artifacts"

## Stats: count lines and files
stats:
	@echo "=== EdgeRun Standards Stats ==="
	@find . -name '*.wat' -not -path '*/node_modules/*' -not -name 'edgerun*.wat' | wc -l | xargs echo "Source WAT files:"
	@find . -name '*.wat' -not -path '*/node_modules/*' -not -name 'edgerun*.wat' -exec wc -l {} + | tail -1 | xargs echo "Source WAT lines: "
	@if [ -f $(WAT) ]; then wc -l $(WAT) | xargs echo "Build output lines:"; fi
	@if [ -f $(WASM) ]; then ls -lh $(WASM) | awk '{print "Build output size: " $$5}'; fi

## Help: show available targets
help:
	@echo "EdgeRun Standards — Developer Commands"
	@echo ""
	@grep -E '^## ' $(MAKEFILE_LIST) | sed 's/## /* /'
