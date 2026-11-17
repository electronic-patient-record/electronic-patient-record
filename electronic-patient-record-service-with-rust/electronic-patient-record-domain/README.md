# electronic-patient-record-domain

The domain rules of the electronic patient record service: what the record allows, refuses and works out, with no
database and no web. Policy and consent (who may see what), the referral state table, care pathways and
suspicion rules, what makes an observation, result, adjustment, amendment, death report or assessment valid, the
hazard register's rules, the site's time zone and the clock.

**NOT FOR CLINICAL USE.** Part of a scaffold; see the repository's `AGENTS.md` and `spec/`.

The service crate, `electronic-patient-record`, stores, serves and audits, and re-exports every module here under
its old path. This crate depends on no database, web framework or async runtime, so its tests run without
Postgres:

```sh
cargo test -p electronic-patient-record-domain
```

Licence: MIT OR Apache-2.0 OR BSD-3-Clause OR GPL-2.0-only OR GPL-3.0-only.
