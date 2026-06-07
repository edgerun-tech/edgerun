(func $pack (param $a i32) (param $b i32) (param $c i32) (param $d i32) (result i32)
    local.get $a)

  ;; Status values:
  ;;   0 ok, 1 missing CRLF, 2 bare LF, 3 bare CR, 4 empty line
  (func (export "mail_crlf_validate") (param $ptr i32) (param $len i32) (result i32)
    (local $i i32)
    (local $b i32)
    (if (i32.eqz (local.get $len)) (then (return (i32.const 4))))
    (if
      (i32.or
        (i32.lt_u (local.get $len) (i32.const 2))
        (i32.ne
          (i32.load8_u
            (i32.add
              (local.get $ptr)
              (i32.sub (local.get $len) (i32.const 2))))
          (i32.const 13)))
      (then (return (i32.const 1))))
    (if
      (i32.ne
        (i32.load8_u
          (i32.add
            (local.get $ptr)
            (i32.sub (local.get $len) (i32.const 1))))
        (i32.const 10))
      (then (return (i32.const 1))))

    (block $done
      (loop $scan
        (br_if $done (i32.ge_u (local.get $i) (i32.sub (local.get $len) (i32.const 2))))
        (local.set $b (i32.load8_u (i32.add (local.get $ptr) (local.get $i))))
        (if (i32.eq (local.get $b) (i32.const 10)) (then (return (i32.const 2))))
        (if (i32.eq (local.get $b) (i32.const 13)) (then (return (i32.const 3))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $scan)))
    i32.const 0)

  (func $m131is_space (param $b i32) (result i32)
    (i32.or (i32.eq (local.get $b) (i32.const 32)) (i32.eq (local.get $b) (i32.const 9))))

  (func $hash_token (param $ptr i32) (param $off i32) (param $len i32) (result i32)
    (local $i i32)
    (local $h i32)
    (loop $loop
      (if (i32.ge_u (local.get $i) (local.get $len)) (then (return (local.get $h))))
      (local.set $h
        (i32.add
          (i32.mul (local.get $h) (i32.const 33))
          (call to_upper (i32.load8_u (i32.add (i32.add (local.get $ptr) (local.get $off)) (local.get $i))))))
      (local.set $i (i32.add (local.get $i) (i32.const 1)))
      (br $loop))
    local.get $h)

  (func $first_token_start (param $ptr i32) (param $len i32) (result i32)
    (local $i i32)
    (loop $loop
      (if (i32.ge_u (local.get $i) (local.get $len)) (then (return (local.get $len))))
      (if (i32.eqz (call $m131is_space (i32.load8_u (i32.add (local.get $ptr) (local.get $i)))))
        (then (return (local.get $i))))
      (local.set $i (i32.add (local.get $i) (i32.const 1)))
      (br $loop))
    local.get $len)

  (func $token_len (param $ptr i32) (param $len i32) (param $off i32) (result i32)
    (local $i i32)
    (local.set $i (local.get $off))
    (loop $loop
      (if
        (i32.or
          (i32.ge_u (local.get $i) (local.get $len))
          (call $m131is_space (i32.load8_u (i32.add (local.get $ptr) (local.get $i)))))
        (then (return (i32.sub (local.get $i) (local.get $off)))))
      (local.set $i (i32.add (local.get $i) (i32.const 1)))
      (br $loop))
    i32.const 0)

  (func $second_token_start (param $ptr i32) (param $len i32) (result i32)
    (local $i i32)
    (local.set $i (call $first_token_start (local.get $ptr) (local.get $len)))
    (local.set $i (i32.add (local.get $i) (call $token_len (local.get $ptr) (local.get $len) (local.get $i))))
    (loop $skip
      (if (i32.ge_u (local.get $i) (local.get $len)) (then (return (local.get $len))))
      (if (i32.eqz (call $m131is_space (i32.load8_u (i32.add (local.get $ptr) (local.get $i)))))
        (then (return (local.get $i))))
      (local.set $i (i32.add (local.get $i) (i32.const 1)))
      (br $skip))
    local.get $len)

  (func $class_by_len_hash (param $len i32) (param $h i32) (param $smtp i32) (result i32)
    ;; IMAP command ids.
    (if (i32.eqz (local.get $smtp))
      (then
        (if (i32.and (i32.eq (local.get $len) (i32.const 10)) (i32.eq (local.get $h) (i32.const 2713781506))) (then (return (i32.const 1))))
        (if (i32.and (i32.eq (local.get $len) (i32.const 4)) (i32.eq (local.get $h) (i32.const 2891804))) (then (return (i32.const 2))))
        (if (i32.and (i32.eq (local.get $len) (i32.const 6)) (i32.eq (local.get $h) (i32.const 3070618074))) (then (return (i32.const 3))))
        (if (i32.and (i32.eq (local.get $len) (i32.const 8)) (i32.eq (local.get $h) (i32.const 2031001025))) (then (return (i32.const 4))))
        (if (i32.and (i32.eq (local.get $len) (i32.const 5)) (i32.eq (local.get $h) (i32.const 93048825))) (then (return (i32.const 5))))
        (if (i32.and (i32.eq (local.get $len) (i32.const 12)) (i32.eq (local.get $h) (i32.const 3074907487))) (then (return (i32.const 6))))
        (if (i32.and (i32.eq (local.get $len) (i32.const 6)) (i32.eq (local.get $h) (i32.const 3332874816))) (then (return (i32.const 7))))
        (if (i32.and (i32.eq (local.get $len) (i32.const 7)) (i32.eq (local.get $h) (i32.const 2440825383))) (then (return (i32.const 8))))
        (if (i32.and (i32.eq (local.get $len) (i32.const 6)) (i32.eq (local.get $h) (i32.const 2721870132))) (then (return (i32.const 9))))
        (if (i32.and (i32.eq (local.get $len) (i32.const 6)) (i32.eq (local.get $h) (i32.const 2745844467))) (then (return (i32.const 10))))
        (if (i32.and (i32.eq (local.get $len) (i32.const 6)) (i32.eq (local.get $h) (i32.const 3293807256))) (then (return (i32.const 11))))
        (if (i32.and (i32.eq (local.get $len) (i32.const 9)) (i32.eq (local.get $h) (i32.const 3577661474))) (then (return (i32.const 12))))
        (if (i32.and (i32.eq (local.get $len) (i32.const 11)) (i32.eq (local.get $h) (i32.const 1480535749))) (then (return (i32.const 13))))
        (if (i32.and (i32.eq (local.get $len) (i32.const 4)) (i32.eq (local.get $h) (i32.const 2813532))) (then (return (i32.const 14))))
        (if (i32.and (i32.eq (local.get $len) (i32.const 4)) (i32.eq (local.get $h) (i32.const 2824470))) (then (return (i32.const 15))))
        (if (i32.and (i32.eq (local.get $len) (i32.const 6)) (i32.eq (local.get $h) (i32.const 3350285252))) (then (return (i32.const 16))))
        (if (i32.and (i32.eq (local.get $len) (i32.const 6)) (i32.eq (local.get $h) (i32.const 2641626968))) (then (return (i32.const 17))))
        (if (i32.and (i32.eq (local.get $len) (i32.const 5)) (i32.eq (local.get $h) (i32.const 82121598))) (then (return (i32.const 18))))
        (if (i32.and (i32.eq (local.get $len) (i32.const 5)) (i32.eq (local.get $h) (i32.const 82276758))) (then (return (i32.const 19))))
        (if (i32.and (i32.eq (local.get $len) (i32.const 7)) (i32.eq (local.get $h) (i32.const 2458906908))) (then (return (i32.const 20))))
        (if (i32.and (i32.eq (local.get $len) (i32.const 6)) (i32.eq (local.get $h) (i32.const 3332493654))) (then (return (i32.const 21))))
        (if (i32.and (i32.eq (local.get $len) (i32.const 5)) (i32.eq (local.get $h) (i32.const 85587882))) (then (return (i32.const 22))))
        (if (i32.and (i32.eq (local.get $len) (i32.const 5)) (i32.eq (local.get $h) (i32.const 101538957))) (then (return (i32.const 23))))
        (if (i32.and (i32.eq (local.get $len) (i32.const 4)) (i32.eq (local.get $h) (i32.const 2496539))) (then (return (i32.const 24))))
        (if (i32.and (i32.eq (local.get $len) (i32.const 3)) (i32.eq (local.get $h) (i32.const 95042))) (then (return (i32.const 25))))
        (if (i32.and (i32.eq (local.get $len) (i32.const 4)) (i32.eq (local.get $h) (i32.const 2700030))) (then (return (i32.const 26))))
        (if (i32.and (i32.eq (local.get $len) (i32.const 4)) (i32.eq (local.get $h) (i32.const 2532390))) (then (return (i32.const 27))))
        (if (i32.and (i32.eq (local.get $len) (i32.const 6)) (i32.eq (local.get $h) (i32.const 2795254311))) (then (return (i32.const 28))))
        (if (i32.and (i32.eq (local.get $len) (i32.const 8)) (i32.eq (local.get $h) (i32.const 2898383811))) (then (return (i32.const 29))))
        (if (i32.and (i32.eq (local.get $len) (i32.const 4)) (i32.eq (local.get $h) (i32.const 2856087))) (then (return (i32.const 30))))
        (if (i32.and (i32.eq (local.get $len) (i32.const 9)) (i32.eq (local.get $h) (i32.const 1222943341))) (then (return (i32.const 31))))
        (if (i32.and (i32.eq (local.get $len) (i32.const 5)) (i32.eq (local.get $h) (i32.const 99203114))) (then (return (i32.const 32))))
        (if (i32.and (i32.eq (local.get $len) (i32.const 8)) (i32.eq (local.get $h) (i32.const 581267958))) (then (return (i32.const 33))))
        (if (i32.and (i32.eq (local.get $len) (i32.const 4)) (i32.eq (local.get $h) (i32.const 3071592))) (then (return (i32.const 34))))
        (if (i32.and (i32.eq (local.get $len) (i32.const 6)) (i32.eq (local.get $h) (i32.const 3375783512))) (then (return (i32.const 35))))
        (if (i32.and (i32.eq (local.get $len) (i32.const 2)) (i32.eq (local.get $h) (i32.const 2477))) (then (return (i32.const 36))))
        (return (i32.const 0))))

    ;; SMTP command ids.
    (if (i32.and (i32.eq (local.get $len) (i32.const 4)) (i32.eq (local.get $h) (i32.const 2560648))) (then (return (i32.const 1))))
    (if (i32.and (i32.eq (local.get $len) (i32.const 4)) (i32.eq (local.get $h) (i32.const 2665192))) (then (return (i32.const 2))))
    (if (i32.and (i32.eq (local.get $len) (i32.const 4)) (i32.eq (local.get $h) (i32.const 2840419))) (then (return (i32.const 3))))
    (if (i32.and (i32.eq (local.get $len) (i32.const 4)) (i32.eq (local.get $h) (i32.const 3022521))) (then (return (i32.const 4))))
    (if (i32.and (i32.eq (local.get $len) (i32.const 4)) (i32.eq (local.get $h) (i32.const 2517338))) (then (return (i32.const 5))))
    (if (i32.and (i32.eq (local.get $len) (i32.const 4)) (i32.eq (local.get $h) (i32.const 2448123))) (then (return (i32.const 6))))
    (if (i32.and (i32.eq (local.get $len) (i32.const 4)) (i32.eq (local.get $h) (i32.const 3039582))) (then (return (i32.const 7))))
    (if (i32.and (i32.eq (local.get $len) (i32.const 4)) (i32.eq (local.get $h) (i32.const 2891804))) (then (return (i32.const 8))))
    (if (i32.and (i32.eq (local.get $len) (i32.const 4)) (i32.eq (local.get $h) (i32.const 3005955))) (then (return (i32.const 9))))
    (if (i32.and (i32.eq (local.get $len) (i32.const 8)) (i32.eq (local.get $h) (i32.const 2031001025))) (then (return (i32.const 10))))
    (if (i32.and (i32.eq (local.get $len) (i32.const 4)) (i32.eq (local.get $h) (i32.const 3182279))) (then (return (i32.const 11))))
    (if (i32.and (i32.eq (local.get $len) (i32.const 4)) (i32.eq (local.get $h) (i32.const 2578203))) (then (return (i32.const 12))))
    (if (i32.and (i32.eq (local.get $len) (i32.const 4)) (i32.eq (local.get $h) (i32.const 2665193))) (then (return (i32.const 13))))
    (if (i32.and (i32.eq (local.get $len) (i32.const 4)) (i32.eq (local.get $h) (i32.const 2431314))) (then (return (i32.const 14))))
    (if (i32.and (i32.eq (local.get $len) (i32.const 4)) (i32.eq (local.get $h) (i32.const 3114057))) (then (return (i32.const 15))))
    (if (i32.and (i32.eq (local.get $len) (i32.const 4)) (i32.eq (local.get $h) (i32.const 2573913))) (then (return (i32.const 16))))
    i32.const 0)

  ;; IMAP lines are TAG COMMAND args. Returns command id above, or 0.
  (func $imap_classify_line (export "imap_classify_line") (param $ptr i32) (param $len i32) (result i32)
    (local $off i32)
    (local $n i32)
    (local.set $off (call $second_token_start (local.get $ptr) (local.get $len)))
    (local.set $n (call $token_len (local.get $ptr) (local.get $len) (local.get $off)))
    (if (i32.eqz (local.get $n)) (then (return (i32.const 0))))
    (call $class_by_len_hash
      (local.get $n)
      (call $hash_token (local.get $ptr) (local.get $off) (local.get $n))
      (i32.const 0)))

  ;; SMTP lines are COMMAND args. Returns command id above, or 0.
  (func $smtp_classify_line (export "smtp_classify_line") (param $ptr i32) (param $len i32) (result i32)
    (local $off i32)
    (local $n i32)
    (local.set $off (call $first_token_start (local.get $ptr) (local.get $len)))
    (local.set $n (call $token_len (local.get $ptr) (local.get $len) (local.get $off)))
    (if (i32.eqz (local.get $n)) (then (return (i32.const 0))))
    (call $class_by_len_hash
      (local.get $n)
      (call $hash_token (local.get $ptr) (local.get $off) (local.get $n))
      (i32.const 1)))

  ;; Mailbox/path validator for portable mail protocol state:
  ;; non-empty, relative, no NUL/CR/LF, no backslash, no repeated slash,
  ;; no "." or ".." segment. INBOX and nested names like Work/2026 are valid.
  (func (export "mailbox_path_valid") (param $ptr i32) (param $len i32) (result i32)
    (local $i i32)
    (local $seg_start i32)
    (local $b i32)
    (if (i32.eqz (local.get $len)) (then (return (i32.const 0))))
    (if (i32.eq (i32.load8_u (local.get $ptr)) (i32.const 47)) (then (return (i32.const 0))))
    (loop $loop
      (if (i32.ge_u (local.get $i) (local.get $len)) (then (return (i32.const 1))))
      (local.set $b (i32.load8_u (i32.add (local.get $ptr) (local.get $i))))
      (if
        (i32.or
          (i32.or (i32.eqz (local.get $b)) (i32.eq (local.get $b) (i32.const 13)))
          (i32.or (i32.eq (local.get $b) (i32.const 10)) (i32.eq (local.get $b) (i32.const 92))))
        (then (return (i32.const 0))))
      (if (i32.eq (local.get $b) (i32.const 47))
        (then
          (if (i32.eq (local.get $i) (local.get $seg_start)) (then (return (i32.const 0))))
          (if
            (i32.and
              (i32.eq (i32.sub (local.get $i) (local.get $seg_start)) (i32.const 1))
              (i32.eq (i32.load8_u (i32.add (local.get $ptr) (local.get $seg_start))) (i32.const 46)))
            (then (return (i32.const 0))))
          (if
            (i32.and
              (i32.eq (i32.sub (local.get $i) (local.get $seg_start)) (i32.const 2))
              (i32.and
                (i32.eq (i32.load8_u (i32.add (local.get $ptr) (local.get $seg_start))) (i32.const 46))
                (i32.eq (i32.load8_u (i32.add (i32.add (local.get $ptr) (local.get $seg_start)) (i32.const 1))) (i32.const 46))))
            (then (return (i32.const 0))))
          (local.set $seg_start (i32.add (local.get $i) (i32.const 1)))))
      (local.set $i (i32.add (local.get $i) (i32.const 1)))
      (br $loop))
    i32.const 1)

  ;; IMAP states: 0 not-authenticated, 1 authenticated, 2 selected, 3 logout.
  ;; IMAP statuses: 0 OK, 1 NO, 2 BAD. Actions: 0 continue, 1 logout.
  (func (export "imap_session_transition")
    (param $state i32)
    (param $cmd i32)
    (param $mailbox_valid i32)
    (result i32)
    (if (i32.eq (local.get $cmd) (i32.const 0)) (then (return (call $pack (i32.const 2) (local.get $state) (i32.const 0) (i32.const 0)))))
    (if (i32.or (i32.eq (local.get $cmd) (i32.const 1)) (i32.eq (local.get $cmd) (i32.const 2)))
      (then (return (call $pack (i32.const 0) (local.get $state) (i32.const 0) (i32.const 0)))))
    (if (i32.eq (local.get $cmd) (i32.const 3))
      (then (return (call $pack (i32.const 0) (i32.const 3) (i32.const 0) (i32.const 1)))))
    (if (i32.or (i32.eq (local.get $cmd) (i32.const 4)) (i32.eq (local.get $cmd) (i32.const 5)))
      (then (return (call $pack (i32.const 2) (local.get $state) (i32.const 0) (i32.const 0)))))
    (if (i32.eq (local.get $cmd) (i32.const 6))
      (then
        (if (i32.eq (local.get $state) (i32.const 0))
          (then (return (call $pack (i32.const 1) (local.get $state) (i32.const 0) (i32.const 0))))
          (else (return (call $pack (i32.const 1) (local.get $state) (i32.const 0) (i32.const 0)))))))
    (if (i32.or (i32.eq (local.get $cmd) (i32.const 7)) (i32.eq (local.get $cmd) (i32.const 8)))
      (then
        (if (i32.and (i32.eq (local.get $state) (i32.const 1)) (local.get $mailbox_valid))
          (then (return (call $pack (i32.const 0) (i32.const 2) (i32.const 0) (i32.const 0)))))
        (return (call $pack (i32.const 1) (local.get $state) (i32.const 0) (i32.const 0)))))
    (if (i32.or (i32.eq (local.get $cmd) (i32.const 19)) (i32.eq (local.get $cmd) (i32.const 29)))
      (then
        (if (i32.eq (local.get $state) (i32.const 2))
          (then (return (call $pack (i32.const 0) (i32.const 1) (i32.const 0) (i32.const 0)))))
        (return (call $pack (i32.const 1) (local.get $state) (i32.const 0) (i32.const 0)))))
    (if
      (i32.or
        (i32.or (i32.or (i32.eq (local.get $cmd) (i32.const 18)) (i32.eq (local.get $cmd) (i32.const 20))) (i32.or (i32.eq (local.get $cmd) (i32.const 21)) (i32.eq (local.get $cmd) (i32.const 22))))
        (i32.or (i32.or (i32.eq (local.get $cmd) (i32.const 23)) (i32.eq (local.get $cmd) (i32.const 24))) (i32.or (i32.eq (local.get $cmd) (i32.const 26)) (i32.eq (local.get $cmd) (i32.const 30)))))
      (then
        (if (i32.eq (local.get $state) (i32.const 2))
          (then (return (call $pack (i32.const 0) (local.get $state) (i32.const 0) (i32.const 0)))))
        (return (call $pack (i32.const 1) (local.get $state) (i32.const 0) (i32.const 0)))))
    (if (i32.eq (local.get $cmd) (i32.const 27))
      (then (return (call $pack (i32.const 2) (local.get $state) (i32.const 0) (i32.const 0)))))
    (if (i32.eq (local.get $state) (i32.const 0))
      (then (return (call $pack (i32.const 1) (local.get $state) (i32.const 0) (i32.const 0)))))
    (call $pack (i32.const 0) (local.get $state) (i32.const 0) (i32.const 0)))

  (func (export "imap_status_code") (param $status i32) (result i32)
    ;; 0 OK, 1 NO, 2 BAD. Returned as ASCII triples packed little-endian.
    (if (i32.eq (local.get $status) (i32.const 0)) (then (return (i32.const 0x004b4f))))
    (if (i32.eq (local.get $status) (i32.const 1)) (then (return (i32.const 0x004f4e))))
    (if (i32.eq (local.get $status) (i32.const 2)) (then (return (i32.const 0x444142))))
    i32.const 0)

  ;; AUTH mechanism from an SMTP AUTH line: 0 none, 1 PLAIN, 2 LOGIN, 3 other or missing.
  (func (export "smtp_auth_mechanism_class") (param $ptr i32) (param $len i32) (result i32)
    (local $off i32)
    (local $n i32)
    (local $h i32)
    (if (i32.ne (call $smtp_classify_line (local.get $ptr) (local.get $len)) (i32.const 14))
      (then (return (i32.const 0))))
    (local.set $off (call $second_token_start (local.get $ptr) (local.get $len)))
    (local.set $n (call $token_len (local.get $ptr) (local.get $len) (local.get $off)))
    (if (i32.eqz (local.get $n)) (then (return (i32.const 3))))
    (local.set $h (call $hash_token (local.get $ptr) (local.get $off) (local.get $n)))
    (if (i32.and (i32.eq (local.get $n) (i32.const 5)) (i32.eq (local.get $h) (i32.const 97678164))) (then (return (i32.const 1))))
    (if (i32.and (i32.eq (local.get $n) (i32.const 5)) (i32.eq (local.get $h) (i32.const 93048825))) (then (return (i32.const 2))))
    i32.const 3)

  (func (export "smtp_auth_has_initial_response") (param $ptr i32) (param $len i32) (result i32)
    (local $off i32)
    (local.set $off (call $second_token_start (local.get $ptr) (local.get $len)))
    (local.set $off (i32.add (local.get $off) (call $token_len (local.get $ptr) (local.get $len) (local.get $off))))
    (loop $skip
      (if (i32.ge_u (local.get $off) (local.get $len)) (then (return (i32.const 0))))
      (if (i32.eqz (call $m131is_space (i32.load8_u (i32.add (local.get $ptr) (local.get $off)))))
        (then (return (i32.const 1))))
      (local.set $off (i32.add (local.get $off) (i32.const 1)))
      (br $skip))
    i32.const 0)

  ;; SMTP states: 0 connected, 1 ready, 2 mail-set, 3 rcpt-set, 4 data, 5 quit.
  ;; Status is the numeric SMTP response code in low 16 bits. Auth exchange:
  ;; 0 idle, 1 PLAIN response pending, 2 LOGIN username pending, 3 LOGIN token pending.
  ;; Action: 0 continue, 1 quit, 2 starttls, 3 read-bdat, 4 deliver.
  (func (export "smtp_session_transition")
    (param $state i32)
    (param $cmd i32)
    (param $authenticated i32)
    (param $tls_active i32)
    (param $starttls_available i32)
    (param $tls_configured i32)
    (param $require_auth i32)
    (param $auth_mech i32)
    (param $auth_initial i32)
    (result i32)
    (if (i32.or (i32.eq (local.get $cmd) (i32.const 1)) (i32.eq (local.get $cmd) (i32.const 2)))
      (then (return (call $pack (i32.const 250) (i32.const 1) (i32.const 0) (i32.const 0)))))
    (if (i32.eq (local.get $cmd) (i32.const 3))
      (then
        (if (i32.eqz (i32.or (i32.eq (local.get $state) (i32.const 1)) (i32.eq (local.get $state) (i32.const 2))))
          (then (return (call $pack (i32.const 503) (local.get $state) (i32.const 0) (i32.const 0)))))
        (if (i32.and (local.get $require_auth) (i32.eqz (local.get $authenticated)))
          (then (return (call $pack (i32.const 578) (local.get $state) (i32.const 0) (i32.const 0)))))
        (return (call $pack (i32.const 250) (i32.const 2) (i32.const 0) (i32.const 0)))))
    (if (i32.eq (local.get $cmd) (i32.const 4))
      (then
        (if (i32.eqz (i32.or (i32.eq (local.get $state) (i32.const 2)) (i32.eq (local.get $state) (i32.const 3))))
          (then (return (call $pack (i32.const 503) (local.get $state) (i32.const 0) (i32.const 0)))))
        (return (call $pack (i32.const 250) (i32.const 3) (i32.const 0) (i32.const 0)))))
    (if (i32.eq (local.get $cmd) (i32.const 5))
      (then
        (if (i32.ne (local.get $state) (i32.const 3))
          (then (return (call $pack (i32.const 503) (local.get $state) (i32.const 0) (i32.const 0)))))
        (return (call $pack (i32.const 354) (i32.const 4) (i32.const 0) (i32.const 0)))))
    (if (i32.eq (local.get $cmd) (i32.const 6))
      (then
        (if (i32.eqz (i32.or (i32.eq (local.get $state) (i32.const 3)) (i32.eq (local.get $state) (i32.const 4))))
          (then (return (call $pack (i32.const 503) (local.get $state) (i32.const 0) (i32.const 0)))))
        (return (call $pack (i32.const 250) (i32.const 4) (i32.const 0) (i32.const 3)))))
    (if (i32.eq (local.get $cmd) (i32.const 7)) (then (return (call $pack (i32.const 250) (i32.const 1) (i32.const 0) (i32.const 0)))))
    (if (i32.eq (local.get $cmd) (i32.const 8)) (then (return (call $pack (i32.const 250) (local.get $state) (i32.const 0) (i32.const 0)))))
    (if (i32.eq (local.get $cmd) (i32.const 9)) (then (return (call $pack (i32.const 221) (i32.const 5) (i32.const 0) (i32.const 1)))))
    (if (i32.eq (local.get $cmd) (i32.const 10))
      (then
        (if (local.get $tls_active) (then (return (call $pack (i32.const 503) (local.get $state) (i32.const 0) (i32.const 0)))))
        (if (i32.eqz (local.get $starttls_available)) (then (return (call $pack (i32.const 551) (local.get $state) (i32.const 0) (i32.const 0)))))
        (return (call $pack (i32.const 250) (local.get $state) (i32.const 0) (i32.const 2)))))
    (if (i32.or (i32.eq (local.get $cmd) (i32.const 11)) (i32.or (i32.eq (local.get $cmd) (i32.const 12)) (i32.eq (local.get $cmd) (i32.const 15))))
      (then (return (call $pack (i32.const 551) (local.get $state) (i32.const 0) (i32.const 0)))))
    (if (i32.eq (local.get $cmd) (i32.const 13)) (then (return (call $pack (i32.const 211) (local.get $state) (i32.const 0) (i32.const 0)))))
    (if (i32.eq (local.get $cmd) (i32.const 16)) (then (return (call $pack (i32.const 250) (local.get $state) (i32.const 0) (i32.const 0)))))
    (if (i32.eq (local.get $cmd) (i32.const 14))
      (then
        (if (i32.and (i32.eqz (local.get $tls_active)) (local.get $tls_configured))
          (then (return (call $pack (i32.const 503) (local.get $state) (i32.const 0) (i32.const 0)))))
        (if (local.get $authenticated)
          (then (return (call $pack (i32.const 503) (local.get $state) (i32.const 0) (i32.const 0)))))
        (if (i32.eq (local.get $auth_mech) (i32.const 1))
          (then
            (if (local.get $auth_initial)
              (then (return (call $pack (i32.const 250) (local.get $state) (i32.const 0) (i32.const 0))))
              (else (return (call $pack (i32.const 334) (local.get $state) (i32.const 1) (i32.const 0)))))))
        (if (i32.eq (local.get $auth_mech) (i32.const 2))
          (then (return (call $pack (i32.const 334) (local.get $state) (i32.const 2) (i32.const 0)))))
        (return (call $pack (i32.const 578) (local.get $state) (i32.const 0) (i32.const 0)))))
    (call $pack (i32.const 554) (local.get $state) (i32.const 0) (i32.const 0)))

  ;; AUTH exchange result input: 0 success, 1 failed, 2 unsupported, 3 invalid.
  (func (export "smtp_auth_exchange_transition")
    (param $exchange i32)
    (param $result i32)
    (result i32)
    (if (i32.eq (local.get $exchange) (i32.const 0))
      (then (return (call $pack (i32.const 554) (i32.const 0) (i32.const 0) (i32.const 0)))))
    (if (i32.eq (local.get $exchange) (i32.const 2))
      (then (return (call $pack (i32.const 334) (i32.const 0) (i32.const 3) (i32.const 0)))))
    (if (i32.eq (local.get $result) (i32.const 0))
      (then (return (call $pack (i32.const 250) (i32.const 0) (i32.const 0) (i32.const 0)))))
    (if (i32.eq (local.get $result) (i32.const 1))
      (then (return (call $pack (i32.const 578) (i32.const 0) (i32.const 0) (i32.const 0)))))
    (if (i32.eq (local.get $result) (i32.const 2))
      (then (return (call $pack (i32.const 578) (i32.const 0) (i32.const 0) (i32.const 0)))))
    (call $pack (i32.const 554) (i32.const 0) (i32.const 0) (i32.const 0)))

  (func (export "smtp_response_kind_code") (param $kind i32) (result i32)
    ;; Mirrors SmtpResponseCode constants used by session.rs.
    (if (i32.eq (local.get $kind) (i32.const 0)) (then (return (i32.const 220))))
    (if (i32.eq (local.get $kind) (i32.const 1)) (then (return (i32.const 221))))
    (if (i32.eq (local.get $kind) (i32.const 2)) (then (return (i32.const 250))))
    (if (i32.eq (local.get $kind) (i32.const 3)) (then (return (i32.const 211))))
    (if (i32.eq (local.get $kind) (i32.const 4)) (then (return (i32.const 354))))
    (if (i32.eq (local.get $kind) (i32.const 5)) (then (return (i32.const 334))))
    (if (i32.eq (local.get $kind) (i32.const 6)) (then (return (i32.const 421))))
    (if (i32.eq (local.get $kind) (i32.const 7)) (then (return (i32.const 453))))
    (if (i32.eq (local.get $kind) (i32.const 8)) (then (return (i32.const 554))))
    (if (i32.eq (local.get $kind) (i32.const 9)) (then (return (i32.const 552))))
    (if (i32.eq (local.get $kind) (i32.const 10)) (then (return (i32.const 551))))
    (if (i32.eq (local.get $kind) (i32.const 11)) (then (return (i32.const 503))))
    (if (i32.eq (local.get $kind) (i32.const 12)) (then (return (i32.const 511))))
    (if (i32.eq (local.get $kind) (i32.const 13)) (then (return (i32.const 578))))
    i32.const 0)

  (func (export "smtp_response_class") (param $code i32) (result i32)
    (i32.div_u (local.get $code) (i32.const 100)))

  (func (export "mail_spf_qualifier_result") (param $byte i32) (result i32)
    (if (i32.eq (local.get $byte) (i32.const 45)) (then (return (i32.const 2))))
    (if (i32.eq (local.get $byte) (i32.const 126)) (then (return (i32.const 3))))
    (if (i32.eq (local.get $byte) (i32.const 63)) (then (return (i32.const 4))))
    i32.const 1)

  (func (export "mail_auth_passes") (param $spf_status i32) (param $dmarc_status i32) (result i32)
    (i32.or (i32.eq (local.get $spf_status) (i32.const 1)) (i32.eq (local.get $dmarc_status) (i32.const 1))))

  (func (export "mail_dkim_header_complete") (param $has_domain i32) (param $has_selector i32) (param $has_signature i32) (param $has_body_hash i32) (result i32)
    (i32.and (local.get $has_domain) (i32.and (local.get $has_selector) (i32.and (local.get $has_signature) (local.get $has_body_hash)))))

  (func (export "mail_dmarc_status") (param $has_policy i32) (param $spf_aligned i32) (param $dkim_aligned i32) (result i32)
    (if (i32.eqz (local.get $has_policy)) (then (return (i32.const 3))))
    (if (i32.or (local.get $spf_aligned) (local.get $dkim_aligned)) (then (return (i32.const 1))))
    i32.const 2)

  (func (export "mail_dmarc_effective_policy") (param $policy i32) (param $subdomain_policy i32) (param $is_subdomain i32) (result i32)
    (if (i32.and (local.get $is_subdomain) (i32.ne (local.get $subdomain_policy) (i32.const 0)))
      (then (return (local.get $subdomain_policy))))
    local.get $policy)

  (func (export "smtp_default_limit") (param $kind i32) (result i32)
    (if (i32.eq (local.get $kind) (i32.const 1)) (then (return (i32.const 35882577))))
    (if (i32.eq (local.get $kind) (i32.const 2)) (then (return (i32.const 100))))
    (if (i32.eq (local.get $kind) (i32.const 3)) (then (return (i32.const 998))))
    (if (i32.eq (local.get $kind) (i32.const 4)) (then (return (i32.const 300))))
    (if (i32.eq (local.get $kind) (i32.const 5)) (then (return (i32.const 1000))))
    i32.const 0)

  (func (export "lmtp_session_transition") (param $state i32) (param $cmd i32) (param $recipient_valid i32) (result i32)
    (if (i32.eq (local.get $cmd) (i32.const 17)) (then (return (call $pack (i32.const 250) (i32.const 1) (i32.const 0) (i32.const 0)))))
    (if (i32.eq (local.get $cmd) (i32.const 3))
      (then
        (if (i32.eqz (i32.or (i32.eq (local.get $state) (i32.const 1)) (i32.eq (local.get $state) (i32.const 2)))) (then (return (call $pack (i32.const 503) (local.get $state) (i32.const 0) (i32.const 0)))))
        (return (call $pack (i32.const 250) (i32.const 2) (i32.const 0) (i32.const 0)))))
    (if (i32.eq (local.get $cmd) (i32.const 4))
      (then
        (if (i32.eqz (i32.or (i32.eq (local.get $state) (i32.const 2)) (i32.eq (local.get $state) (i32.const 3)))) (then (return (call $pack (i32.const 503) (local.get $state) (i32.const 0) (i32.const 0)))))
        (if (i32.eqz (local.get $recipient_valid)) (then (return (call $pack (i32.const 550) (local.get $state) (i32.const 0) (i32.const 0)))))
        (return (call $pack (i32.const 250) (i32.const 3) (i32.const 0) (i32.const 0)))))
    (if (i32.eq (local.get $cmd) (i32.const 5))
      (then
        (if (i32.ne (local.get $state) (i32.const 3)) (then (return (call $pack (i32.const 503) (local.get $state) (i32.const 0) (i32.const 0)))))
        (return (call $pack (i32.const 354) (i32.const 4) (i32.const 0) (i32.const 0)))))
    (if (i32.eq (local.get $cmd) (i32.const 9)) (then (return (call $pack (i32.const 221) (i32.const 5) (i32.const 0) (i32.const 1)))))
    (call $pack (i32.const 554) (local.get $state) (i32.const 0) (i32.const 0)))
