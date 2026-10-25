// Translations of the access-log filter form on `/my-record` (SE-6).
//
// MACHINE-WRITTEN. No speaker of any language here has reviewed these, and nothing may call them
// reviewed until one has (AGENTS.md). The rest of the UI is still English only; a locale with no
// entry, or a missing key, falls back to English through `logFormStrings`.
import { STRINGS } from "./strings.js";
import type { Locale } from "./locales.js";

export const LOG_FORM_KEYS = [
  "filterTitle",
  "zone",
  "from",
  "to",
  "by",
  "byAny",
  "byYou",
  "byStaff",
  "outcome",
  "outcomeAny",
  "refusedOnly",
  "okOnly",
  "purposeFilter",
  "purposeAny",
  "apply",
  "clear",
  "badDay",
] as const;

export type LogFormStrings = Record<(typeof LOG_FORM_KEYS)[number], string> & {
  purposes: Record<keyof typeof STRINGS.accessLogPurposes, string>;
};

/** English, from the one table the rest of the UI reads. */
const en: LogFormStrings = {
  filterTitle: STRINGS.accessLogFilterTitle,
  zone: STRINGS.accessLogZone,
  from: STRINGS.accessLogFrom,
  to: STRINGS.accessLogTo,
  by: STRINGS.accessLogBy,
  byAny: STRINGS.accessLogByAny,
  byYou: STRINGS.accessLogByYou,
  byStaff: STRINGS.accessLogByStaff,
  outcome: STRINGS.accessLogOutcome,
  outcomeAny: STRINGS.accessLogOutcomeAny,
  refusedOnly: STRINGS.accessLogRefusedOnly,
  okOnly: STRINGS.accessLogOkOnly,
  purposeFilter: STRINGS.accessLogPurposeFilter,
  purposeAny: STRINGS.accessLogPurposeAny,
  apply: STRINGS.accessLogApply,
  clear: STRINGS.accessLogClear,
  badDay: STRINGS.accessLogBadDay,
  purposes: STRINGS.accessLogPurposes,
};

const de: LogFormStrings = {
    filterTitle: "Protokoll eingrenzen",
    zone: "Tage und Uhrzeiten hier gelten in Ihrer Zeitzone: {zone}.",
    from: "Von (Tag)",
    to: "Bis einschließlich (Tag)",
    by: "Wer",
    byAny: "Jeder",
    byYou: "Nur ich",
    byStaff: "Nur andere",
    outcome: "Ergebnis",
    outcomeAny: "Alle",
    refusedOnly: "Nur abgelehnte Zugriffsversuche",
    okOnly: "Nur erfolgreiche",
    purposeFilter: "Zweck",
    purposeAny: "Beliebiger Zweck",
    apply: "Anwenden",
    clear: "Zurücksetzen",
    badDay: "Bitte echte Tage im Format 2026-10-21 angeben.",
    purposes: {
      "treatment": "Behandlung",
      "emergency": "Notfall",
      "operations": "Betrieb des Dienstes",
      "payment": "Abrechnung",
      "research": "Forschung",
      "patient-access": "Sie oder Ihr Vertreter beim Ansehen der Akte",
      "audit": "Prüfung des Zugriffsprotokolls",
    },
};

const es: LogFormStrings = {
    filterTitle: "Filtrar el registro",
    zone: "Los días y las horas aquí están en su zona horaria: {zone}.",
    from: "Desde (día)",
    to: "Hasta e incluyendo (día)",
    by: "Quién",
    byAny: "Cualquiera",
    byYou: "Solo yo",
    byStaff: "Solo otras personas",
    outcome: "Resultado",
    outcomeAny: "Todos",
    refusedOnly: "Solo intentos rechazados",
    okOnly: "Solo los que se lograron",
    purposeFilter: "Finalidad",
    purposeAny: "Cualquier finalidad",
    apply: "Aplicar",
    clear: "Borrar",
    badDay: "Use días reales escritos como 2026-10-21.",
    purposes: {
      "treatment": "Tratamiento",
      "emergency": "Emergencia",
      "operations": "Funcionamiento del servicio",
      "payment": "Pago",
      "research": "Investigación",
      "patient-access": "Usted o su representante viendo el historial",
      "audit": "Revisión del registro de auditoría",
    },
};

const fr: LogFormStrings = {
    filterTitle: "Filtrer le journal",
    zone: "Les jours et les heures sont ici dans votre fuseau horaire : {zone}.",
    from: "Du (jour)",
    to: "Jusqu'au (jour) inclus",
    by: "Qui",
    byAny: "N'importe qui",
    byYou: "Moi seulement",
    byStaff: "Les autres seulement",
    outcome: "Résultat",
    outcomeAny: "Tous",
    refusedOnly: "Tentatives refusées seulement",
    okOnly: "Accès réussis seulement",
    purposeFilter: "Finalité",
    purposeAny: "Toute finalité",
    apply: "Appliquer",
    clear: "Effacer",
    badDay: "Saisissez de vrais jours écrits comme 2026-10-21.",
    purposes: {
      "treatment": "Soins",
      "emergency": "Urgence",
      "operations": "Fonctionnement du service",
      "payment": "Paiement",
      "research": "Recherche",
      "patient-access": "Vous ou votre mandataire consultant le dossier",
      "audit": "Revue de la piste d'audit",
    },
};

const zh: LogFormStrings = {
    filterTitle: "筛选记录",
    zone: "此处的日期和时间使用您的时区:{zone}。",
    from: "从(日期)",
    to: "截至并包括(日期)",
    by: "谁",
    byAny: "任何人",
    byYou: "仅我本人",
    byStaff: "仅其他人",
    outcome: "结果",
    outcomeAny: "全部",
    refusedOnly: "仅被拒绝的尝试",
    okOnly: "仅成功的",
    purposeFilter: "目的",
    purposeAny: "任何目的",
    apply: "应用",
    clear: "清除",
    badDay: "请输入真实的日期,格式如 2026-10-21。",
    purposes: {
      "treatment": "治疗",
      "emergency": "紧急情况",
      "operations": "服务运营",
      "payment": "付款",
      "research": "研究",
      "patient-access": "您或您的代理人查看记录",
      "audit": "审查审计记录",
    },
};

const hi: LogFormStrings = {
    filterTitle: "लॉग को सीमित करें",
    zone: "यहाँ दिन और समय आपके समय क्षेत्र में हैं: {zone}।",
    from: "से (दिन)",
    to: "तक और उसमें शामिल (दिन)",
    by: "कौन",
    byAny: "कोई भी",
    byYou: "केवल मैं",
    byStaff: "केवल अन्य लोग",
    outcome: "परिणाम",
    outcomeAny: "सभी",
    refusedOnly: "केवल अस्वीकृत प्रयास",
    okOnly: "केवल सफल",
    purposeFilter: "उद्देश्य",
    purposeAny: "कोई भी उद्देश्य",
    apply: "लागू करें",
    clear: "साफ़ करें",
    badDay: "2026-10-21 जैसे असली दिन लिखें।",
    purposes: {
      "treatment": "उपचार",
      "emergency": "आपातकाल",
      "operations": "सेवा का संचालन",
      "payment": "भुगतान",
      "research": "अनुसंधान",
      "patient-access": "आप या आपके प्रतिनिधि द्वारा रिकॉर्ड देखना",
      "audit": "ऑडिट ट्रेल की समीक्षा",
    },
};

const ar: LogFormStrings = {
    filterTitle: "تضييق السجل",
    zone: "الأيام والأوقات هنا بحسب منطقتك الزمنية: {zone}.",
    from: "من (يوم)",
    to: "حتى (يوم) ضمناً",
    by: "من",
    byAny: "أي شخص",
    byYou: "أنا فقط",
    byStaff: "الآخرون فقط",
    outcome: "النتيجة",
    outcomeAny: "الكل",
    refusedOnly: "المحاولات المرفوضة فقط",
    okOnly: "الناجحة فقط",
    purposeFilter: "الغرض",
    purposeAny: "أي غرض",
    apply: "تطبيق",
    clear: "مسح",
    badDay: "استخدم أياماً حقيقية مكتوبة مثل 2026-10-21.",
    purposes: {
      "treatment": "العلاج",
      "emergency": "الطوارئ",
      "operations": "تشغيل الخدمة",
      "payment": "الدفع",
      "research": "البحث",
      "patient-access": "أنت أو وكيلك تطّلع على السجل",
      "audit": "مراجعة سجل التدقيق",
    },
};

const cy: LogFormStrings = {
    filterTitle: "Cyfyngu'r cofnod",
    zone: "Mae'r dyddiau a'r amserau yma yn eich parth amser chi: {zone}.",
    from: "O (diwrnod)",
    to: "Hyd at a chan gynnwys (diwrnod)",
    by: "Pwy",
    byAny: "Unrhyw un",
    byYou: "Fi yn unig",
    byStaff: "Eraill yn unig",
    outcome: "Canlyniad",
    outcomeAny: "Pob un",
    refusedOnly: "Ymdrechion a wrthodwyd yn unig",
    okOnly: "Rhai llwyddiannus yn unig",
    purposeFilter: "Pwrpas",
    purposeAny: "Unrhyw bwrpas",
    apply: "Gosod",
    clear: "Clirio",
    badDay: "Defnyddiwch ddyddiau go iawn wedi'u hysgrifennu fel 2026-10-21.",
    purposes: {
      "treatment": "Triniaeth",
      "emergency": "Argyfwng",
      "operations": "Rhedeg y gwasanaeth",
      "payment": "Taliad",
      "research": "Ymchwil",
      "patient-access": "Chi neu'ch dirprwy yn edrych ar y cofnod",
      "audit": "Adolygu'r llwybr archwilio",
    },
};

/** Machine-written, not reviewed. `de-de` reads as `de-001`, and `zh-cn` as `zh-001` (simplified). */
export const LOG_FORM: Partial<Record<Locale, LogFormStrings>> = {
  "ar-001": ar,
  "cy-001": cy,
  "de-001": de,
  "de-de": de,
  "es-001": es,
  "fr-001": fr,
  "hi-001": hi,
  "zh-001": zh,
  "zh-cn": zh,
};

/** The filter form's words in `locale`, English where there is no translation. */
export function logFormStrings(locale: Locale): LogFormStrings {
  return LOG_FORM[locale] ?? en;
}

export { en as LOG_FORM_ENGLISH };
