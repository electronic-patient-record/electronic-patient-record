#!/bin/sh
# Gate service-checked-arithmetic (LC-3): the review greps of spec/rust-checked-arithmetic (RCA-ENF-5, RCA-ENF-7).
set -eu
cd "$(dirname "$0")/../.."
python3 scripts/check-checked-arithmetic.py
