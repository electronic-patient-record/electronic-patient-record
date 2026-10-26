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

/** The refusals to merge or unmerge (CQ-38); `{id}` is the other record. Machine-written, not reviewed. */
export type RefusalWords = Record<"reasonRequired" | "sameRecord" | "drillAndReal" | "notMerged" | "sourceAlreadyMerged" | "targetAlreadyMerged", string>;

export const REFUSAL_ENGLISH: RefusalWords = {
  reasonRequired: "A reason is required.",
  sameRecord: "A record cannot be merged into itself.",
  drillAndReal: "A drill record and a real record can never be merged.",
  notMerged: "That record is not currently merged into this one.",
  sourceAlreadyMerged: "The record to merge away was already merged into {id}.",
  targetAlreadyMerged: "The surviving record was itself merged into {id}: merge into that one.",
};

const refusal_de: RefusalWords = {
  reasonRequired: "Ein Grund ist erforderlich.",
  sameRecord: "Ein Datensatz kann nicht mit sich selbst zusammengeführt werden.",
  drillAndReal: "Ein Übungsdatensatz und ein echter Datensatz dürfen nie zusammengeführt werden.",
  notMerged: "Dieser Datensatz ist derzeit nicht in diesen zusammengeführt.",
  sourceAlreadyMerged: "Der aufzulösende Datensatz wurde bereits mit {id} zusammengeführt.",
  targetAlreadyMerged: "Der bestehende Datensatz wurde selbst mit {id} zusammengeführt: führen Sie mit diesem zusammen.",
};

const refusal_es: RefusalWords = {
  reasonRequired: "Se requiere un motivo.",
  sameRecord: "Un registro no puede fusionarse consigo mismo.",
  drillAndReal: "Un registro de simulacro y un registro real nunca pueden fusionarse.",
  notMerged: "Ese registro no está fusionado actualmente con este.",
  sourceAlreadyMerged: "El registro que se iba a fusionar ya estaba fusionado con {id}.",
  targetAlreadyMerged: "El registro que se conserva ya fue fusionado con {id}: fusione con ese.",
};

const refusal_fr: RefusalWords = {
  reasonRequired: "Un motif est obligatoire.",
  sameRecord: "Un dossier ne peut pas être fusionné avec lui-même.",
  drillAndReal: "Un dossier d'exercice et un dossier réel ne peuvent jamais être fusionnés.",
  notMerged: "Ce dossier n'est pas actuellement fusionné avec celui-ci.",
  sourceAlreadyMerged: "Le dossier à fusionner a déjà été fusionné avec {id}.",
  targetAlreadyMerged: "Le dossier conservé a lui-même été fusionné avec {id} : fusionnez avec celui-là.",
};

const refusal_zh: RefusalWords = {
  reasonRequired: "必须填写原因。",
  sameRecord: "记录不能与自身合并。",
  drillAndReal: "演练记录与真实记录永远不能合并。",
  notMerged: "该记录当前没有合并到此记录中。",
  sourceAlreadyMerged: "要并入的记录已合并到 {id}。",
  targetAlreadyMerged: "保留的记录本身已合并到 {id}:请合并到那一条。",
};

const refusal_hi: RefusalWords = {
  reasonRequired: "कारण आवश्यक है।",
  sameRecord: "कोई रिकॉर्ड स्वयं में मर्ज नहीं किया जा सकता।",
  drillAndReal: "अभ्यास रिकॉर्ड और वास्तविक रिकॉर्ड को कभी मर्ज नहीं किया जा सकता।",
  notMerged: "वह रिकॉर्ड अभी इस रिकॉर्ड में मर्ज नहीं है।",
  sourceAlreadyMerged: "जिस रिकॉर्ड को मर्ज करना था, वह पहले ही {id} में मर्ज हो चुका है।",
  targetAlreadyMerged: "बचा हुआ रिकॉर्ड स्वयं {id} में मर्ज हो चुका है: उसी में मर्ज करें।",
};

const refusal_ar: RefusalWords = {
  reasonRequired: "السبب مطلوب.",
  sameRecord: "لا يمكن دمج سجل مع نفسه.",
  drillAndReal: "لا يمكن أبدًا دمج سجل تدريبي مع سجل حقيقي.",
  notMerged: "هذا السجل غير مدمج حاليًا في هذا السجل.",
  sourceAlreadyMerged: "السجل المراد دمجه سبق دمجه في {id}.",
  targetAlreadyMerged: "السجل المتبقي نفسه مدمج في {id}: ادمج في ذلك السجل.",
};

const refusal_cy: RefusalWords = {
  reasonRequired: "Mae angen rheswm.",
  sameRecord: "Ni ellir uno cofnod ag ef ei hun.",
  drillAndReal: "Ni ellir byth uno cofnod ymarfer â chofnod go iawn.",
  notMerged: "Nid yw'r cofnod hwnnw wedi'i uno â'r un hwn ar hyn o bryd.",
  sourceAlreadyMerged: "Roedd y cofnod i'w uno eisoes wedi'i uno â {id}.",
  targetAlreadyMerged: "Roedd y cofnod sy'n aros wedi'i uno ag {id}: unwch â'r un hwnnw.",
};

export const REFUSAL_WORDS: Partial<Record<Locale, RefusalWords>> = {
  "ar-001": refusal_ar,
  "cy-001": refusal_cy,
  "de-001": refusal_de,
  "de-de": refusal_de,
  "es-001": refusal_es,
  "fr-001": refusal_fr,
  "hi-001": refusal_hi,
  "zh-001": refusal_zh,
  "zh-cn": refusal_zh,
};

/** The refusal words for `locale`, English where there is no translation. */
export function refusalWords(locale: Locale): RefusalWords {
  return REFUSAL_WORDS[locale] ?? REFUSAL_ENGLISH;
}

/** The reasons a patient record is refused (CQ-38, 422 `error_codes`). Machine-written, not reviewed. */
export type PatientProblemWords = Record<
  "familyNameRequired" | "birthDateInvalid" | "nhsNumberPlaceholder" | "nhsNumberCheckDigit" | "identifierIncomplete",
  string
>;

export const PATIENT_PROBLEM_ENGLISH: PatientProblemWords = {
  familyNameRequired: "At least one name with a family name is required.",
  birthDateInvalid: "The birth date must be a year, a year and month, or a real date.",
  nhsNumberPlaceholder: "The NHS number is a placeholder (ten copies of one digit), not an issued number.",
  nhsNumberCheckDigit: "The NHS number fails the check-digit test.",
  identifierIncomplete: "An identifier needs a system and a value.",
};

const problem_de: PatientProblemWords = {
  familyNameRequired: "Mindestens ein Name mit Familiennamen ist erforderlich.",
  birthDateInvalid: "Das Geburtsdatum muss ein Jahr, ein Jahr mit Monat oder ein echtes Datum sein.",
  nhsNumberPlaceholder: "Die NHS-Nummer ist ein Platzhalter (zehnmal dieselbe Ziffer), keine vergebene Nummer.",
  nhsNumberCheckDigit: "Die NHS-Nummer besteht die Prüfziffernprüfung nicht.",
  identifierIncomplete: "Eine Kennung braucht ein System und einen Wert.",
};

const problem_es: PatientProblemWords = {
  familyNameRequired: "Se requiere al menos un nombre con apellido.",
  birthDateInvalid: "La fecha de nacimiento debe ser un año, un año y mes, o una fecha real.",
  nhsNumberPlaceholder: "El número NHS es un marcador (diez copias de un mismo dígito), no un número asignado.",
  nhsNumberCheckDigit: "El número NHS no supera la comprobación del dígito de control.",
  identifierIncomplete: "Un identificador necesita un sistema y un valor.",
};

const problem_fr: PatientProblemWords = {
  familyNameRequired: "Au moins un nom avec un nom de famille est obligatoire.",
  birthDateInvalid: "La date de naissance doit être une année, une année et un mois, ou une date réelle.",
  nhsNumberPlaceholder: "Le numéro NHS est un numéro factice (dix fois le même chiffre), pas un numéro attribué.",
  nhsNumberCheckDigit: "Le numéro NHS échoue au contrôle de la clé.",
  identifierIncomplete: "Un identifiant a besoin d'un système et d'une valeur.",
};

const problem_zh: PatientProblemWords = {
  familyNameRequired: "至少需要一个带姓氏的姓名。",
  birthDateInvalid: "出生日期必须是年份、年月或真实日期。",
  nhsNumberPlaceholder: "NHS 号码是占位符(同一数字重复十次),不是已签发的号码。",
  nhsNumberCheckDigit: "NHS 号码未通过校验位检查。",
  identifierIncomplete: "标识符需要系统和值。",
};

const problem_hi: PatientProblemWords = {
  familyNameRequired: "कम से कम एक नाम, उपनाम के साथ, आवश्यक है।",
  birthDateInvalid: "जन्म तिथि एक वर्ष, वर्ष और महीना, या वास्तविक तिथि होनी चाहिए।",
  nhsNumberPlaceholder: "NHS नंबर एक प्लेसहोल्डर है (एक ही अंक दस बार), जारी किया गया नंबर नहीं।",
  nhsNumberCheckDigit: "NHS नंबर चेक-डिजिट जाँच में विफल है।",
  identifierIncomplete: "पहचानकर्ता के लिए सिस्टम और मान दोनों आवश्यक हैं।",
};

const problem_ar: PatientProblemWords = {
  familyNameRequired: "مطلوب اسم واحد على الأقل مع اسم العائلة.",
  birthDateInvalid: "يجب أن يكون تاريخ الميلاد سنة، أو سنة وشهرًا، أو تاريخًا حقيقيًا.",
  nhsNumberPlaceholder: "رقم NHS عنصر نائب (عشر نسخ من رقم واحد) وليس رقمًا صادرًا.",
  nhsNumberCheckDigit: "رقم NHS لا يجتاز فحص رقم التحقق.",
  identifierIncomplete: "يحتاج المعرّف إلى نظام وقيمة.",
};

const problem_cy: PatientProblemWords = {
  familyNameRequired: "Mae angen o leiaf un enw gydag enw teuluol.",
  birthDateInvalid: "Rhaid i'r dyddiad geni fod yn flwyddyn, yn flwyddyn a mis, neu'n ddyddiad go iawn.",
  nhsNumberPlaceholder: "Dim ond llanwr yw rhif GIG (deg copi o'r un digid), nid rhif a gyhoeddwyd.",
  nhsNumberCheckDigit: "Nid yw rhif GIG yn pasio'r prawf digid gwirio.",
  identifierIncomplete: "Mae angen system a gwerth ar ddynodydd.",
};

export const PATIENT_PROBLEM_WORDS: Partial<Record<Locale, PatientProblemWords>> = {
  "ar-001": problem_ar,
  "cy-001": problem_cy,
  "de-001": problem_de,
  "de-de": problem_de,
  "es-001": problem_es,
  "fr-001": problem_fr,
  "hi-001": problem_hi,
  "zh-001": problem_zh,
  "zh-cn": problem_zh,
};

/** The patient-problem words for `locale`, English where there is no translation. */
export function patientProblemWords(locale: Locale): PatientProblemWords {
  return PATIENT_PROBLEM_WORDS[locale] ?? PATIENT_PROBLEM_ENGLISH;
}
