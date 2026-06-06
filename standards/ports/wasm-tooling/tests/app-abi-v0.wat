(module
  (@custom "er.manifest" "abi=edgerun.app.v0;mode=release;memory.min=65536;storage.min=4096;recursion=false;memory.static=true;storage.direct_access=false;capabilities=transition.emit,storage.intent,route.intent,tls.intent,sealed.intent,child.spawn,dependency.use,render.emit;dependencies=edgerun.ui:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef")

  (memory 1)

  ;; ABI v0 smoke: pointer/length style signatures without hostcalls.
  (func (export "er_init") (param $ctx i32) (result i32)
    local.get $ctx
    i32.const 0x494e4954
    i32.store
    local.get $ctx)

  (func (export "er_handle_message") (param $ctx i32) (param $msg_ptr i32) (param $msg_len i32) (result i32)
    local.get $ctx
    i32.const 4
    i32.add
    i32.const 0x4d534700
    i32.store
    local.get $msg_ptr
    i32.const 16
    i32.add
    local.get $msg_len
    i32.store
    i32.const 96
    i32.const 0x5452414e
    i32.store
    i32.const 100
    i32.const 1
    i32.store
    i32.const 104
    local.get $msg_len
    i32.store
    i32.const 108
    i32.const 1
    i32.store
    i32.const 112
    i32.const 1
    i32.store
    i32.const 116
    i32.const 0
    i32.store
    i32.const 120
    i32.const 64
    i32.store
    i32.const 124
    i32.const 0
    i32.store
    i32.const 128
    i32.const 0
    i32.store
    i32.const 132
    i32.const 0
    i32.store
    i32.const 136
    i32.const 1
    i32.store
    i32.const 140
    i32.const 6
    i32.store
    i32.const 144
    i32.const 0
    i32.store
    i32.const 148
    i32.const 0
    i32.store
    i32.const 152
    i32.const 0
    i32.store
    i32.const 156
    i32.const 0
    i32.store
    i32.const 160
    i32.const 0
    i32.store
    i32.const 164
    i32.const 0
    i32.store
    i32.const 168
    i32.const 0
    i32.store
    i32.const 172
    i32.const 0
    i32.store
    i32.const 176
    i32.const 1
    i32.store
    i32.const 180
    i32.const 0
    i32.store
    i32.const 184
    i32.const 0
    i32.store
    i32.const 188
    i32.const 1
    i32.store
    i32.const 192
    i32.const 2
    i32.store
    i32.const 196
    i32.const 3
    i32.store
    i32.const 200
    i32.const 0
    i32.store
    i32.const 204
    i32.const 4096
    i32.store
    i32.const 208
    i32.const 1000000
    i32.store
    i32.const 212
    i32.const 0
    i32.store
    i32.const 216
    i32.const 1
    i32.store
    i32.const 220
    i32.const 1
    i32.store
    i32.const 224
    i32.const 0
    i32.store
    i32.const 228
    i32.const 0
    i32.store
    i32.const 232
    i32.const 0
    i32.store
    i32.const 236
    i32.const 0
    i32.store
    i32.const 240
    i32.const 0
    i32.store
    i32.const 244
    i32.const 2
    i32.store
    i32.const 248
    i32.const 0
    i32.store
    i32.const 252
    i32.const 0
    i32.store
    i32.const 256
    i32.const 0
    i32.store
    i32.const 260
    i32.const 0
    i32.store
    i32.const 264
    i32.const 0
    i32.store
    i32.const 268
    i32.const 0
    i32.store
    i32.const 272
    i32.const 0
    i32.store
    i32.const 276
    i32.const 0
    i32.store
    i32.const 280
    i32.const 0
    i32.store
    i32.const 284
    i32.const 0
    i32.store
    i32.const 288
    i32.const 0
    i32.store
    i32.const 292
    i32.const 0
    i32.store
    i32.const 296
    i32.const 0
    i32.store
    i32.const 300
    i32.const 0
    i32.store
    i32.const 304
    i32.const 0
    i32.store
    i32.const 308
    i32.const 0
    i32.store
    i32.const 312
    i32.const 1
    i32.store
    i32.const 316
    i32.const 0
    i32.store
    i32.const 320
    i32.const 0
    i32.store
    i32.const 324
    i32.const 0
    i32.store
    ;; intent[0]: storage append event + durable result, 64 bytes
    i32.const 328
    i32.const 1
    i32.store
    i32.const 332
    i32.const 3
    i32.store
    i32.const 336
    i32.const 64
    i32.store
    i32.const 340
    i32.const 0
    i32.store
    i32.const 344
    i32.const 1
    i32.store
    i32.const 348
    i32.const 1
    i32.store
    i32.const 352
    i32.const 0x11111111
    i32.store
    i32.const 356
    i32.const 0x22222222
    i32.store
    ;; intent[1]: sealed store to app+user with one data requirement
    i32.const 360
    i32.const 4
    i32.store
    i32.const 364
    i32.const 1
    i32.store
    i32.const 368
    i32.const 1
    i32.store
    i32.const 372
    i32.const 6
    i32.store
    i32.const 376
    i32.const 1
    i32.store
    i32.const 380
    i32.const 0
    i32.store
    i32.const 384
    i32.const 0x33333333
    i32.store
    i32.const 388
    i32.const 0x44444444
    i32.store
    i32.const 96)

  (func (export "er_handle_action") (param $ctx i32) (param $action_ptr i32) (param $action_len i32) (result i32)
    local.get $ctx
    i32.const 8
    i32.add
    i32.const 0x41435400
    i32.store
    local.get $action_ptr
    i32.const 16
    i32.add
    local.get $action_len
    i32.store
    i32.const 352
    i32.const 0x5452414e
    i32.store
    i32.const 356
    i32.const 2
    i32.store
    i32.const 360
    local.get $action_len
    i32.store
    i32.const 364
    i32.const 1
    i32.store
    i32.const 368
    i32.const 1
    i32.store
    i32.const 372
    i32.const 0
    i32.store
    i32.const 376
    i32.const 32
    i32.store
    i32.const 380
    i32.const 1
    i32.store
    i32.const 384
    i32.const 0
    i32.store
    i32.const 388
    i32.const 24
    i32.store
    i32.const 392
    i32.const 0
    i32.store
    i32.const 396
    i32.const 0
    i32.store
    i32.const 400
    i32.const 0
    i32.store
    i32.const 404
    i32.const 1
    i32.store
    i32.const 408
    i32.const 16384
    i32.store
    i32.const 412
    i32.const 2048
    i32.store
    i32.const 416
    i32.const 0
    i32.store
    i32.const 420
    i32.const 1
    i32.store
    i32.const 424
    i32.const 0
    i32.store
    i32.const 428
    i32.const 0
    i32.store
    i32.const 432
    i32.const 1
    i32.store
    i32.const 436
    i32.const 0
    i32.store
    i32.const 440
    i32.const 0
    i32.store
    i32.const 444
    i32.const 1
    i32.store
    i32.const 448
    i32.const 4
    i32.store
    i32.const 452
    i32.const 15
    i32.store
    i32.const 456
    i32.const 0
    i32.store
    i32.const 460
    i32.const 8192
    i32.store
    i32.const 464
    i32.const 1000000
    i32.store
    i32.const 468
    i32.const 0
    i32.store
    i32.const 472
    i32.const 1
    i32.store
    i32.const 476
    i32.const 1
    i32.store
    i32.const 480
    i32.const 0
    i32.store
    i32.const 484
    i32.const 1
    i32.store
    i32.const 488
    i32.const 1
    i32.store
    i32.const 492
    i32.const 1
    i32.store
    i32.const 496
    i32.const 0
    i32.store
    i32.const 500
    i32.const 4
    i32.store
    i32.const 504
    i32.const 0
    i32.store
    i32.const 508
    i32.const 0
    i32.store
    i32.const 512
    i32.const 0
    i32.store
    i32.const 516
    i32.const 0
    i32.store
    i32.const 520
    i32.const 0
    i32.store
    i32.const 524
    i32.const 1
    i32.store
    i32.const 528
    i32.const 0
    i32.store
    i32.const 532
    i32.const 0
    i32.store
    i32.const 536
    i32.const 1
    i32.store
    i32.const 540
    i32.const 1
    i32.store
    i32.const 544
    i32.const 0
    i32.store
    i32.const 548
    i32.const 0
    i32.store
    i32.const 552
    i32.const 0
    i32.store
    i32.const 556
    i32.const 0
    i32.store
    i32.const 560
    i32.const 0
    i32.store
    i32.const 564
    i32.const 0
    i32.store
    i32.const 568
    i32.const 0
    i32.store
    i32.const 572
    i32.const 0
    i32.store
    i32.const 576
    i32.const 0
    i32.store
    i32.const 580
    i32.const 0
    i32.store
    ;; intent[0]: storage append event + durable result, 32 bytes
    i32.const 584
    i32.const 1
    i32.store
    i32.const 588
    i32.const 3
    i32.store
    i32.const 592
    i32.const 32
    i32.store
    i32.const 596
    i32.const 0
    i32.store
    i32.const 600
    i32.const 1
    i32.store
    i32.const 604
    i32.const 1
    i32.store
    i32.const 608
    i32.const 0x55555555
    i32.store
    i32.const 612
    i32.const 0x66666666
    i32.store
    ;; intent[1]: send sealed message to identity route, 24 bytes
    i32.const 616
    i32.const 2
    i32.store
    i32.const 620
    i32.const 1
    i32.store
    i32.const 624
    i32.const 24
    i32.store
    i32.const 628
    i32.const 0
    i32.store
    i32.const 632
    i32.const 1
    i32.store
    i32.const 636
    i32.const 1
    i32.store
    i32.const 640
    i32.const 0x77777777
    i32.store
    i32.const 644
    i32.const 0x88888888
    i32.store
    ;; intent[2]: create hidden service by identity, not raw port
    i32.const 648
    i32.const 3
    i32.store
    i32.const 652
    i32.const 1
    i32.store
    i32.const 656
    i32.const 0
    i32.store
    i32.const 660
    i32.const 0
    i32.store
    i32.const 664
    i32.const 0
    i32.store
    i32.const 668
    i32.const 0
    i32.store
    i32.const 672
    i32.const 0xbbbbbbbb
    i32.store
    i32.const 676
    i32.const 0xcccccccc
    i32.store
    ;; intent[3]: child spawn moves memory/storage and names child app identity
    i32.const 680
    i32.const 5
    i32.store
    i32.const 684
    i32.const 0
    i32.store
    i32.const 688
    i32.const 16384
    i32.store
    i32.const 692
    i32.const 0
    i32.store
    i32.const 696
    i32.const 2048
    i32.store
    i32.const 700
    i32.const 0
    i32.store
    i32.const 704
    i32.const 0x99999999
    i32.store
    i32.const 708
    i32.const 0xaaaaaaaa
    i32.store
    i32.const 352)

  (func (export "er_render") (param $ctx i32) (result i32)
    i32.const 896
    i32.const 0x55490000
    i32.store
    i32.const 900
    i32.const 1
    i32.store
    i32.const 904
    i32.const 0
    i32.store
    i32.const 908
    i32.const 1
    i32.store
    i32.const 912
    i32.const 0
    i32.store
    i32.const 916
    i32.const 0
    i32.store
    i32.const 920
    i32.const 1
    i32.store
    i32.const 924
    i32.const 0
    i32.store
    i32.const 928
    i32.const 0
    i32.store
    i32.const 932
    i32.const 0
    i32.store
    i32.const 896)
)
