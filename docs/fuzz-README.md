# Edgerun Fuzz Testing

Auto-generated fuzz corpora from proto definitions.

## Generated Targets

- **75** fuzz targets (3 specialized + 72 enum-driven)
- **1309** seed corpus files

## Usage

```bash
# Run all fuzz targets
cargo fuzz run css_properties_cssproperty

# Run with specific corpus
cargo fuzz run css_border_parsing corpus/css_box/

# Run with address sanitizer
cargo fuzz run css_color_parsing -- -sanitize=address
```

## Coverage

Fuzz targets cover:
- All CSS property enum discriminants
- All CSS value types (66 types)
- All border styles and dash patterns
- All color formats (rgb, hsl, hwb, lab, lch, hex, named)
- Edge cases: out-of-range, boundary, zero, max values
