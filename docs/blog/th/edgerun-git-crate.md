---
title: edgerun-git: controlled source release จาก checkout เดียวกัน
date: 2026-04-30
author: Ken
summary: Git และ crate explorer ที่อยู่หลัง public code surface ของ Edgerun, สร้างมาเพื่อ release แบบเลือกส่วน ไม่ใช่เปิดทั้ง platform ทีเดียว.
tags: [edgerun, crates, git, email]
---
# edgerun-git: controlled source release จาก checkout เดียวกัน

`edgerun-git` คือ Git และ crate explorer ขนาดเล็กสำหรับ public code surface.
มันไม่ใช่ full forge. ไม่มี issues, pull requests, accounts, CI pages หรือ
social features.

นี่เป็น design ที่ตั้งใจ. เป้าหมายตอนนี้คือ controlled release: แสดงเฉพาะส่วน
ที่อธิบายระบบที่กำลังรันใน production โดยเฉพาะ email-facing platform code โดย
ไม่ต้องเปิดทุกส่วนที่ยัง internal หรือ experimental.

## Visibility Model

Implemented in code:

- repository ถูกซ่อนจนกว่า `.edgerun/git.yaml` จะกำหนด `visible: true`;
- files และ directories ถูกซ่อนจนกว่าจะ release ด้วย `.gitvisible` markers;
- generated crate metadata อ่านจาก `.edgerun/git/crates/*.txt`;
- dashboard render repository, crate, API, call-edge, test และ related RFC summaries จาก released surface;
- raw source tree และ generated crate explorer ใช้ visibility policy เดียวกัน.

ตอนนี้ public slice ตั้งใจให้เล็ก: blog crate, Git crate และ blog content. แค่นี้
พออธิบาย build log และ source explorer เอง โดยไม่เปิด platform internals ที่ไม่
เกี่ยวข้อง.

## เทียบกับทางเลือกอื่น

เทียบกับ source browser แบบ `cgit`, `edgerun-git` opinionated กว่าเรื่อง
release boundaries. มันไม่ใช่ "ชี้ไปที่ repo แล้ว browse ได้ทุกอย่าง".
Visibility เป็นส่วนหนึ่งของ repo.

เทียบกับ full forge มันเล็กกว่ามาก. ไม่มี project management surface และไม่มี
account system ให้ operate. สำหรับ deployment นี้ feature เหล่านั้นเป็น moving
parts เพิ่มเติมรอบความต้องการที่เรียบง่าย: ให้คนอ่าน inspect code ที่รองรับ
public claim.

เทียบกับ hosted code pages, source publication ยังอยู่บนเครื่องเดียวกันและใน
operational model เดียวกับ email server. สำหรับ Edgerun เรื่องสำคัญไม่ใช่แค่
code แต่คือการรัน mail, web, blog และ code surfaces ร่วมกันโดยไม่ต้องมี stack
ขนาดใหญ่ข้างหลัง.
