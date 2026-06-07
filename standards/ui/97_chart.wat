(func (export "er_ui_chart_separator_height") (result f32) f32.const 1)
  (func (export "er_ui_chart_bar_count") (result i32) i32.const 5)
  (func (export "er_ui_chart_grid_count") (result i32) i32.const 3)
  (func (export "er_ui_chart_radius") (result f32) f32.const 8)
  (func (export "er_ui_chart_padding") (result f32) f32.const 8)
  (func (export "er_ui_chart_label_h") (result f32) f32.const 14)
  (func (export "er_ui_chart_label_max_lines") (result i32) i32.const 2)
  (func (export "er_ui_chart_label_gap") (result f32) f32.const 4)
  (func (export "er_ui_chart_bar_gap") (result f32) f32.const 5)
  (func (export "er_ui_chart_bar_radius") (result f32) f32.const 5)
  (func (export "er_ui_chart_plot_min_h") (result f32) f32.const 64)
  (func (export "er_ui_chart_min_width") (result f32) f32.const 120)
  (func (export "er_ui_chart_min_height") (result f32) f32.const 72)
  (func (export "er_ui_chart_grid_height") (result f32) f32.const 1)

  (func $er_ui_chart_bar_value (export "er_ui_chart_bar_value") (param $index i32) (result f32)
    local.get $index i32.const 0 i32.const 4 call $clamp_i32
    if (result f32)
      local.get $index i32.const 1 i32.eq
      if (result f32)
        f32.const 0.72
      else
        local.get $index i32.const 2 i32.eq
        if (result f32)
          f32.const 0.38
        else
          local.get $index i32.const 3 i32.eq
          if (result f32)
            f32.const 0.86
          else
            local.get $index i32.const 4 i32.eq
            if (result f32)
              f32.const 0.62
            else
              f32.const 0.45
            end
          end
        end
      end
    else
      f32.const 0.45
    end)

  (func $er_ui_chart_label_height (param $label_ptr i32) (param $label_len i32) (param $width f32) (param $bounds_h f32) (result f32)
    local.get $label_len i32.eqz
    if (result f32)
      f32.const 14
    else
      local.get $label_ptr local.get $label_len local.get $width f32.const 14 i32.const 2 call $er_ui_alert_measured_text_height
    end
    local.get $bounds_h f32.const 16 f32.sub f32.const 1 call $max_f32
    call $min_f32)

  (func $er_ui_chart_label_bounds (export "er_ui_chart_label_bounds") (param $bounds i32) (param $label_ptr i32) (param $label_len i32) (param $out i32) (result i32)
    (local $w f32) (local $h f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds i32.const 8 i32.add f32.load f32.const 16 f32.sub f32.const 1 call $max_f32 local.set $w
    local.get $label_ptr local.get $label_len local.get $w local.get $bounds i32.const 12 i32.add f32.load call $er_ui_chart_label_height local.set $h
    local.get $out
    local.get $bounds f32.load f32.const 8 f32.add
    local.get $bounds i32.const 4 i32.add f32.load f32.const 8 f32.add
    local.get $w
    local.get $h
    call $rect_store)

  (func $er_ui_chart_plot_bounds (export "er_ui_chart_plot_bounds") (param $bounds i32) (param $label_ptr i32) (param $label_len i32) (param $out i32) (result i32)
    (local $label_h f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds local.get $label_ptr local.get $label_len i32.const 123200 call $er_ui_chart_label_bounds drop
    i32.const 123200 i32.const 12 i32.add f32.load local.set $label_h
    local.get $out
    local.get $bounds f32.load f32.const 8 f32.add
    local.get $bounds i32.const 4 i32.add f32.load f32.const 12 f32.add local.get $label_h f32.add
    local.get $bounds i32.const 8 i32.add f32.load f32.const 16 f32.sub f32.const 1 call $max_f32
    local.get $bounds i32.const 12 i32.add f32.load f32.const 20 f32.sub local.get $label_h f32.sub f32.const 1 call $max_f32
    call $rect_store)

  (func (export "er_ui_chart_grid_line_bounds") (param $bounds i32) (param $label_ptr i32) (param $label_len i32) (param $index i32) (param $out i32) (result i32)
    (local $grid_y f32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds local.get $label_ptr local.get $label_len i32.const 123216 call $er_ui_chart_plot_bounds drop
    i32.const 123216 i32.const 4 i32.add f32.load
    i32.const 123216 i32.const 12 i32.add f32.load
    local.get $index i32.const 0 i32.const 2 call $clamp_i32 i32.const 1 i32.add f32.convert_i32_u
    f32.const 4
    f32.div
    f32.mul
    f32.add
    local.set $grid_y
    local.get $out
    i32.const 123216 f32.load
    local.get $grid_y
    i32.const 123216 i32.const 8 i32.add f32.load
    f32.const 1
    call $rect_store)

  (func (export "er_ui_chart_baseline_bounds") (param $bounds i32) (param $label_ptr i32) (param $label_len i32) (param $out i32) (result i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $bounds local.get $label_ptr local.get $label_len i32.const 123232 call $er_ui_chart_plot_bounds drop
    local.get $out
    i32.const 123232 f32.load
    i32.const 123232 i32.const 4 i32.add f32.load i32.const 123232 i32.const 12 i32.add f32.load f32.add f32.const 1 f32.sub
    i32.const 123232 i32.const 8 i32.add f32.load
    f32.const 1
    call $rect_store)

  (func (export "er_ui_chart_bar_bounds") (param $bounds i32) (param $label_ptr i32) (param $label_len i32) (param $index i32) (param $out i32) (result i32)
    (local $bar_w f32) (local $h f32) (local $idx i32)
    local.get $bounds i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    local.get $index i32.const 0 i32.const 4 call $clamp_i32 local.set $idx
    local.get $bounds local.get $label_ptr local.get $label_len i32.const 123248 call $er_ui_chart_plot_bounds drop
    i32.const 123248 i32.const 8 i32.add f32.load f32.const 20 f32.sub f32.const 5 f32.div f32.const 1 call $max_f32 local.set $bar_w
    i32.const 123248 i32.const 12 i32.add f32.load local.get $idx call $er_ui_chart_bar_value f32.mul f32.const 1 call $max_f32 local.set $h
    local.get $out
    i32.const 123248 f32.load local.get $idx f32.convert_i32_u local.get $bar_w f32.const 5 f32.add f32.mul f32.add
    i32.const 123248 i32.const 4 i32.add f32.load i32.const 123248 i32.const 12 i32.add f32.load f32.add local.get $h f32.sub
    local.get $bar_w
    local.get $h
    call $rect_store)

  (func (export "er_ui_chart_measure") (param $label_ptr i32) (param $label_len i32) (param $constraints i32) (param $out i32) (result i32)
    (local $label_w f32) (local $label_h f32) (local $pref_w f32) (local $pref_h f32)
    local.get $constraints i32.eqz local.get $out i32.eqz i32.or
    if i32.const 0 return end
    i32.const 123264 f32.const 8 f32.store
    i32.const 123268 f32.const 8 f32.store
    i32.const 123272 f32.const 8 f32.store
    i32.const 123276 f32.const 8 f32.store
    local.get $constraints i32.const 123264 i32.const 123280 call $er_ui_layout_constraints_inner drop
    i32.const 123304 f32.const 14 f32.store
    i32.const 123308 local.get $label_ptr local.get $label_len f32.const 14 call $primitives_average_width f32.store
    i32.const 123312 i32.const 2 i32.store
    local.get $label_ptr local.get $label_len i32.const 123280 i32.const 123304 i32.const 123316 call $er_ui_text_component_measure_value drop
    i32.const 123324 f32.load local.set $label_w
    i32.const 123328 f32.load local.set $label_h
    f32.const 120 local.get $label_w f32.const 16 f32.add call $max_f32 local.set $pref_w
    f32.const 16 local.get $label_h f32.add f32.const 4 f32.add f32.const 64 f32.add local.set $pref_h
    local.get $pref_w local.get $pref_h local.get $constraints i32.const 123340 call $er_ui_primitives_constrain_preferred_size drop
    i32.const 123340 f32.load local.set $pref_w
    i32.const 123344 f32.load local.set $pref_h
    f32.const 120 local.get $pref_w call $min_f32
    f32.const 72 local.get $pref_h call $min_f32
    local.get $pref_w local.get $pref_h
    local.get $constraints local.get $pref_w call $er_ui_layout_axis_constraint_limit
    local.get $pref_h
    local.get $out
    call $er_ui_layout_measurement_flexible drop
    local.get $out local.get $constraints local.get $out call $er_ui_layout_measurement_apply_exact)
