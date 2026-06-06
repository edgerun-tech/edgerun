(module
  (func (export "select_role_caps") (param $role i32) (result i32)
    (block $invalid
      (block $full
        (block $peer
          (block $rend
            (block $intro
              (block $service
                (block $bridge
                  (block $dir
                    (block $exit
                      (block $relay
                        (block $client
                          (br_table $client $relay $relay $exit $dir $bridge $service $intro $rend $peer $full $invalid
                            (local.get $role)))
                        (return (i32.const 1)))
                      (return (i32.const 2)))
                    (return (i32.const 6)))
                  (return (i32.const 8)))
                (return (i32.const 18)))
              (return (i32.const 32)))
            (return (i32.const 64)))
          (return (i32.const 128)))
        (return (i32.const 33)))
      (return (i32.const 227)))
    (i32.const -1))
)
