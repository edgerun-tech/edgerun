// Solid JSX shim — re-exports h and Fragment for Bun's inject mechanism.
// When Bun bundles with --inject, this module's exports become top-level bindings.
import h from "solid-js/h";

function Fragment(props) {
  return props.children;
}

export { h, Fragment };

// Benchmark comment