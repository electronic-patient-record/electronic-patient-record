# electronic-patient-record (service)

loco.rs REST service. Scaffold; **not for clinical use**.

```sh
TEST_DB_PORT=5543 TEST_REDIS_PORT=6390 podman compose -f compose.test.yaml up -d --wait
DATABASE_URL=postgres://loco:loco@localhost:5543/electronic_patient_record_test cargo run -- start
curl -s 127.0.0.1:5150/_health
cargo test --lib
```

Development config has login off; any other environment has it on and needs `EPR_PASETO_KEYS` or
`EPR_PASETO_KEYS_URL`. See [`spec/index.md`](spec/index.md).
