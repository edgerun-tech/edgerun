---
title: Benchmark อีเมลของ Edgerun: SMTP ภายใน 10,000 ครั้ง
date: 2026-04-30
author: Ken
summary: การวัดแบบทำซ้ำได้ครั้งแรกของเซิร์ฟเวอร์อีเมลแบบ all-in-one ของ Edgerun เทียบกับ Postfix, OpenSMTPD และ Exim.
tags: [email, benchmarks, release]
---
# Benchmark อีเมลของ Edgerun: SMTP ภายใน 10,000 ครั้ง

ตอนนี้ Edgerun รัน mail stack สำหรับ `edgerun.tech` ใน production แล้ว: SMTP, SMTPS, submission, IMAP, IMAPS, DNS, HTTPS, webmail, DKIM signing, queue handling, blog นี้ และ Git host ทั้งหมดอยู่ใน binary `edgerun-server` ตัวเดียวที่ strip แล้ว.

Benchmark นี้ตั้งใจวัดเรื่องแคบ ๆ ก่อน: ส่ง SMTP ภายในเครื่อง 10,000 ข้อความ, concurrency 32, body 1 KiB, หนึ่ง connection ต่อหนึ่งข้อความ. ทุกอย่างอยู่บน localhost และ recipient เป็น local เท่านั้น ไม่มีการส่งอีเมลออกอินเทอร์เน็ต.

## ผลลัพธ์

| Stack | OK/Fail | Throughput | p95 latency | Memory สูงสุดตอน load |
| --- | ---: | ---: | ---: | ---: |
| Edgerun | 10000/0 | 342.71 ops/s | 140.577 ms | 3.58 MB RSS |
| Postfix | 10000/0 | 351.69 ops/s | 141.674 ms | 94.02 MB container memory |
| OpenSMTPD | 9976/24 | 176.01 ops/s | 227.632 ms | 17.91 MB container memory |
| Exim | 2146/7854 | 27.41 ops/s | 1663.886 ms | 219.9 MB container memory |

Postfix คือ comparison ที่ใกล้ที่สุดในรอบนี้. สำหรับ workload แบบ local SMTP accept, Edgerun อยู่ใกล้ Postfix มาก แต่ใช้ memory น้อยกว่ามาก. OpenSMTPD เป็น MTA ที่เล็กและ config อ่านง่าย แต่รอบนี้มี operation fail 24 ครั้ง และ tail latency สูงกว่า.

ผลของ Exim ยังไม่ควรถูกอ่านว่าเป็นข้อสรุปเรื่องความสามารถของ Exim. มันเป็น finding เรื่อง configuration มากกว่า. config แบบ minimal rootless container ของเรารับได้แค่ 2,146 จาก 10,000 attempts และ memory/process count โตสูงมาก จึงต้อง tune เพิ่มก่อนจะเทียบอย่างยุติธรรม.

## สิ่งที่สรุปได้

สรุปแบบแคบได้ว่า local SMTP delivery path ของ Edgerun แข่งขันกับ Postfix ได้แล้วใน warmed 10k localhost workload และใช้ memory น้อยกว่ามาก.

แต่ยังไม่ใช่ข้อสรุปว่า Edgerun ชนะทุก mail-server workload. งานถัดไปควรวัด Stalwart ด้วย first-run config ที่ทำซ้ำได้, IMAP mailbox operations, webmail inbox/send, outbound STARTTLS delivery, concurrent idle connections และพฤติกรรมกับ client ที่ malformed หรือ slow.

Raw evidence ถูก commit อยู่ใน source tree:

- `docs/benchmarks/email-stack/20260430Tscale-v2/`
- `docs/benchmarks/email-stack/20260430Tscale-v3/`
- `docs/benchmarks/email-stack/20260430Tbench-v1/`

นี่คือวิธีที่ควร publish: claim ให้แคบ, แนบ raw evidence, แล้วค่อย ๆ tighten server กับ workload ถัดไป.
