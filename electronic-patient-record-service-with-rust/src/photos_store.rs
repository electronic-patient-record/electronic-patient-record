//! Patient photos (the banner photo, task ID-7): append-only versions in `patient_photos`.
//!
//! A photo is identity data, so it is read and written under the same rules as the identity record
//! (role, relationship, purpose, consent) and every access is audited by the caller. The store
//! checks the bytes, not the claim: the image type comes from the file's magic bytes (PNG or JPEG
//! only), the size is capped, and the stored SHA-256 lets a reader see that the bytes are those
//! that were uploaded. No decoding or resizing happens here: nothing parses the image, so a
//! malformed one cannot reach an image library, and the browser is told not to sniff it.
//!
//! Withdrawing a photo is a new version with no bytes; earlier versions stay in the table (and so
//! in backups), which is the same promise the rest of the record makes.

use crate::patients_store::{self, StoreError};
use sea_orm::{ConnectionTrait, DbBackend, Statement, TransactionSession, TransactionTrait, Value};
use sha2::{Digest, Sha256};

/// The largest photo accepted, in bytes.
pub const MAX_BYTES: usize = 512 * 1024;

/// A photo version as stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredPhoto {
    /// The patient.
    pub patient_id: String,
    /// 1 for the first photo version.
    pub version: i32,
    /// Who wrote this version.
    pub author: String,
    /// `image/png` or `image/jpeg`; `None` when this version withdraws the photo.
    pub content_type: Option<String>,
    /// The image bytes; `None` when withdrawn.
    pub bytes: Option<Vec<u8>>,
}

fn pg(sql: &str, values: Vec<Value>) -> Statement {
    Statement::from_sql_and_values(DbBackend::Postgres, sql, values)
}

fn db_err(e: impl std::fmt::Display) -> StoreError {
    StoreError::Db(e.to_string())
}

/// The image type of `bytes` from its magic number, or `None` if it is neither PNG nor JPEG.
#[must_use]
pub fn sniff(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]) {
        Some("image/png")
    } else if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some("image/jpeg")
    } else {
        None
    }
}

/// Why a photo is refused, or `Ok` with its type.
///
/// # Errors
///
/// A message for an empty, oversized or unrecognised file.
pub fn validate(bytes: &[u8]) -> Result<&'static str, String> {
    if bytes.is_empty() {
        return Err("the photo is empty".into());
    }
    if bytes.len() > MAX_BYTES {
        return Err(format!("the photo is larger than {} KiB", MAX_BYTES / 1024));
    }
    sniff(bytes).ok_or_else(|| "the photo must be a PNG or a JPEG".into())
}

fn row_to_photo(row: &sea_orm::QueryResult) -> Result<StoredPhoto, StoreError> {
    Ok(StoredPhoto {
        patient_id: row.try_get("", "patient_id").map_err(db_err)?,
        version: row.try_get("", "version").map_err(db_err)?,
        author: row.try_get("", "author").map_err(db_err)?,
        content_type: row.try_get("", "content_type").map_err(db_err)?,
        bytes: row.try_get("", "bytes").map_err(db_err)?,
    })
}

const COLUMNS: &str = "patient_id::text AS patient_id, version, author, content_type, bytes";

/// The current photo version of a patient, or `None` if there never was one. A withdrawn photo is
/// returned as a version with no bytes.
///
/// # Errors
///
/// [`StoreError::NotFound`] when the patient does not exist.
pub async fn current(
    db: &impl ConnectionTrait,
    patient_id: &str,
) -> Result<Option<StoredPhoto>, StoreError> {
    let patient = patients_store::current(db, patient_id).await?;
    let row = db
        .query_one_raw(pg(
            &format!("SELECT {COLUMNS} FROM patient_photo_current WHERE patient_id = $1::uuid"),
            vec![patient.id.into()],
        ))
        .await
        .map_err(db_err)?;
    row.as_ref().map(row_to_photo).transpose()
}

/// Append a photo version. `expected_version` is the photo version the caller last saw (0 for
/// none); anything else is a [`StoreError::Conflict`]. `bytes` of `None` withdraws the photo.
///
/// # Errors
///
/// [`StoreError::NotFound`] (no such patient), [`StoreError::BadQuery`] (the file is refused),
/// [`StoreError::Conflict`] (stale or racing version).
pub async fn put(
    db: &(impl ConnectionTrait + TransactionTrait),
    patient_id: &str,
    expected_version: i32,
    author: &str,
    reason: Option<&str>,
    bytes: Option<&[u8]>,
) -> Result<StoredPhoto, StoreError> {
    let kind = match bytes {
        Some(b) => Some(validate(b).map_err(StoreError::BadQuery)?),
        None => None,
    };
    let now = current(db, patient_id).await?;
    let have = now.as_ref().map_or(0, |p| p.version);
    if have != expected_version {
        return Err(StoreError::Conflict { current: have });
    }
    if bytes.is_none() && now.as_ref().is_none_or(|p| p.bytes.is_none()) {
        return Err(StoreError::BadQuery("there is no photo to withdraw".into()));
    }
    let id = patients_store::current(db, patient_id).await?.id;
    let sha = bytes.map(|b| {
        Sha256::digest(b).iter().fold(String::new(), |mut s, x| {
            use std::fmt::Write as _;
            let _ = write!(s, "{x:02x}");
            s
        })
    });
    let sp = db.begin().await.map_err(db_err)?;
    let inserted = sp
        .query_one_raw(pg(
            &format!(
                "INSERT INTO patient_photos
                   (patient_id, version, author, reason, content_type, bytes, sha256)
                 VALUES ($1::uuid, $2, $3, $4, $5, $6, $7) RETURNING {COLUMNS}"
            ),
            vec![
                id.into(),
                (expected_version + 1).into(),
                author.into(),
                reason.map(str::to_string).into(),
                kind.map(str::to_string).into(),
                bytes.map(<[u8]>::to_vec).into(),
                sha.into(),
            ],
        ))
        .await;
    let inserted = match inserted {
        Ok(v) => {
            sp.commit().await.map_err(db_err)?;
            Ok(v)
        }
        Err(e) => {
            let _ = sp.rollback().await;
            Err(e)
        }
    };
    match inserted {
        Ok(Some(row)) => row_to_photo(&row),
        Ok(None) => Err(StoreError::Db("insert returned no row".into())),
        Err(e) if e.to_string().contains("patient_photos_pkey") => {
            let current = current(db, patient_id).await?.map_or(0, |p| p.version);
            Err(StoreError::Conflict { current })
        }
        Err(e) => Err(db_err(e)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PNG: &[u8] = &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0];
    const JPG: &[u8] = &[0xFF, 0xD8, 0xFF, 0xE0, 0, 0x10];

    #[test]
    fn the_type_comes_from_the_bytes_not_a_claim() {
        assert_eq!(sniff(PNG), Some("image/png"));
        assert_eq!(sniff(JPG), Some("image/jpeg"));
        for not_an_image in [
            &b"<svg xmlns='http://www.w3.org/2000/svg'><script>alert(1)</script></svg>"[..],
            b"GIF89a....",
            b"<html><script>",
            b"%PDF-1.7",
            b"\x89PNG", // truncated magic
            b"",
        ] {
            assert_eq!(sniff(not_an_image), None, "{not_an_image:?}");
        }
    }

    #[test]
    fn size_and_emptiness_are_checked() {
        assert_eq!(validate(PNG), Ok("image/png"));
        assert!(validate(&[]).is_err());
        let mut big = PNG.to_vec();
        big.resize(MAX_BYTES, 0);
        assert!(validate(&big).is_ok());
        big.push(0);
        assert!(validate(&big).unwrap_err().contains("larger than 512 KiB"));
        assert!(validate(b"hello").unwrap_err().contains("PNG or a JPEG"));
    }
}
