(module
  (memory (export "memory") 1)

  (data (i32.const 32768) "android.permission.")
  (data (i32.const 32832) "uses-permission")
  (data (i32.const 32896) "package=")
  (data (i32.const 32960) "import android.")
  (data (i32.const 33024) "import java.")
  (data (i32.const 33088) "import javax.")
  (data (i32.const 33152) "landroid/")
  (data (i32.const 33216) "ljava/")
  (data (i32.const 33280) "ljavax/")
  (data (i32.const 33344) "lorg/apache/http/")
  (data (i32.const 33408) "lokhttp3/")
  (data (i32.const 33472) "lretrofit2/")
  (data (i32.const 33536) "okhttpclient")
  (data (i32.const 33600) "request.builder")
  (data (i32.const 33664) ".url(")
  (data (i32.const 33728) ".baseurl(")
  (data (i32.const 33792) "httpurlconnection")
  (data (i32.const 33856) "websocket")
  (data (i32.const 33920) "uri.parse(")
  (data (i32.const 33984) "landroid/net/uri;->parse")
  (data (i32.const 34048) "landroid/webkit/webview;")
  (data (i32.const 34112) "http://")
  (data (i32.const 34176) "https://")
  (data (i32.const 34240) "ws://")
  (data (i32.const 34304) "wss://")
  (data (i32.const 34368) "content://")
  (data (i32.const 34432) "intent://")
  (data (i32.const 34496) "ljavax/crypto/")
  (data (i32.const 34560) "javax.crypto.")
  (data (i32.const 34624) "ljava/security/")
  (data (i32.const 34688) "java.security.")
  (data (i32.const 34752) "keystore")
  (data (i32.const 34816) "cipher")
  (data (i32.const 34880) "signature")
  (data (i32.const 34944) "landroid/location/")
  (data (i32.const 35008) "android.location.")
  (data (i32.const 35072) "location")
  (data (i32.const 35136) "landroid/hardware/camera")
  (data (i32.const 35200) "android.hardware.camera")
  (data (i32.const 35264) "camera")
  (data (i32.const 35328) "landroid/bluetooth/")
  (data (i32.const 35392) "android.bluetooth.")
  (data (i32.const 35456) "bluetooth")
  (data (i32.const 35520) "landroid/telephony/smsmanager;")
  (data (i32.const 35584) "android.telephony.")
  (data (i32.const 35648) "sms")
  (data (i32.const 35712) "landroid/provider/contactscontract;")
  (data (i32.const 35776) "landroid/provider/calendarcontract;")
  (data (i32.const 35840) "contacts")
  (data (i32.const 35904) "calendar")
  (data (i32.const 35968) "ldalvik/system/dexclassloader;")
  (data (i32.const 36032) "ldalvik/system/pathclassloader;")
  (data (i32.const 36096) "ljava/lang/runtime;->exec")
  (data (i32.const 36160) "ljava/lang/processbuilder;")
  (data (i32.const 36224) "addjavascriptinterface")
  (data (i32.const 36288) "graphql")
  (data (i32.const 36352) "analytics")
  (data (i32.const 36416) "token")
  (data (i32.const 36480) "auth")
  (data (i32.const 36544) "login")

  (func (export "proto_abi_version") (result i32)
    i32.const 2)

  (func (export "proto_standard_id") (result i32)
    i32.const 300091)

  ;; Status: 0 ok, 2 no finding, 3 unsupported/invalid.
  ;; Finding record, 20 bytes:
  ;;   u32 kind, u32 start_byte, u32 end_byte, u32 flags, u32 status.
  ;; Kinds: 1 package, 2 permission, 3 platform_api, 4 import_api, 5 uri,
  ;;   6 network_web, 7 crypto_security, 8 location, 9 camera_media,
  ;;   10 bluetooth_nearby, 11 sms_telephony, 12 contacts_calendar,
  ;;   13 dynamic_process, 14 javascript_bridge, 15 endpoint_string.
  ;; Flags: bit0 matched inside a quoted string, bit1 Android/platform API,
  ;;   bit2 network/URI, bit3 sensitive capability, bit4 manifest-ish.

  (func $ch (param $ptr i32) (param $pos i32) (result i32)
    local.get $ptr
    local.get $pos
    i32.add
    i32.load8_u)

  (func $lower (param $c i32) (result i32)
    local.get $c
    i32.const 65
    i32.ge_u
    local.get $c
    i32.const 90
    i32.le_u
    i32.and
    if (result i32)
      local.get $c
      i32.const 32
      i32.add
    else
      local.get $c
    end)

  (func $pack (param $kind i32) (param $plen i32) (result i64)
    local.get $kind
    i64.extend_i32_u
    i64.const 32
    i64.shl
    local.get $plen
    i64.extend_i32_u
    i64.or)

  (func $packed_kind (param $v i64) (result i32)
    local.get $v
    i64.const 32
    i64.shr_u
    i32.wrap_i64)

  (func $packed_len (param $v i64) (result i32)
    local.get $v
    i32.wrap_i64)

  (func $match_lit (param $ptr i32) (param $len i32) (param $pos i32) (param $pat i32) (param $plen i32) (result i32)
    (local $j i32)
    local.get $pos
    local.get $plen
    i32.add
    local.get $len
    i32.gt_u
    if
      i32.const 0
      return
    end
    i32.const 0
    local.set $j
    block $no
      loop $scan
        local.get $j
        local.get $plen
        i32.ge_u
        if
          i32.const 1
          return
        end
        local.get $ptr
        local.get $pos
        local.get $j
        i32.add
        call $ch
        call $lower
        local.get $pat
        local.get $j
        i32.add
        i32.load8_u
        i32.ne
        br_if $no
        local.get $j
        i32.const 1
        i32.add
        local.set $j
        br $scan
      end
    end
    i32.const 0)

  (func $classify_string (param $ptr i32) (param $len i32) (param $pos i32) (result i64)
    local.get $ptr local.get $len local.get $pos i32.const 32768 i32.const 19 call $match_lit
    if i32.const 2 i32.const 19 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 32832 i32.const 15 call $match_lit
    if i32.const 2 i32.const 15 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 34112 i32.const 7 call $match_lit
    if i32.const 5 i32.const 7 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 34176 i32.const 8 call $match_lit
    if i32.const 5 i32.const 8 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 34240 i32.const 5 call $match_lit
    if i32.const 5 i32.const 5 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 34304 i32.const 6 call $match_lit
    if i32.const 5 i32.const 6 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 34368 i32.const 10 call $match_lit
    if i32.const 5 i32.const 10 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 34432 i32.const 9 call $match_lit
    if i32.const 5 i32.const 9 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 36288 i32.const 7 call $match_lit
    if i32.const 15 i32.const 7 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 36352 i32.const 9 call $match_lit
    if i32.const 15 i32.const 9 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 36416 i32.const 5 call $match_lit
    if i32.const 15 i32.const 5 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 36480 i32.const 4 call $match_lit
    if i32.const 15 i32.const 4 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 36544 i32.const 5 call $match_lit
    if i32.const 15 i32.const 5 call $pack return end
    i64.const 0)

  (func $classify_code (param $ptr i32) (param $len i32) (param $pos i32) (result i64)
    local.get $ptr local.get $len local.get $pos i32.const 32896 i32.const 8 call $match_lit
    if i32.const 1 i32.const 8 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 32768 i32.const 19 call $match_lit
    if i32.const 2 i32.const 19 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 32832 i32.const 15 call $match_lit
    if i32.const 2 i32.const 15 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 32960 i32.const 15 call $match_lit
    if i32.const 4 i32.const 15 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 33024 i32.const 12 call $match_lit
    if i32.const 4 i32.const 12 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 33088 i32.const 13 call $match_lit
    if i32.const 4 i32.const 13 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 33408 i32.const 9 call $match_lit
    if i32.const 6 i32.const 9 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 33472 i32.const 11 call $match_lit
    if i32.const 6 i32.const 11 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 33536 i32.const 12 call $match_lit
    if i32.const 6 i32.const 12 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 33600 i32.const 15 call $match_lit
    if i32.const 6 i32.const 15 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 33664 i32.const 5 call $match_lit
    if i32.const 6 i32.const 5 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 33728 i32.const 9 call $match_lit
    if i32.const 6 i32.const 9 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 33792 i32.const 17 call $match_lit
    if i32.const 6 i32.const 17 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 33856 i32.const 9 call $match_lit
    if i32.const 6 i32.const 9 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 33920 i32.const 10 call $match_lit
    if i32.const 6 i32.const 10 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 33984 i32.const 24 call $match_lit
    if i32.const 6 i32.const 24 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 34048 i32.const 23 call $match_lit
    if i32.const 6 i32.const 23 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 34112 i32.const 7 call $match_lit
    if i32.const 5 i32.const 7 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 34176 i32.const 8 call $match_lit
    if i32.const 5 i32.const 8 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 34240 i32.const 5 call $match_lit
    if i32.const 5 i32.const 5 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 34304 i32.const 6 call $match_lit
    if i32.const 5 i32.const 6 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 34368 i32.const 10 call $match_lit
    if i32.const 5 i32.const 10 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 34432 i32.const 9 call $match_lit
    if i32.const 5 i32.const 9 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 34496 i32.const 14 call $match_lit
    if i32.const 7 i32.const 14 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 34560 i32.const 13 call $match_lit
    if i32.const 7 i32.const 13 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 34624 i32.const 15 call $match_lit
    if i32.const 7 i32.const 15 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 34688 i32.const 14 call $match_lit
    if i32.const 7 i32.const 14 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 34752 i32.const 8 call $match_lit
    if i32.const 7 i32.const 8 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 34816 i32.const 6 call $match_lit
    if i32.const 7 i32.const 6 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 34880 i32.const 9 call $match_lit
    if i32.const 7 i32.const 9 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 34944 i32.const 18 call $match_lit
    if i32.const 8 i32.const 18 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 35008 i32.const 17 call $match_lit
    if i32.const 8 i32.const 17 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 35072 i32.const 8 call $match_lit
    if i32.const 8 i32.const 8 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 35136 i32.const 24 call $match_lit
    if i32.const 9 i32.const 24 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 35200 i32.const 23 call $match_lit
    if i32.const 9 i32.const 23 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 35264 i32.const 6 call $match_lit
    if i32.const 9 i32.const 6 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 35328 i32.const 19 call $match_lit
    if i32.const 10 i32.const 19 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 35392 i32.const 18 call $match_lit
    if i32.const 10 i32.const 18 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 35456 i32.const 9 call $match_lit
    if i32.const 10 i32.const 9 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 35520 i32.const 30 call $match_lit
    if i32.const 11 i32.const 30 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 35584 i32.const 18 call $match_lit
    if i32.const 11 i32.const 18 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 35648 i32.const 3 call $match_lit
    if i32.const 11 i32.const 3 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 35712 i32.const 35 call $match_lit
    if i32.const 12 i32.const 35 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 35776 i32.const 35 call $match_lit
    if i32.const 12 i32.const 35 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 35840 i32.const 8 call $match_lit
    if i32.const 12 i32.const 8 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 35904 i32.const 8 call $match_lit
    if i32.const 12 i32.const 8 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 35968 i32.const 29 call $match_lit
    if i32.const 13 i32.const 29 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 36032 i32.const 30 call $match_lit
    if i32.const 13 i32.const 30 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 36096 i32.const 26 call $match_lit
    if i32.const 13 i32.const 26 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 36160 i32.const 26 call $match_lit
    if i32.const 13 i32.const 26 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 36224 i32.const 22 call $match_lit
    if i32.const 14 i32.const 22 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 33152 i32.const 9 call $match_lit
    if i32.const 3 i32.const 9 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 33216 i32.const 6 call $match_lit
    if i32.const 3 i32.const 6 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 33280 i32.const 7 call $match_lit
    if i32.const 3 i32.const 7 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 33344 i32.const 16 call $match_lit
    if i32.const 3 i32.const 16 call $pack return end
    i64.const 0)

  (func $flags_for (param $kind i32) (param $in_string i32) (result i32)
    (local $flags i32)
    local.get $in_string
    local.set $flags
    local.get $kind i32.const 3 i32.eq
    local.get $kind i32.const 4 i32.eq i32.or
    if
      local.get $flags i32.const 2 i32.or local.set $flags
    end
    local.get $kind i32.const 5 i32.eq
    local.get $kind i32.const 6 i32.eq i32.or
    if
      local.get $flags i32.const 4 i32.or local.set $flags
    end
    local.get $kind i32.const 7 i32.eq
    local.get $kind i32.const 8 i32.eq i32.or
    local.get $kind i32.const 9 i32.eq i32.or
    local.get $kind i32.const 10 i32.eq i32.or
    local.get $kind i32.const 11 i32.eq i32.or
    local.get $kind i32.const 12 i32.eq i32.or
    local.get $kind i32.const 13 i32.eq i32.or
    local.get $kind i32.const 14 i32.eq i32.or
    if
      local.get $flags i32.const 8 i32.or local.set $flags
    end
    local.get $kind i32.const 1 i32.eq
    local.get $kind i32.const 2 i32.eq i32.or
    if
      local.get $flags i32.const 16 i32.or local.set $flags
    end
    local.get $flags)

  (func $write_out (param $out i32) (param $kind i32) (param $start i32) (param $end i32) (param $flags i32) (param $status i32)
    local.get $out local.get $kind i32.store align=1
    local.get $out i32.const 4 i32.add local.get $start i32.store align=1
    local.get $out i32.const 8 i32.add local.get $end i32.store align=1
    local.get $out i32.const 12 i32.add local.get $flags i32.store align=1
    local.get $out i32.const 16 i32.add local.get $status i32.store align=1)

  (func $state_until (param $ptr i32) (param $len i32) (param $limit i32) (result i64)
    (local $i i32) (local $c i32) (local $next i32) (local $state i32) (local $quote i32)
    i32.const 0 local.set $i
    i32.const 0 local.set $state
    i32.const 0 local.set $quote
    block $done
      loop $scan
        local.get $i local.get $limit i32.ge_u br_if $done
        local.get $i local.get $len i32.ge_u br_if $done
        local.get $ptr local.get $i call $ch local.set $c
        local.get $state i32.eqz
        if
          local.get $i i32.const 1 i32.add local.get $limit i32.lt_u
          local.get $i i32.const 1 i32.add local.get $len i32.lt_u i32.and
          if
            local.get $ptr local.get $i i32.const 1 i32.add call $ch local.set $next
            local.get $c i32.const 47 i32.eq
            local.get $next i32.const 47 i32.eq i32.and
            if
              i32.const 1 local.set $state
              local.get $i i32.const 2 i32.add local.set $i
              br $scan
            end
            local.get $c i32.const 47 i32.eq
            local.get $next i32.const 42 i32.eq i32.and
            if
              i32.const 2 local.set $state
              local.get $i i32.const 2 i32.add local.set $i
              br $scan
            end
          end
          local.get $c i32.const 34 i32.eq
          local.get $c i32.const 39 i32.eq i32.or
          local.get $c i32.const 96 i32.eq i32.or
          if
            i32.const 3 local.set $state
            local.get $c local.set $quote
            local.get $i i32.const 1 i32.add local.set $i
            br $scan
          end
        else
          local.get $state i32.const 1 i32.eq
          if
            local.get $c i32.const 10 i32.eq
            local.get $c i32.const 13 i32.eq i32.or
            if i32.const 0 local.set $state end
            local.get $i i32.const 1 i32.add local.set $i
            br $scan
          end
          local.get $state i32.const 2 i32.eq
          if
            local.get $i i32.const 1 i32.add local.get $limit i32.lt_u
            local.get $i i32.const 1 i32.add local.get $len i32.lt_u i32.and
            if
              local.get $c i32.const 42 i32.eq
              local.get $ptr local.get $i i32.const 1 i32.add call $ch i32.const 47 i32.eq
              i32.and
              if
                i32.const 0 local.set $state
                local.get $i i32.const 2 i32.add local.set $i
                br $scan
              end
            end
            local.get $i i32.const 1 i32.add local.set $i
            br $scan
          end
          local.get $state i32.const 3 i32.eq
          if
            local.get $c i32.const 92 i32.eq
            if
              local.get $i i32.const 2 i32.add local.set $i
              br $scan
            end
            local.get $c local.get $quote i32.eq
            if
              i32.const 0 local.set $state
              i32.const 0 local.set $quote
            end
          end
        end
        local.get $i i32.const 1 i32.add local.set $i
        br $scan
      end
    end
    local.get $state i64.extend_i32_u i64.const 32 i64.shl
    local.get $quote i64.extend_i32_u i64.or)

  (func $apk_api_scan_next (export "apk_api_scan_next") (param $ptr i32) (param $len i32) (param $start i32) (param $out i32) (result i32)
    (local $i i32) (local $c i32) (local $next i32) (local $state i32) (local $quote i32)
    (local $hit i64) (local $kind i32) (local $plen i32) (local $in_string i32) (local $packed_state i64)
    local.get $ptr local.get $len i32.add i32.const 65500 i32.gt_u
    local.get $out i32.const 20 i32.add i32.const 65500 i32.gt_u i32.or
    local.get $start local.get $len i32.gt_u i32.or
    if
      local.get $out i32.const 0 i32.const 0 i32.const 0 i32.const 0 i32.const 3 call $write_out
      i32.const 3
      return
    end
    local.get $ptr local.get $len local.get $start call $state_until local.set $packed_state
    local.get $packed_state i64.const 32 i64.shr_u i32.wrap_i64 local.set $state
    local.get $packed_state i32.wrap_i64 local.set $quote
    local.get $start local.set $i
    block $done
      loop $scan
        local.get $i
        local.get $len
        i32.ge_u
        br_if $done
        local.get $ptr local.get $i call $ch
        local.set $c
        local.get $state
        i32.eqz
        if
          local.get $i i32.const 1 i32.add local.set $next
          local.get $i i32.const 1 i32.add local.get $len i32.lt_u
          if
            local.get $ptr local.get $i i32.const 1 i32.add call $ch
            local.set $next
            local.get $c i32.const 47 i32.eq
            local.get $next i32.const 47 i32.eq i32.and
            if
              i32.const 1 local.set $state
              local.get $i i32.const 2 i32.add local.set $i
              br $scan
            end
            local.get $c i32.const 47 i32.eq
            local.get $next i32.const 42 i32.eq i32.and
            if
              i32.const 2 local.set $state
              local.get $i i32.const 2 i32.add local.set $i
              br $scan
            end
          end
          local.get $c i32.const 34 i32.eq
          local.get $c i32.const 39 i32.eq i32.or
          local.get $c i32.const 96 i32.eq i32.or
          if
            i32.const 3 local.set $state
            local.get $c local.set $quote
            local.get $i i32.const 1 i32.add local.set $i
            br $scan
          end
          local.get $ptr local.get $len local.get $i call $classify_code local.set $hit
          i32.const 0 local.set $in_string
        else
          local.get $state i32.const 1 i32.eq
          if
            local.get $c i32.const 10 i32.eq
            local.get $c i32.const 13 i32.eq i32.or
            if i32.const 0 local.set $state end
            local.get $i i32.const 1 i32.add local.set $i
            br $scan
          end
          local.get $state i32.const 2 i32.eq
          if
            local.get $i i32.const 1 i32.add local.get $len i32.lt_u
            if
              local.get $c i32.const 42 i32.eq
              local.get $ptr local.get $i i32.const 1 i32.add call $ch i32.const 47 i32.eq
              i32.and
              if
                i32.const 0 local.set $state
                local.get $i i32.const 2 i32.add local.set $i
                br $scan
              end
            end
            local.get $i i32.const 1 i32.add local.set $i
            br $scan
          end
          local.get $state i32.const 3 i32.eq
          if
            local.get $c i32.const 92 i32.eq
            if
              local.get $i i32.const 2 i32.add local.set $i
              br $scan
            end
            local.get $c local.get $quote i32.eq
            if
              i32.const 0 local.set $state
              local.get $i i32.const 1 i32.add local.set $i
              br $scan
            end
            local.get $ptr local.get $len local.get $i call $classify_string local.set $hit
            i32.const 1 local.set $in_string
          end
        end
        local.get $hit i64.eqz
        i32.eqz
        if
          local.get $hit call $packed_kind local.set $kind
          local.get $hit call $packed_len local.set $plen
          local.get $out
          local.get $kind
          local.get $i
          local.get $i local.get $plen i32.add
          local.get $kind local.get $in_string call $flags_for
          i32.const 0
          call $write_out
          i32.const 0
          return
        end
        local.get $i i32.const 1 i32.add local.set $i
        br $scan
      end
    end
    local.get $out i32.const 0 i32.const 0 i32.const 0 i32.const 0 i32.const 2 call $write_out
    i32.const 2)

  (func (export "apk_api_scan_count") (param $ptr i32) (param $len i32) (result i64)
    (local $pos i32) (local $count i32) (local $status i32) (local $out i32) (local $end i32)
    i32.const 65472
    local.set $out
    local.get $ptr local.get $len i32.add i32.const 65500 i32.gt_u
    if i64.const 3 return end
    i32.const 0 local.set $pos
    i32.const 0 local.set $count
    block $done
      loop $loop
        local.get $ptr local.get $len local.get $pos local.get $out call $apk_api_scan_next
        local.tee $status
        i32.const 2
        i32.eq
        br_if $done
        local.get $status
        if
          local.get $status i64.extend_i32_u
          return
        end
        local.get $count i32.const 1 i32.add local.set $count
        local.get $out i32.const 8 i32.add i32.load align=1 local.set $end
        local.get $end local.get $pos i32.le_u
        if
          local.get $pos i32.const 1 i32.add local.set $pos
        else
          local.get $end local.set $pos
        end
        local.get $pos local.get $len i32.ge_u br_if $done
        br $loop
      end
    end
    local.get $count
    i64.extend_i32_u
    i64.const 32
    i64.shl)
)
