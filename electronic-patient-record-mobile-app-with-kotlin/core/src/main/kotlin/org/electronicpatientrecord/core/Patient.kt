package org.electronicpatientrecord.core

import kotlinx.serialization.KSerializer
import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import kotlinx.serialization.SerializationException
import kotlinx.serialization.descriptors.PrimitiveKind
import kotlinx.serialization.descriptors.PrimitiveSerialDescriptor
import kotlinx.serialization.descriptors.SerialDescriptor
import kotlinx.serialization.encoding.Decoder
import kotlinx.serialization.encoding.Encoder
import kotlinx.serialization.json.Json

// The service's patient identity shapes, as `GET /api/patients` answers them. The field names are the matcher's `Patient`
// (electronic-patient-record-matcher-with-rust/src/patient.rs) and the service's `Stored` (src/patients_store.rs). Every
// list may be omitted or null and reads as empty; every single value may be omitted or null and reads as absent (`null`),
// never as an empty string or zero.

/** A person's name. */
@Serializable
public data class HumanName(
    /** The family name, if known. */
    val family: String? = null,
    /** The given names, in order. */
    val given: List<String> = emptyList(),
)

/** An identifier such as a national health number. */
@Serializable
public data class Identifier(
    /** The identifier's system URI. */
    val system: String,
    /** The value within that system. */
    val value: String,
)

/** The administrative sex. Absent (`null`) is not the same as [UNKNOWN]. */
@Serializable(with = AdministrativeSexSerializer::class)
public enum class AdministrativeSex(
    /** The service's word. */
    public val wire: String,
) {
    /** Male. */
    MALE("male"),

    /** Female. */
    FEMALE("female"),

    /** Another administrative sex. */
    OTHER("other"),

    /** Not known, or no usable answer was given. */
    UNKNOWN("unknown"),
}

/** Reads the service's word strictly: an unknown value is an error, never coerced to absent or to [AdministrativeSex.UNKNOWN]. */
internal object AdministrativeSexSerializer : KSerializer<AdministrativeSex> {
    override val descriptor: SerialDescriptor = PrimitiveSerialDescriptor("AdministrativeSex", PrimitiveKind.STRING)

    override fun deserialize(decoder: Decoder): AdministrativeSex {
        val word = decoder.decodeString()
        return AdministrativeSex.entries.firstOrNull { it.wire == word }
            ?: throw SerializationException("unknown administrative sex")
    }

    override fun serialize(encoder: Encoder, value: AdministrativeSex) {
        encoder.encodeString(value.wire)
    }
}

/** A patient identity record. */
@Serializable
public data class Patient(
    /** The record's id once stored. */
    val id: String? = null,
    /** The person's names; the first is the one shown. */
    val names: List<HumanName> = emptyList(),
    /** The birth date as stored: `YYYY`, `YYYY-MM` or `YYYY-MM-DD`. */
    @SerialName("birth_date") val birthDate: String? = null,
    /** The administrative sex, if recorded. */
    val sex: AdministrativeSex? = null,
    /** Identifiers. */
    val identifiers: List<Identifier> = emptyList(),
    /** The names and birth date are placeholders for someone not yet identified. */
    @SerialName("temporary_identity") val temporaryIdentity: Boolean = false,
)

/** One stored version of a patient. */
@Serializable
public data class StoredPatient(
    /** The server's id. */
    val id: String,
    /** 1 for the first version. */
    val version: Int,
    /** Who wrote this version. */
    val author: String,
    /** The identity record. */
    val patient: Patient,
    /** A drill or test record, not a real person. */
    val drill: Boolean = false,
)

/** The answer to `GET /api/patients`. */
@Serializable
public data class PatientSearchResults(
    /** The matches, up to [limit]. */
    val results: List<StoredPatient>,
    /** The most results the service returns. */
    val limit: Int,
) {
    public companion object {
        // coerceInputValues turns a null list into its empty default; the sex serializer is a string, so it is not coerced.
        private val json = Json {
            ignoreUnknownKeys = true
            coerceInputValues = true
        }

        /** Reads the service's JSON. @throws SerializationException when it does not fit. */
        public fun decode(text: String): PatientSearchResults = json.decodeFromString(serializer(), text)
    }
}
