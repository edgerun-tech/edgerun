#!/usr/bin/env python3

import argparse
from pathlib import Path

K = [
    0x428a2f98d728ae22, 0x7137449123ef65cd, 0xb5c0fbcfec4d3b2f, 0xe9b5dba58189dbbc,
    0x3956c25bf348b538, 0x59f111f1b605d019, 0x923f82a4af194f9b, 0xab1c5ed5da6d8118,
    0xd807aa98a3030242, 0x12835b0145706fbe, 0x243185be4ee4b28c, 0x550c7dc3d5ffb4e2,
    0x72be5d74f27b896f, 0x80deb1fe3b1696b1, 0x9bdc06a725c71235, 0xc19bf174cf692694,
    0xe49b69c19ef14ad2, 0xefbe4786384f25e3, 0x0fc19dc68b8cd5b5, 0x240ca1cc77ac9c65,
    0x2de92c6f592b0275, 0x4a7484aa6ea6e483, 0x5cb0a9dcbd41fbd4, 0x76f988da831153b5,
    0x983e5152ee66dfab, 0xa831c66d2db43210, 0xb00327c898fb213f, 0xbf597fc7beef0ee4,
    0xc6e00bf33da88fc2, 0xd5a79147930aa725, 0x06ca6351e003826f, 0x142929670a0e6e70,
    0x27b70a8546d22ffc, 0x2e1b21385c26c926, 0x4d2c6dfc5ac42aed, 0x53380d139d95b3df,
    0x650a73548baf63de, 0x766a0abb3c77b2a8, 0x81c2c92e47edaee6, 0x92722c851482353b,
    0xa2bfe8a14cf10364, 0xa81a664bbc423001, 0xc24b8b70d0f89791, 0xc76c51a30654be30,
    0xd192e819d6ef5218, 0xd69906245565a910, 0xf40e35855771202a, 0x106aa07032bbd1b8,
    0x19a4c116b8d2d0c8, 0x1e376c085141ab53, 0x2748774cdf8eeb99, 0x34b0bcb5e19b48a8,
    0x391c0cb3c5c95a63, 0x4ed8aa4ae3418acb, 0x5b9cca4f7763e373, 0x682e6ff3d6b2b8a3,
    0x748f82ee5defb2fc, 0x78a5636f43172f60, 0x84c87814a1f0ab72, 0x8cc702081a6439ec,
    0x90befffa23631e28, 0xa4506cebde82bde9, 0xbef9a3f7b2c67915, 0xc67178f2e372532b,
    0xca273eceea26619c, 0xd186b8c721c0c207, 0xeada7dd6cde0eb1e, 0xf57d4f7fee6ed178,
    0x06f067aa72176fba, 0x0a637dc5a2c898a6, 0x113f9804bef90dae, 0x1b710b35131c471b,
    0x28db77f523047d84, 0x32caab7b40c72493, 0x3c9ebe0a15c9bebc, 0x431d67c49c100d4c,
    0x4cc5d4becb3e42b6, 0x597f299cfc657e2a, 0x5fcb6fab3ad6faec, 0x6c44198c4a475817,
]

IV = {
    "sha512": [
        0x6a09e667f3bcc908, 0xbb67ae8584caa73b, 0x3c6ef372fe94f82b, 0xa54ff53a5f1d36f1,
        0x510e527fade682d1, 0x9b05688c2b3e6c1f, 0x1f83d9abfb41bd6b, 0x5be0cd19137e2179,
    ],
    "sha384": [
        0xcbbb9d5dc1059ed8, 0x629a292a367cd507, 0x9159015a3070dd17, 0x152fecd8f70e5939,
        0x67332667ffc00b31, 0x8eb44a8768581511, 0xdb0c2e0d64f98fa7, 0x47b5481dbefa4fa4,
    ],
}


def wat_bytes(values):
    data = b"".join(v.to_bytes(8, "little") for v in values)
    return "".join(f"\\{b:02x}" for b in data)


def build(kind):
    digest_len = 64 if kind == "sha512" else 48
    standard_id = 180512 if kind == "sha512" else 180384
    iv = IV[kind]
    return f''';; {kind}-fips180 fixed ABI unit
(module
  (memory (export "memory") 1)
  (data (i32.const 6144) "{wat_bytes(K)}")

  (func $read16le (param $ptr i32) (result i32)
    (i32.or (i32.load8_u (local.get $ptr))
            (i32.shl (i32.load8_u (i32.add (local.get $ptr) (i32.const 1))) (i32.const 8))))
  (func $read32le (param $ptr i32) (result i32)
    (i32.or
      (i32.or (i32.load8_u (local.get $ptr))
              (i32.shl (i32.load8_u (i32.add (local.get $ptr) (i32.const 1))) (i32.const 8)))
      (i32.or (i32.shl (i32.load8_u (i32.add (local.get $ptr) (i32.const 2))) (i32.const 16))
              (i32.shl (i32.load8_u (i32.add (local.get $ptr) (i32.const 3))) (i32.const 24)))))
  (func $read32be (param $ptr i32) (result i32)
    (i32.or
      (i32.or
        (i32.shl (i32.load8_u (local.get $ptr)) (i32.const 24))
        (i32.shl (i32.load8_u (i32.add (local.get $ptr) (i32.const 1))) (i32.const 16)))
      (i32.or
        (i32.shl (i32.load8_u (i32.add (local.get $ptr) (i32.const 2))) (i32.const 8))
        (i32.load8_u (i32.add (local.get $ptr) (i32.const 3))))))
  (func $read64be (param $ptr i32) (result i64)
    (i64.or
      (i64.shl (i64.extend_i32_u (call $read32be (local.get $ptr))) (i64.const 32))
      (i64.extend_i32_u (call $read32be (i32.add (local.get $ptr) (i32.const 4))))))
  (func $write16le (param $ptr i32) (param $value i32)
    (i32.store8 (local.get $ptr) (local.get $value))
    (i32.store8 (i32.add (local.get $ptr) (i32.const 1)) (i32.shr_u (local.get $value) (i32.const 8))))
  (func $write32le (param $ptr i32) (param $value i32)
    (i32.store8 (local.get $ptr) (local.get $value))
    (i32.store8 (i32.add (local.get $ptr) (i32.const 1)) (i32.shr_u (local.get $value) (i32.const 8)))
    (i32.store8 (i32.add (local.get $ptr) (i32.const 2)) (i32.shr_u (local.get $value) (i32.const 16)))
    (i32.store8 (i32.add (local.get $ptr) (i32.const 3)) (i32.shr_u (local.get $value) (i32.const 24))))
  (func $write64be (param $ptr i32) (param $value i64)
    (i64.store8 (local.get $ptr) (i64.shr_u (local.get $value) (i64.const 56)))
    (i64.store8 (i32.add (local.get $ptr) (i32.const 1)) (i64.shr_u (local.get $value) (i64.const 48)))
    (i64.store8 (i32.add (local.get $ptr) (i32.const 2)) (i64.shr_u (local.get $value) (i64.const 40)))
    (i64.store8 (i32.add (local.get $ptr) (i32.const 3)) (i64.shr_u (local.get $value) (i64.const 32)))
    (i64.store8 (i32.add (local.get $ptr) (i32.const 4)) (i64.shr_u (local.get $value) (i64.const 24)))
    (i64.store8 (i32.add (local.get $ptr) (i32.const 5)) (i64.shr_u (local.get $value) (i64.const 16)))
    (i64.store8 (i32.add (local.get $ptr) (i32.const 6)) (i64.shr_u (local.get $value) (i64.const 8)))
    (i64.store8 (i32.add (local.get $ptr) (i32.const 7)) (local.get $value)))
  (func $packed_result (param $ptr i32) (param $len i32) (result i64)
    (i64.or (i64.shl (i64.extend_i32_u (local.get $ptr)) (i64.const 32)) (i64.extend_i32_u (local.get $len))))
  (func $finish_error (param $code i32) (result i64)
    (call $write16le (i32.const 4096) (i32.const 12))
    (call $write16le (i32.const 4098) (i32.const 1))
    (call $write32le (i32.const 4100) (i32.const 4))
    (call $write32le (i32.const 4104) (local.get $code))
    (call $packed_result (i32.const 4096) (i32.const 12)))
  (func $finish_digest
    (param $h0 i64) (param $h1 i64) (param $h2 i64) (param $h3 i64)
    (param $h4 i64) (param $h5 i64) (param $h6 i64) (param $h7 i64)
    (result i64)
    (call $write16le (i32.const 4096) (i32.const 11))
    (call $write16le (i32.const 4098) (i32.const 0))
    (call $write32le (i32.const 4100) (i32.const {digest_len}))
    (call $write64be (i32.const 4104) (local.get $h0))
    (call $write64be (i32.const 4112) (local.get $h1))
    (call $write64be (i32.const 4120) (local.get $h2))
    (call $write64be (i32.const 4128) (local.get $h3))
    (call $write64be (i32.const 4136) (local.get $h4))
    (call $write64be (i32.const 4144) (local.get $h5))
    {'' if kind == 'sha384' else '(call $write64be (i32.const 4152) (local.get $h6))'}
    {'' if kind == 'sha384' else '(call $write64be (i32.const 4160) (local.get $h7))'}
    (call $packed_result (i32.const 4096) (i32.const {8 + digest_len})))

  (func (export "proto_abi_version") (result i32) (i32.const 2))
  (func (export "simd_capabilities") (result i32) (i32.const 1))
  (func (export "proto_standard_id") (result i32) (i32.const {standard_id}))
  (func (export "proto_open") (param $config_ptr i32) (param $config_len i32) (result i32) (i32.const 1))
  (func (export "proto_close") (param $handle i32))
  (func (export "proto_pull") (param $handle i32) (result i64) (i64.const 0))
  (func (export "proto_push")
    (param $handle i32) (param $frame_ptr i32) (param $frame_len i32)
    (result i64)
    (local $kind i32) (local $payload_len i32) (local $payload_ptr i32)
    (local $padded_len i32) (local $src i32) (local $dst i32) (local $i i32) (local $chunk i32)
    (local $h0 i64) (local $h1 i64) (local $h2 i64) (local $h3 i64)
    (local $h4 i64) (local $h5 i64) (local $h6 i64) (local $h7 i64)
    (local $a i64) (local $b i64) (local $c i64) (local $d i64)
    (local $e i64) (local $f i64) (local $g i64) (local $hh i64)
    (local $s0 i64) (local $s1 i64) (local $ch i64) (local $maj i64)
    (local $t1 i64) (local $t2 i64)

    (if (i32.ne (local.get $handle) (i32.const 1)) (then (return (call $finish_error (i32.const 3)))))
    (if (i32.lt_u (local.get $frame_len) (i32.const 8)) (then (return (call $finish_error (i32.const 1)))))
    (local.set $kind (call $read16le (local.get $frame_ptr)))
    (local.set $payload_len (call $read32le (i32.add (local.get $frame_ptr) (i32.const 4))))
    (if (i32.gt_u (local.get $payload_len) (i32.sub (local.get $frame_len) (i32.const 8))) (then (return (call $finish_error (i32.const 2)))))
    (if (i32.gt_u (local.get $payload_len) (i32.const 48000)) (then (return (call $finish_error (i32.const 5)))))
    (if (i32.and (i32.ne (local.get $kind) (i32.const 1)) (i32.ne (local.get $kind) (i32.const 3))) (then (return (call $finish_error (i32.const 4)))))

    (local.set $payload_ptr (i32.add (local.get $frame_ptr) (i32.const 8)))
    (local.set $padded_len (i32.mul (i32.div_u (i32.add (local.get $payload_len) (i32.const 144)) (i32.const 128)) (i32.const 128)))
    (local.set $src (local.get $payload_ptr))
    (local.set $dst (i32.const 8192))
    (local.set $i (i32.const 0))
    (loop $copy
      (if (i32.lt_u (local.get $i) (local.get $payload_len))
        (then
          (i32.store8 (i32.add (local.get $dst) (local.get $i)) (i32.load8_u (i32.add (local.get $src) (local.get $i))))
          (local.set $i (i32.add (local.get $i) (i32.const 1)))
          (br $copy))))
    (loop $zero
      (if (i32.lt_u (local.get $i) (local.get $padded_len))
        (then
          (i32.store8 (i32.add (local.get $dst) (local.get $i)) (i32.const 0))
          (local.set $i (i32.add (local.get $i) (i32.const 1)))
          (br $zero))))
    (i32.store8 (i32.add (local.get $dst) (local.get $payload_len)) (i32.const 128))
    (call $write64be (i32.add (local.get $dst) (i32.sub (local.get $padded_len) (i32.const 16))) (i64.const 0))
    (call $write64be (i32.add (local.get $dst) (i32.sub (local.get $padded_len) (i32.const 8))) (i64.shl (i64.extend_i32_u (local.get $payload_len)) (i64.const 3)))

    (local.set $h0 (i64.const {iv[0]})) (local.set $h1 (i64.const {iv[1]}))
    (local.set $h2 (i64.const {iv[2]})) (local.set $h3 (i64.const {iv[3]}))
    (local.set $h4 (i64.const {iv[4]})) (local.set $h5 (i64.const {iv[5]}))
    (local.set $h6 (i64.const {iv[6]})) (local.set $h7 (i64.const {iv[7]}))

    (local.set $chunk (i32.const 0))
    (loop $chunks
      (if (i32.lt_u (local.get $chunk) (local.get $padded_len))
        (then
          (local.set $i (i32.const 0))
          (loop $wfirst
            (if (i32.lt_u (local.get $i) (i32.const 16))
              (then
                (i64.store (i32.add (i32.const 4608) (i32.shl (local.get $i) (i32.const 3)))
                  (call $read64be (i32.add (i32.add (i32.const 8192) (local.get $chunk)) (i32.shl (local.get $i) (i32.const 3)))))
                (local.set $i (i32.add (local.get $i) (i32.const 1)))
                (br $wfirst))))
          (loop $wrest
            (if (i32.lt_u (local.get $i) (i32.const 80))
              (then
                (local.set $s0
                  (i64.xor
                    (i64.xor
                      (i64.rotr (i64.load (i32.add (i32.const 4608) (i32.shl (i32.sub (local.get $i) (i32.const 15)) (i32.const 3)))) (i64.const 1))
                      (i64.rotr (i64.load (i32.add (i32.const 4608) (i32.shl (i32.sub (local.get $i) (i32.const 15)) (i32.const 3)))) (i64.const 8)))
                    (i64.shr_u (i64.load (i32.add (i32.const 4608) (i32.shl (i32.sub (local.get $i) (i32.const 15)) (i32.const 3)))) (i64.const 7))))
                (local.set $s1
                  (i64.xor
                    (i64.xor
                      (i64.rotr (i64.load (i32.add (i32.const 4608) (i32.shl (i32.sub (local.get $i) (i32.const 2)) (i32.const 3)))) (i64.const 19))
                      (i64.rotr (i64.load (i32.add (i32.const 4608) (i32.shl (i32.sub (local.get $i) (i32.const 2)) (i32.const 3)))) (i64.const 61)))
                    (i64.shr_u (i64.load (i32.add (i32.const 4608) (i32.shl (i32.sub (local.get $i) (i32.const 2)) (i32.const 3)))) (i64.const 6))))
                (i64.store (i32.add (i32.const 4608) (i32.shl (local.get $i) (i32.const 3)))
                  (i64.add (i64.add (i64.add
                    (i64.load (i32.add (i32.const 4608) (i32.shl (i32.sub (local.get $i) (i32.const 16)) (i32.const 3))))
                    (local.get $s0))
                    (i64.load (i32.add (i32.const 4608) (i32.shl (i32.sub (local.get $i) (i32.const 7)) (i32.const 3)))))
                    (local.get $s1)))
                (local.set $i (i32.add (local.get $i) (i32.const 1)))
                (br $wrest))))

          (local.set $a (local.get $h0)) (local.set $b (local.get $h1))
          (local.set $c (local.get $h2)) (local.set $d (local.get $h3))
          (local.set $e (local.get $h4)) (local.set $f (local.get $h5))
          (local.set $g (local.get $h6)) (local.set $hh (local.get $h7))
          (local.set $i (i32.const 0))
          (loop $rounds
            (if (i32.lt_u (local.get $i) (i32.const 80))
              (then
                (local.set $s1 (i64.xor (i64.xor (i64.rotr (local.get $e) (i64.const 14)) (i64.rotr (local.get $e) (i64.const 18))) (i64.rotr (local.get $e) (i64.const 41))))
                (local.set $ch (i64.xor (i64.and (local.get $e) (local.get $f)) (i64.and (i64.xor (local.get $e) (i64.const -1)) (local.get $g))))
                (local.set $t1 (i64.add (i64.add (i64.add (i64.add (i64.add (local.get $hh) (local.get $s1)) (local.get $ch)) (i64.load (i32.add (i32.const 6144) (i32.shl (local.get $i) (i32.const 3))))) (i64.load (i32.add (i32.const 4608) (i32.shl (local.get $i) (i32.const 3))))) (i64.const 0)))
                (local.set $s0 (i64.xor (i64.xor (i64.rotr (local.get $a) (i64.const 28)) (i64.rotr (local.get $a) (i64.const 34))) (i64.rotr (local.get $a) (i64.const 39))))
                (local.set $maj (i64.xor (i64.xor (i64.and (local.get $a) (local.get $b)) (i64.and (local.get $a) (local.get $c))) (i64.and (local.get $b) (local.get $c))))
                (local.set $t2 (i64.add (local.get $s0) (local.get $maj)))
                (local.set $hh (local.get $g))
                (local.set $g (local.get $f))
                (local.set $f (local.get $e))
                (local.set $e (i64.add (local.get $d) (local.get $t1)))
                (local.set $d (local.get $c))
                (local.set $c (local.get $b))
                (local.set $b (local.get $a))
                (local.set $a (i64.add (local.get $t1) (local.get $t2)))
                (local.set $i (i32.add (local.get $i) (i32.const 1)))
                (br $rounds))))
          (local.set $h0 (i64.add (local.get $h0) (local.get $a)))
          (local.set $h1 (i64.add (local.get $h1) (local.get $b)))
          (local.set $h2 (i64.add (local.get $h2) (local.get $c)))
          (local.set $h3 (i64.add (local.get $h3) (local.get $d)))
          (local.set $h4 (i64.add (local.get $h4) (local.get $e)))
          (local.set $h5 (i64.add (local.get $h5) (local.get $f)))
          (local.set $h6 (i64.add (local.get $h6) (local.get $g)))
          (local.set $h7 (i64.add (local.get $h7) (local.get $hh)))
          (local.set $chunk (i32.add (local.get $chunk) (i32.const 128)))
          (br $chunks))))
    (call $finish_digest (local.get $h0) (local.get $h1) (local.get $h2) (local.get $h3) (local.get $h4) (local.get $h5) (local.get $h6) (local.get $h7)))
)
'''


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("algorithm", choices=["sha384", "sha512"])
    parser.add_argument("output")
    args = parser.parse_args()
    Path(args.output).write_text(build(args.algorithm))


if __name__ == "__main__":
    main()
