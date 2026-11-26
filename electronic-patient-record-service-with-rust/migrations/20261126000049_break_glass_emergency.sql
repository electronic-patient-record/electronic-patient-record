-- Migration: what the person who opens break-the-glass says about the emergency (RF-6, PI-10). Four answers, each their
-- word: what is at risk, how soon, whether the patient can be asked, and what the patient said when they could. A grant
-- opened before this migration has none, and reads as "not asked"; a new grant has all three closed answers, and the
-- words of what was said exactly when the patient could be asked. The table stays append-only (its triggers are
-- unchanged); adding columns writes no row.

ALTER TABLE break_glass_grants
    ADD COLUMN threat TEXT CHECK (threat IN ('life', 'limb', 'neither', 'not-known')),
    ADD COLUMN time_to_harm TEXT CHECK (time_to_harm IN ('minutes', 'hours', 'days', 'not-known')),
    ADD COLUMN can_be_asked TEXT CHECK (can_be_asked IN ('yes', 'no', 'not-known')),
    ADD COLUMN asked_and_said TEXT CHECK (asked_and_said IS NULL OR (length(btrim(asked_and_said)) > 0 AND length(asked_and_said) <= 500)),
    ADD CONSTRAINT break_glass_grants_answered CHECK (
        (threat IS NULL) = (time_to_harm IS NULL) AND (threat IS NULL) = (can_be_asked IS NULL)
        AND (asked_and_said IS NOT NULL) = (can_be_asked IS NOT DISTINCT FROM 'yes')
    );
