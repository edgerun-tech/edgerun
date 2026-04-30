---
title: Benchmark อีเมลของ Edgerun: สถานะปัจจุบันของ server
date: 2026-04-30
author: Ken
summary: การเปรียบเทียบแบบ rootless Podman ระหว่าง Edgerun, Maddy และ Postfix หลังจากปรับ runtime, SMTP/IMAP และ Maildir index.
tags: [email, benchmarks, release]
---
# Benchmark อีเมลของ Edgerun: สถานะปัจจุบันของ server

ตอนนี้ Edgerun รัน mail stack สำหรับ `edgerun.tech` ใน production แล้ว: SMTP,
SMTPS, submission, IMAP, IMAPS, DNS, HTTPS, webmail, DKIM signing, queue
handling, blog นี้ และ Git host ทั้งหมดอยู่ใน binary `edgerun-server` ตัวเดียว
ที่ strip แล้ว.

Product claim คือ server ขนาดเล็กแบบ integrated สามารถแทนหลาย daemon ได้ใน
deployment ขนาดเล็ก. Benchmark claim ต้องแคบกว่า: หลังจากปรับ runtime,
SMTP/IMAP buffering และ Maildir index แล้ว Edgerun เร็วกว่า Maddy และ Postfix
ที่วัดได้ใน local delivery workload นี้.

## สิ่งที่เปลี่ยน

ใน code ตอนนี้มี host multi-thread runtime, buffered SMTP/IMAP I/O,
local delivery แบบ recipient batch, Maildir status sidecar สำหรับ IMAP
`SELECT`, และ SMTP delivery status-index updates ที่ flush ประมาณทุก 500 ms.
per-message Maildir `sync_all()` กลายเป็น strict mode ผ่าน
`EDGERUN_MAILDIR_SYNC_DELIVERY=1`.

## ผลลัพธ์

Evidence ล่าสุดคือ rootless Podman run จำนวน 5 repetitions:

`docs/benchmarks/email-stack/20260430Tedgerun-postfix-maddy-batched-r5/`

ข้อจำกัดสำคัญ: ทั้ง 5 repetitions ใช้ container เดิมของแต่ละ stack ดังนั้น
mailbox/spool state โตขึ้นระหว่างรอบ. นี่คือ sustained growing-state result
ไม่ใช่ cold start อิสระ 5 ครั้ง.

Median throughput จาก 5 repetitions:

| Stack | SMTP median | IMAP median |
| --- | ---: | ---: |
| Edgerun | 527.03 ops/s | 216.92 ops/s |
| Maddy | 255.45 ops/s | 81.10 ops/s |
| Postfix | 126.10 ops/s | n/a |

รอบแรกก่อน state สะสมมาก:

| Stack | SMTP throughput | SMTP p95 | Peak memory / PIDs |
| --- | ---: | ---: | --- |
| Edgerun | 557.72 ops/s | 64.173 ms | 113.20 MB / 10 |
| Maddy | 344.62 ops/s | 123.828 ms | 148.70 MB / 41 |
| Postfix | 347.70 ops/s | 110.324 ms | 134.20 MB / 75 |

## Footprint

| Stack | Image size | Build time | Startup time | Process count |
| --- | ---: | ---: | ---: | ---: |
| Edgerun | 4,812,313 bytes | 797 ms | 2,199 ms | 10 |
| Maddy | 58,114,573 bytes | 1,310 ms | 2,230 ms | 41-45 |
| Postfix | 129,522,342 bytes | 14,198 ms | 2,186 ms | 75-76 |

Memory claim ยังต้องระวัง. Peak memory ของ Edgerun ใน SMTP run โตจาก
113.20 MB เป็น 202.40 MB ตลอด 5 repetitions. Maddy โตจาก 148.70 MB เป็น
249.20 MB และ Postfix โตจาก 134.20 MB เป็น 209.00 MB. Evidence ตอนนี้รองรับ
claim เรื่อง image/process footprint ได้ชัด แต่ยังไม่ควร claim ว่า memory
ต่ำสุดในทุกสภาพ.

## สิ่งที่สรุปได้

Claim ที่ publish ได้อย่างแคบ:

> Edgerun เป็น compact integrated mail stack ที่ควรพิจารณาสำหรับ operator ที่ต้องการ SMTP, IMAP, web, DNS, DKIM, queue handling และ site hosting ใน service ขนาดเล็กตัวเดียว. ใน rootless Podman local-delivery benchmark ล่าสุด Edgerun มี SMTP throughput สูงกว่า Maddy และ Postfix, basic IMAP เร็วกว่า Maddy, และ image/process footprint เล็กกว่ามาก.

ยังไม่ควร claim ว่า Edgerun ชนะทุก mail-server workload, realistic Dovecot
IMAP, Mox หรือ Stalwart ใน production configuration. Exim ถูกถอดจาก active
comparison เพราะเรายังไม่มี tuned harness result ที่ยุติธรรม.

Raw evidence:

- `docs/benchmarks/email-stack/20260430Tedgerun-postfix-maddy-batched-r5/`
- `docs/benchmarks/email-stack/20260430Tedgerun-postfix-maddy-batched/`
- `docs/benchmarks/email-stack/20260430Tactive-smoke/`
- `docs/benchmarks/email-stack/research-status-20260430.md`
