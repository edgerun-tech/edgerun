export const locales = ["en", "et", "th"] as const
export type Locale = (typeof locales)[number]

export interface LocaleCopy {
  navHome: string
  navMethodology: string
  navBenchmarks: string
  navEnglish: string
  navEstonian: string
  navThai: string
  homeTitle: string
  homeSubtitle: string
  homeLead: string
  homePrimaryCta: string
  homeSecondaryCta: string
  methodologyTitle: string
  methodologyIntro: string
  resultsTitle: string
  resultsIntro: string
  methodologyTemplate: {
    title: string
    sourceOfTruth: string
    fairnessClause: string
    reproducibility: string
    confidence: string
    reporting: string
  }
  fairnessColumns: {
    metric: string
    weight: string
    score: string
    confidence: string
    notes: string
  }
}

export const localeDictionary: Record<Locale, LocaleCopy> = {
  en: {
    navHome: "Home",
    navMethodology: "Methodology",
    navBenchmarks: "Benchmarks",
    navEnglish: "English",
    navEstonian: "Eesti",
    navThai: "ไทย",
    homeTitle: "EdgeRun Performance Methodology",
    homeSubtitle: "Technical results with auditable transparency.",
    homeLead:
      "We publish only measurement-driven evidence. Commands are irrelevant without provenance; methodology is first-class truth.",
    homePrimaryCta: "Read methodology",
    homeSecondaryCta: "View results",
    methodologyTitle: "How we measure",
    methodologyIntro:
      "Every dataset, run condition, and metric is versioned. If a claim cannot be reproduced by the public templates and raw command log, it never ships as a benchmark fact.",
    resultsTitle: "Latest benchmark runs",
    resultsIntro: "Fair templates for comparing throughput, latency, and resource behavior.",
    methodologyTemplate: {
      title: "Template: Fair Comparison",
      sourceOfTruth: "1. Source of truth is a signed run manifest + raw logs.",
      fairnessClause: "2. Comparable workloads only, same input seeds and duration.",
      reproducibility:
        "3. Reproducibility requires three independent runs with CI metadata exported.",
      confidence: "4. Confidence must include variance (MAD and 95% CI).",
      reporting:
        "5. Raw totals are normalized and displayed with both absolute and percentile scores.",
    },
    fairnessColumns: {
      metric: "Metric",
      weight: "Weight",
      score: "Weighted score",
      confidence: "Confidence",
      notes: "Notes",
    },
  },
  et: {
    navHome: "Avaleht",
    navMethodology: "Meetodoloogia",
    navBenchmarks: "Mõõtmised",
    navEnglish: "Inglise",
    navEstonian: "Eesti",
    navThai: "Tai",
    homeTitle: "EdgeRuni jõudlusmeetod",
    homeSubtitle: "Tehnilised tulemused auditeeritava läbipaistvusega.",
    homeLead:
      "Avaldame ainult mõõtmistel põhinevaid tõendeid. Ilma allikandmeteta pole väide fakti.",
    homePrimaryCta: "Loe meetodit",
    homeSecondaryCta: "Vaata tulemusi",
    methodologyTitle: "Kuidas me mõõdame",
    methodologyIntro:
      "Kõik andmekogumid, jooksutustingimused ja metrikad on versioonitud. Kui väidet ei saa avalike mallide ja logide abil korrata, ei esitata seda faktina.",
    resultsTitle: "Uusimad mõõtmised",
    resultsIntro:
      "Õiglased mallid läbilaskevõime, latentsuse ja ressursikulu võrdlemiseks.",
    methodologyTemplate: {
      title: "Mall: Õiglane võrdlus",
      sourceOfTruth: "1. Õigusallikas on allkirjastatud testimismanifest ja toorlogid.",
      fairnessClause: "2. Võrreldakse ainult samade töökoormuste jooksutusi.",
      reproducibility:
        "3. Korduvust kinnitavad kolm sõltumatut jooksutust CI metaga.",
      confidence: "4. Kinnitus sisaldab hajuvust: MAD ja 95% usaldusvahemikku.",
      reporting: "5. Põhiandmed normaliseeritakse absolut- ja protsentiliikmetena.",
    },
    fairnessColumns: {
      metric: "Mõõdik",
      weight: "Kaal",
      score: "Kaalutud skoor",
      confidence: "Kindlus",
      notes: "Märkused",
    },
  },
  th: {
    navHome: "หน้าแรก",
    navMethodology: "วิธีการ",
    navBenchmarks: "ผลการทดสอบ",
    navEnglish: "อังกฤษ",
    navEstonian: "เอสโตเนีย",
    navThai: "ไทย",
    homeTitle: "ระเบียบวิธีการวัดประสิทธิภาพ",
    homeSubtitle: "เผยแพร่ข้อมูลเชิงเทคนิคที่ตรวจสอบได้อย่างโปร่งใส",
    homeLead:
      "เราเผยแพร่เฉพาะหลักฐานที่วัดได้จริง ไม่มีผลการวัดที่ติดตามย้อนกลับไม่ได้จะไม่เป็นข้อเท็จจริง",
    homePrimaryCta: "อ่านระเบียบวิธี",
    homeSecondaryCta: "ดูผลลัพธ์",
    methodologyTitle: "เราใช้อย่างไร",
    methodologyIntro:
      "ข้อมูลเซต, เงื่อนไขการรัน และเมตริกทั้งหมดถูกจัดเวอร์ชัน หากข้ออ้างไม่สามารถทำซ้ำผ่านเทมเพลตสาธารณะและบันทึกคำสั่งได้ จะไม่แสดงเป็นความจริง",
    resultsTitle: "ผลล่าสุด",
    resultsIntro:
      "เทมเพลตที่เป็นธรรมสำหรับเปรียบเทียบ throughput, latency และการใช้ทรัพยากร",
    methodologyTemplate: {
      title: "เทมเพลต: การเปรียบเทียบที่เป็นธรรม",
      sourceOfTruth: "1. แหล่งข้อมูลที่เชื่อถือได้คือ manifest การรันที่เซ็นรับรองและบันทึกดิบ",
      fairnessClause: "2. เปรียบเทียบเฉพาะ workload เทียบเท่ากันด้วย seed และเวลาเท่ากัน",
      reproducibility: "3. ต้องมีการรันทดลองซ้ำอย่างน้อย 3 ครั้งและใส่ metadata CI",
      confidence: "4. ต้องแสดงความไม่แน่นอนด้วย MAD และ CI 95%",
      reporting: "5. ค่าเดิมถูก normalization และแสดงทั้งค่าจริงและเปอร์เซนไทล์",
    },
    fairnessColumns: {
      metric: "เมตริก",
      weight: "น้ำหนัก",
      score: "สกอร์ถ่วงน้ำหนัก",
      confidence: "ความเชื่อมั่น",
      notes: "หมายเหตุ",
    },
  },
}

export const localeDefault: Locale = "en"

export const getLocaleCopy = (locale: string | undefined): LocaleCopy => {
  return localeDictionary[(locale as Locale) ?? localeDefault] ?? localeDictionary.en
}
