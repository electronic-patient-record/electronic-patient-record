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
  "download",
  "downloadCut",
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
  download: STRINGS.accessLogDownload,
  downloadCut: STRINGS.accessLogDownloadCut,
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
    download: "Als CSV-Datei herunterladen (bis zu 1000 neueste Einträge)",
    downloadCut: "Die Datei enthält nur die 1000 neuesten Einträge. Grenzen Sie die Tage ein, um ältere zu erhalten.",
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
    download: "Descargar como archivo CSV (hasta las 1000 entradas más recientes)",
    downloadCut: "El archivo solo contiene las 1000 entradas más recientes. Acote los días para obtener las anteriores.",
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
    download: "Télécharger en fichier CSV (jusqu'à 1000 entrées les plus récentes)",
    downloadCut: "Le fichier ne contient que les 1000 entrées les plus récentes. Réduisez les jours pour obtenir les plus anciennes.",
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
    download: "下载为 CSV 文件(最多最近 1000 条记录)",
    downloadCut: "文件仅包含最近的 1000 条记录。请缩小日期范围以获取更早的记录。",
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
    download: "CSV फ़ाइल के रूप में डाउनलोड करें (सबसे हाल की 1000 प्रविष्टियाँ तक)",
    downloadCut: "फ़ाइल में केवल सबसे हाल की 1000 प्रविष्टियाँ हैं। पुरानी प्रविष्टियों के लिए दिनों को सीमित करें।",
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
    download: "تنزيل كملف CSV (حتى أحدث 1000 إدخال)",
    downloadCut: "يحتوي الملف على أحدث 1000 إدخال فقط. ضيّق الأيام للحصول على الإدخالات الأقدم.",
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
    download: "Lawrlwytho fel ffeil CSV (hyd at y 1000 cofnod diweddaraf)",
    downloadCut: "Dim ond y 1000 cofnod diweddaraf sydd yn y ffeil. Cyfyngwch y dyddiau i gael rhai hŷn.",
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

// The reasons a link cannot be automatic (CQ-38), in the words of each locale. The matcher sends each
// reason as a code with its numbers (`conflict_codes`) beside its English text; the page translates the
// code and falls back to the English text for anything it does not know.
//
// MACHINE-WRITTEN, not reviewed by any speaker, and this is clinical wording (a registrar reads it
// when two records may be one person): do not call it reviewed until a clinician who speaks the
// language has read it (hazard H-001).
export const CONFLICT_KEYS = [
  "differentNhsNumbers",
  "sameNhsDifferentBirthDates",
  "sameNhsDifferentNames",
  "namesSoundAlike",
  "namesSwapped",
  "extraGivenName",
  "fieldUnavailable",
  "fieldBelowMinimum",
  "evidenceThin",
  "givenName",
  "familyName",
  "birthDate",
  "notSamePerson",
] as const;
export type ConflictWords = Record<(typeof CONFLICT_KEYS)[number], string>;

export const CONFLICT_ENGLISH: ConflictWords = {
  differentNhsNumbers: "different verified NHS numbers",
  sameNhsDifferentBirthDates: "same NHS number but different birth dates",
  sameNhsDifferentNames: "same NHS number but different names",
  namesSoundAlike: "names sound alike but are spelled differently",
  namesSwapped: "given and family names appear swapped",
  extraGivenName: "one record has a given name the other lacks",
  fieldUnavailable: "{field} not available to confirm the link",
  fieldBelowMinimum: "{field} similarity {similarity} below {minimum}",
  evidenceThin: "only {percent}% of the evidence is present",
  givenName: "given name",
  familyName: "family name",
  birthDate: "birth date",
  notSamePerson: "These do not look like the same person:",
};

const conflict_de: ConflictWords = {
  differentNhsNumbers: "unterschiedliche geprüfte NHS-Nummern",
  sameNhsDifferentBirthDates: "gleiche NHS-Nummer, aber unterschiedliche Geburtsdaten",
  sameNhsDifferentNames: "gleiche NHS-Nummer, aber unterschiedliche Namen",
  namesSoundAlike: "Namen klingen ähnlich, sind aber unterschiedlich geschrieben",
  namesSwapped: "Vor- und Nachname scheinen vertauscht",
  extraGivenName: "ein Datensatz hat einen Vornamen, der dem anderen fehlt",
  fieldUnavailable: "{field} nicht verfügbar, um die Verknüpfung zu bestätigen",
  fieldBelowMinimum: "{field}: Ähnlichkeit {similarity} unter {minimum}",
  evidenceThin: "nur {percent} % der Hinweise sind vorhanden",
  givenName: "Vorname",
  familyName: "Nachname",
  birthDate: "Geburtsdatum",
  notSamePerson: "Das scheinen nicht dieselben Personen zu sein:",
};

const conflict_es: ConflictWords = {
  differentNhsNumbers: "números NHS verificados distintos",
  sameNhsDifferentBirthDates: "mismo número NHS pero fechas de nacimiento distintas",
  sameNhsDifferentNames: "mismo número NHS pero nombres distintos",
  namesSoundAlike: "los nombres suenan parecido pero se escriben distinto",
  namesSwapped: "el nombre y el apellido parecen intercambiados",
  extraGivenName: "un registro tiene un nombre de pila que el otro no tiene",
  fieldUnavailable: "{field} no disponible para confirmar el vínculo",
  fieldBelowMinimum: "{field}: similitud {similarity} por debajo de {minimum}",
  evidenceThin: "solo el {percent} % de la evidencia está presente",
  givenName: "nombre de pila",
  familyName: "apellido",
  birthDate: "fecha de nacimiento",
  notSamePerson: "No parecen ser la misma persona:",
};

const conflict_fr: ConflictWords = {
  differentNhsNumbers: "numéros NHS vérifiés différents",
  sameNhsDifferentBirthDates: "même numéro NHS mais dates de naissance différentes",
  sameNhsDifferentNames: "même numéro NHS mais noms différents",
  namesSoundAlike: "les noms se prononcent de façon proche mais s'écrivent différemment",
  namesSwapped: "le prénom et le nom semblent inversés",
  extraGivenName: "un dossier a un prénom que l'autre n'a pas",
  fieldUnavailable: "{field} non disponible pour confirmer le lien",
  fieldBelowMinimum: "{field} : similarité {similarity} inférieure à {minimum}",
  evidenceThin: "seulement {percent} % des éléments sont présents",
  givenName: "prénom",
  familyName: "nom de famille",
  birthDate: "date de naissance",
  notSamePerson: "Ce ne semble pas être la même personne :",
};

const conflict_zh: ConflictWords = {
  differentNhsNumbers: "已验证的 NHS 编号不同",
  sameNhsDifferentBirthDates: "NHS 编号相同但出生日期不同",
  sameNhsDifferentNames: "NHS 编号相同但姓名不同",
  namesSoundAlike: "姓名发音相近但拼写不同",
  namesSwapped: "名和姓似乎颠倒了",
  extraGivenName: "其中一条记录有另一条没有的名字",
  fieldUnavailable: "{field}不可用,无法确认关联",
  fieldBelowMinimum: "{field}相似度 {similarity} 低于 {minimum}",
  evidenceThin: "仅有 {percent}% 的证据",
  givenName: "名",
  familyName: "姓",
  birthDate: "出生日期",
  notSamePerson: "这看起来不是同一个人:",
};

const conflict_hi: ConflictWords = {
  differentNhsNumbers: "सत्यापित NHS नंबर अलग हैं",
  sameNhsDifferentBirthDates: "NHS नंबर वही है लेकिन जन्म तिथियाँ अलग हैं",
  sameNhsDifferentNames: "NHS नंबर वही है लेकिन नाम अलग हैं",
  namesSoundAlike: "नाम सुनने में मिलते हैं लेकिन वर्तनी अलग है",
  namesSwapped: "पहला और अंतिम नाम आपस में बदले हुए लगते हैं",
  extraGivenName: "एक रिकॉर्ड में एक पहला नाम है जो दूसरे में नहीं है",
  fieldUnavailable: "लिंक की पुष्टि के लिए {field} उपलब्ध नहीं है",
  fieldBelowMinimum: "{field} समानता {similarity}, {minimum} से कम",
  evidenceThin: "केवल {percent}% प्रमाण मौजूद हैं",
  givenName: "पहला नाम",
  familyName: "अंतिम नाम",
  birthDate: "जन्म तिथि",
  notSamePerson: "ये एक ही व्यक्ति नहीं लगते:",
};

const conflict_ar: ConflictWords = {
  differentNhsNumbers: "أرقام NHS الموثّقة مختلفة",
  sameNhsDifferentBirthDates: "رقم NHS نفسه لكن تواريخ الميلاد مختلفة",
  sameNhsDifferentNames: "رقم NHS نفسه لكن الأسماء مختلفة",
  namesSoundAlike: "الأسماء متشابهة في النطق لكنها مكتوبة بشكل مختلف",
  namesSwapped: "يبدو أن الاسم الأول واسم العائلة متبادلان",
  extraGivenName: "في أحد السجلين اسم أول ليس في الآخر",
  fieldUnavailable: "{field} غير متوفر لتأكيد الربط",
  fieldBelowMinimum: "{field}: التشابه {similarity} أقل من {minimum}",
  evidenceThin: "تتوفر {percent}% فقط من الأدلة",
  givenName: "الاسم الأول",
  familyName: "اسم العائلة",
  birthDate: "تاريخ الميلاد",
  notSamePerson: "لا يبدو أنهما الشخص نفسه:",
};

const conflict_cy: ConflictWords = {
  differentNhsNumbers: "rhifau NHS gwiriedig gwahanol",
  sameNhsDifferentBirthDates: "yr un rhif NHS ond dyddiadau geni gwahanol",
  sameNhsDifferentNames: "yr un rhif NHS ond enwau gwahanol",
  namesSoundAlike: "mae'r enwau'n swnio'n debyg ond wedi'u sillafu'n wahanol",
  namesSwapped: "mae'n ymddangos bod yr enw cyntaf a'r cyfenw wedi'u cyfnewid",
  extraGivenName: "mae gan un cofnod enw cyntaf nad yw ar y llall",
  fieldUnavailable: "nid yw {field} ar gael i gadarnhau'r cyswllt",
  fieldBelowMinimum: "{field}: tebygrwydd {similarity} o dan {minimum}",
  evidenceThin: "dim ond {percent}% o'r dystiolaeth sy'n bresennol",
  givenName: "enw cyntaf",
  familyName: "cyfenw",
  birthDate: "dyddiad geni",
  notSamePerson: "Nid yw'r rhain yn edrych fel yr un person:",
};

/** Machine-written, not reviewed. `de-de` reads as `de-001`, and `zh-cn` as `zh-001` (simplified). */
export const CONFLICT_WORDS: Partial<Record<Locale, ConflictWords>> = {
  "ar-001": conflict_ar,
  "cy-001": conflict_cy,
  "de-001": conflict_de,
  "de-de": conflict_de,
  "es-001": conflict_es,
  "fr-001": conflict_fr,
  "hi-001": conflict_hi,
  "zh-001": conflict_zh,
  "zh-cn": conflict_zh,
};

/** The conflict words for `locale`, English where there is no translation. */
export function conflictWords(locale: Locale): ConflictWords {
  return CONFLICT_WORDS[locale] ?? CONFLICT_ENGLISH;
}
