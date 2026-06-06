(import "edgerun" "is_alpha" (func $is_alpha (param i32) (result i32)))
(import "edgerun" "is_digit" (func $is_digit (param i32) (result i32)))

;; Captures proc-macro2 token model and fallback lexer semantics.
  (func (export "proto_standard_id") (result i32) (i32.const 300138))

  (func (export "pm2_delimiter_open_code") (param $c i32) (result i32)
    (if (i32.eq (local.get $c) (i32.const 40)) (then (return (i32.const 1))))
    (if (i32.eq (local.get $c) (i32.const 123)) (then (return (i32.const 2))))
    (if (i32.eq (local.get $c) (i32.const 91)) (then (return (i32.const 3))))
    (i32.const 0))

  (func (export "pm2_delimiter_close_code") (param $c i32) (result i32)
    (if (i32.eq (local.get $c) (i32.const 41)) (then (return (i32.const 1))))
    (if (i32.eq (local.get $c) (i32.const 125)) (then (return (i32.const 2))))
    (if (i32.eq (local.get $c) (i32.const 93)) (then (return (i32.const 3))))
    (i32.const 0))

  (func (export "pm2_delimiter_matches") (param $open_code i32) (param $close_code i32) (result i32)
    (i32.and (i32.ne (local.get $open_code) (i32.const 0)) (i32.eq (local.get $open_code) (local.get $close_code))))

  ;; TokenTree variant codes: group=1, ident=2, punct=3, literal=4.
  (func (export "pm2_leaf_token_choice")
    (param $literal_ok i32) (param $punct_ok i32) (param $ident_ok i32) (param $rustc_error_marker i32)
    (result i32)
    (if (local.get $literal_ok) (then (return (i32.const 4))))
    (if (local.get $punct_ok) (then (return (i32.const 3))))
    (if (local.get $ident_ok) (then (return (i32.const 2))))
    (if (local.get $rustc_error_marker) (then (return (i32.const 4))))
    (i32.const 0))

  (func $pm2_ident_start_ascii (export "pm2_ident_start_ascii") (param $c i32) (result i32)
    (i32.or (i32.eq (local.get $c) (i32.const 95)) (call $is_alpha (local.get $c))))

  (func (export "pm2_ident_continue_ascii") (param $c i32) (result i32)
    (i32.or (call $pm2_ident_start_ascii (local.get $c)) (call $is_digit (local.get $c))))

  ;; Raw idents reject _, super, self, Self, crate. Input class codes:
  ;; 0 normal identifier, 1 _, 2 super, 3 self, 4 Self, 5 crate.
  (func (export "pm2_raw_ident_allowed") (param $ident_class i32) (result i32)
    (i32.eq (local.get $ident_class) (i32.const 0)))

  (func $pm2_punct_char_valid (export "pm2_punct_char_valid") (param $c i32) (result i32)
    (if (i32.eq (local.get $c) (i32.const 126)) (then (return (i32.const 1))))
    (if (i32.eq (local.get $c) (i32.const 33)) (then (return (i32.const 1))))
    (if (i32.eq (local.get $c) (i32.const 64)) (then (return (i32.const 1))))
    (if (i32.eq (local.get $c) (i32.const 35)) (then (return (i32.const 1))))
    (if (i32.eq (local.get $c) (i32.const 36)) (then (return (i32.const 1))))
    (if (i32.eq (local.get $c) (i32.const 37)) (then (return (i32.const 1))))
    (if (i32.eq (local.get $c) (i32.const 94)) (then (return (i32.const 1))))
    (if (i32.eq (local.get $c) (i32.const 38)) (then (return (i32.const 1))))
    (if (i32.eq (local.get $c) (i32.const 42)) (then (return (i32.const 1))))
    (if (i32.eq (local.get $c) (i32.const 45)) (then (return (i32.const 1))))
    (if (i32.eq (local.get $c) (i32.const 61)) (then (return (i32.const 1))))
    (if (i32.eq (local.get $c) (i32.const 43)) (then (return (i32.const 1))))
    (if (i32.eq (local.get $c) (i32.const 124)) (then (return (i32.const 1))))
    (if (i32.eq (local.get $c) (i32.const 59)) (then (return (i32.const 1))))
    (if (i32.eq (local.get $c) (i32.const 58)) (then (return (i32.const 1))))
    (if (i32.eq (local.get $c) (i32.const 44)) (then (return (i32.const 1))))
    (if (i32.eq (local.get $c) (i32.const 60)) (then (return (i32.const 1))))
    (if (i32.eq (local.get $c) (i32.const 46)) (then (return (i32.const 1))))
    (if (i32.eq (local.get $c) (i32.const 62)) (then (return (i32.const 1))))
    (if (i32.eq (local.get $c) (i32.const 47)) (then (return (i32.const 1))))
    (if (i32.eq (local.get $c) (i32.const 63)) (then (return (i32.const 1))))
    (if (i32.eq (local.get $c) (i32.const 39)) (then (return (i32.const 1))))
    (i32.const 0))

  ;; Punct spacing: 1 Alone, 2 Joint. Comments cannot start punct '/'. Quote joins only valid lifetime.
  (func (export "pm2_punct_spacing")
    (param $ch i32) (param $next_is_punct i32) (param $is_comment_start i32) (param $lifetime_ok i32)
    (result i32)
    (if (local.get $is_comment_start) (then (return (i32.const 0))))
    (if (i32.eqz (call $pm2_punct_char_valid (local.get $ch))) (then (return (i32.const 0))))
    (if (i32.eq (local.get $ch) (i32.const 39))
      (then (return (select (i32.const 2) (i32.const 0) (local.get $lifetime_ok)))))
    (select (i32.const 2) (i32.const 1) (local.get $next_is_punct)))

  ;; Skip state: 0 no skip, 1 whitespace, 2 line comment, 3 block comment, 4 empty block comment.
  (func (export "pm2_skip_state")
    (param $first i32) (param $second i32) (param $third i32) (param $fourth i32)
    (result i32)
    (if (i32.or (i32.eq (local.get $first) (i32.const 32))
                (i32.and (i32.ge_u (local.get $first) (i32.const 9)) (i32.le_u (local.get $first) (i32.const 13))))
      (then (return (i32.const 1))))
    (if (i32.and (i32.eq (local.get $first) (i32.const 47)) (i32.eq (local.get $second) (i32.const 47)))
      (then
        (if (i32.or (i32.and (i32.eq (local.get $third) (i32.const 47)) (i32.ne (local.get $fourth) (i32.const 47)))
                    (i32.eq (local.get $third) (i32.const 33)))
          (then (return (i32.const 0))))
        (return (i32.const 2))))
    (if (i32.and (i32.eq (local.get $first) (i32.const 47)) (i32.eq (local.get $second) (i32.const 42)))
      (then
        (if (i32.and (i32.eq (local.get $third) (i32.const 42)) (i32.eq (local.get $fourth) (i32.const 47)))
          (then (return (i32.const 4))))
        (if (i32.or (i32.and (i32.eq (local.get $third) (i32.const 42)) (i32.ne (local.get $fourth) (i32.const 42)))
                    (i32.eq (local.get $third) (i32.const 33)))
          (then (return (i32.const 0))))
        (return (i32.const 3))))
    (i32.const 0))

  ;; Block comment depth transition for token pair: /* increments, */ decrements.
  (func (export "pm2_block_comment_depth") (param $depth i32) (param $first i32) (param $second i32) (result i32)
    (if (i32.and (i32.eq (local.get $first) (i32.const 47)) (i32.eq (local.get $second) (i32.const 42)))
      (then (return (i32.add (local.get $depth) (i32.const 1)))))
    (if (i32.and (i32.eq (local.get $first) (i32.const 42)) (i32.eq (local.get $second) (i32.const 47)))
      (then (return (i32.sub (local.get $depth) (i32.const 1)))))
    (local.get $depth))

  ;; Negative literals are accepted only when '-' is followed by an ASCII digit.
  (func (export "pm2_negative_literal_accepts") (param $after_minus i32) (result i32)
    (call $is_digit (local.get $after_minus)))

  ;; Negative literal from compiler stream splits into '-' punct plus positive literal.
  (func (export "pm2_negative_literal_token_count") (param $repr_starts_minus i32) (result i32)
    (select (i32.const 2) (i32.const 1) (local.get $repr_starts_minus)))

  ;; Raw literal validity gates.
  (func (export "pm2_raw_unit_valid") (param $literal_kind i32) (param $char_code i32) (result i32)
    ;; kind 1 raw str, 2 raw byte str, 3 raw C str
    (if (i32.eq (local.get $char_code) (i32.const 13)) (then (return (i32.const 0))))
    (if (i32.and (i32.eq (local.get $literal_kind) (i32.const 2)) (i32.gt_u (local.get $char_code) (i32.const 127)))
      (then (return (i32.const 0))))
    (if (i32.and (i32.eq (local.get $literal_kind) (i32.const 3)) (i32.eqz (local.get $char_code)))
      (then (return (i32.const 0))))
    (i32.const 1))

  ;; Escape errors: warnings 20/21 are nonfatal, all other nonzero errors fatal.
  (func (export "pm2_escape_error_fatal") (param $error_code i32) (result i32)
    (if (i32.eqz (local.get $error_code)) (then (return (i32.const 0))))
    (if (i32.or (i32.eq (local.get $error_code) (i32.const 20)) (i32.eq (local.get $error_code) (i32.const 21)))
      (then (return (i32.const 0))))
    (i32.const 1))

  ;; String display inserts a space unless previous punct was Joint.
  (func (export "pm2_display_needs_space") (param $index i32) (param $previous_joint i32) (result i32)
    (i32.and (i32.gt_u (local.get $index) (i32.const 0)) (i32.eqz (local.get $previous_joint))))

  ;; Float unsuffixed appends .0 when the decimal text has no dot.
  (func (export "pm2_float_unsuffixed_appends_dot_zero") (param $contains_dot i32) (result i32)
    (i32.eqz (local.get $contains_dot)))

  ;; Span location availability is cfg-gated and best-effort diagnostics only.
  (func (export "pm2_span_location_available") (param $span_locations_cfg i32) (param $fuzzing_cfg i32) (result i32)
    (i32.and (local.get $span_locations_cfg) (i32.eqz (local.get $fuzzing_cfg))))

  ;; Wrapper implementation gate: forced fallback overrides compiler wrapper availability.
  (func (export "pm2_impl_kind") (param $wrap_proc_macro i32) (param $forced_fallback i32) (result i32)
    ;; 1 fallback, 2 compiler wrapper.
    (if (local.get $forced_fallback) (then (return (i32.const 1))))
    (select (i32.const 2) (i32.const 1) (local.get $wrap_proc_macro)))
