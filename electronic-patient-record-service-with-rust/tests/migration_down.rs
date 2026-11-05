//! The clinical record is not rolled back (task CQ-15). Migrations are `.sql` files with no down files, so
//! there is nothing to run: `db down` refuses, and a new down file fails here. Database-free.

use electronic_patient_record::MIGRATOR;
use std::path::Path;

#[test]
fn no_migration_can_be_rolled_back() {
    assert!(MIGRATOR.iter().count() >= 26, "the migrator is empty");
    for migration in MIGRATOR.iter() {
        assert!(
            !migration.migration_type.is_down_migration(),
            "{} has a down migration",
            migration.description
        );
    }
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("migrations");
    for entry in std::fs::read_dir(dir).unwrap() {
        let name = entry.unwrap().file_name().to_string_lossy().into_owned();
        assert!(
            !name.contains(".down."),
            "{name}: the clinical record is not rolled back"
        );
    }
}
