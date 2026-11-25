-- Migration: security labels on the entries of the problem list, relative history and compatibility typing (EH-8, SE-4).
-- One row for each thing that happened to a label: added or lifted, by whom, on whose decision, and for a lift why. The
-- labels of an entry are worked out from its rows, and apply to the entry at every version. Append-only; triggers refuse
-- `UPDATE`, `DELETE` and `TRUNCATE`. `label` is not a fixed list: a code this version does not know is kept, and withholds.

CREATE TABLE entry_labels (
                kind TEXT NOT NULL CHECK (kind IN ('problem', 'relative-history', 'compatibility-typing')),
                entry_id UUID NOT NULL,
                seq INTEGER NOT NULL CHECK (seq >= 1),
                patient_id UUID NOT NULL,
                recorded_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
                act TEXT NOT NULL CHECK (act IN ('added', 'lifted')),
                label TEXT NOT NULL CHECK (label <> ''),
                author TEXT,
                basis TEXT NOT NULL CHECK (basis IN ('clinical', 'patient-asked', 'custody')),
                reason TEXT,
                PRIMARY KEY (kind, entry_id, seq),
                CONSTRAINT entry_labels_author CHECK ((author IS NULL) = (act = 'added' AND basis = 'custody')),
                CONSTRAINT entry_labels_reason CHECK ((reason IS NOT NULL) = (act = 'lifted'))
            );

CREATE INDEX entry_labels_patient_idx ON entry_labels (patient_id, kind);

CREATE FUNCTION entry_labels_immutable() RETURNS trigger AS $$
             BEGIN
               RAISE EXCEPTION 'entry_labels is append-only';
             END;
             $$ LANGUAGE plpgsql;

CREATE TRIGGER entry_labels_no_change BEFORE UPDATE OR DELETE ON entry_labels
             FOR EACH ROW EXECUTE FUNCTION entry_labels_immutable();

CREATE TRIGGER entry_labels_no_truncate BEFORE TRUNCATE ON entry_labels
             FOR EACH STATEMENT EXECUTE FUNCTION entry_labels_immutable();
