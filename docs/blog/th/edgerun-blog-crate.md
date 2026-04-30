---
title: edgerun-blog: publish โดยไม่ต้องมี CMS แยก
date: 2026-04-30
author: Ken
summary: crate ขนาดเล็กแบบ first-party ที่อยู่หลัง build log ของ Edgerun และเหตุผลที่มันอยู่ใน server เดียวกับ mail และ code.
tags: [edgerun, crates, blog, email]
---
# edgerun-blog: publish โดยไม่ต้องมี CMS แยก

Build log นี้ไม่ได้ใช้ hosted CMS, database หรือ static-site SaaS แยกต่างหาก.
มันถูก serve โดย `edgerun-blog`, crate แบบ host-only ที่อ่าน Markdown จาก Git
checkout เดียวกับ code.

นี่เป็นหนึ่งใน platform pieces ที่กำลังช่วย production email deployment ของ
Edgerun อยู่ตอนนี้. Scope ตั้งใจให้เล็ก: publish notes, render posts, expose
feed และให้ dashboard embed content เดียวกันเป็น workspace surface.

## สิ่งที่ทำได้ตอนนี้

Implemented in code:

- อ่าน Markdown และ HTML posts จาก `docs/blog`;
- public posts ต้องมี English, Thai และ Estonian ครบ;
- validate front matter สำหรับ title, date, author, summary และ tags;
- render post pages, index pages, feeds, search metadata, sitemap, robots และ app shell assets;
- serve จาก checkout โดยตรง หรือ generate deterministic static output;
- link ชื่อ crate ที่เปิด public แล้วไปยัง code explorer.

เป้าหมายไม่ใช่แทนที่ CMS เต็มรูปแบบ. เป้าหมายคือ production mail host สามารถ
publish release notes ได้โดยไม่ต้องเพิ่ม database, plugin runtime, admin panel
หรือ external analytics script.

## เทียบกับทางเลือกอื่น

เทียบกับ static-site generator ทั่วไป `edgerun-blog` แคบกว่า. มันไม่ได้พยายาม
เป็น theme ecosystem. Layout, feed, metadata, language rules และ dashboard
fragments ถูกทำมาเพื่อ publishing workflow เดียว.

เทียบกับ database-backed CMS มันมี runtime state น้อยกว่า. Source of truth คือ
Git checkout. Review, rollback และ deployment ใช้เส้นทางเดียวกับ code.

เทียบกับ HTML ที่เขียนเอง มันย้ายงาน safety ซ้ำ ๆ เข้าไปอยู่ใน code: front
matter checks, escaping, feeds, sitemap entries, canonical URLs และ language
routes.

นี่เข้ากับ release strategy ตอนนี้: publish มากพอให้ project เข้าใจได้ โดยไม่
ต้องเปิด operational surface ใหญ่เพื่อ host แค่ไม่กี่หน้า.
