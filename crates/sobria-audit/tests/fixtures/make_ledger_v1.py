"""Builds ledger-v1.sqlite: an audit ledger as written before the `method` column (schema v1).

Each signature is SHA-256 of "<timestamp>|<payload>|<prev_sig>" (src/entry.rs), computed here
without the Rust code, so the fixture also checks that the current code reads a legacy chain.

    python crates/sobria-audit/tests/fixtures/make_ledger_v1.py
"""

import hashlib
import os
import sqlite3

OUT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "ledger-v1.sqlite")

# (timestamp as chrono's to_rfc3339 writes it, payload, purged_at). Row 3 is purged: its stored
# payload becomes "PURGED" but its signature, computed from the original payload, is kept.
ROWS = [
    (
        "2026-05-12T14:32:08.123456789+00:00",
        '{"method":"afnor_sobria","request":{"model_id":"gpt-4o-mini","tokens_in":100,"tokens_out_estimated":500},"co2eq_p50":2.14,"seed":42}',
        None,
    ),
    (
        "2026-05-12T14:35:41.500+00:00",
        '{"method":"afnor_sobria","request":{"model_id":"claude-3-5-sonnet","tokens_in":850,"tokens_out_estimated":1200},"co2eq_p50":9.7,"seed":42}',
        None,
    ),
    (
        "2026-05-13T09:02:17+00:00",
        '{"method":"afnor_sobria","request":{"model_id":"gpt-4o","tokens_in":40,"tokens_out_estimated":300},"co2eq_p50":1.05,"seed":42}',
        "2026-05-20T08:00:00+00:00",
    ),
    (
        "2026-05-14T18:47:55.000001+00:00",
        '{"method":"afnor_sobria","request":{"model_id":"mistral-large","tokens_in":2000,"tokens_out_estimated":800},"co2eq_p50":6.3,"seed":42}',
        None,
    ),
]


def sig(timestamp, payload, prev_sig):
    return hashlib.sha256(f"{timestamp}|{payload}|{prev_sig}".encode()).hexdigest()


for suffix in ("", "-wal", "-shm"):
    if os.path.exists(OUT + suffix):
        os.remove(OUT + suffix)

con = sqlite3.connect(OUT)
con.execute("PRAGMA journal_mode = WAL")
con.executescript(
    """
    CREATE TABLE audit_entries (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        timestamp TEXT NOT NULL,
        estimation_result_json TEXT NOT NULL,
        prev_sig TEXT NOT NULL,
        sig TEXT NOT NULL,
        purged_at TEXT
    );
    CREATE INDEX idx_audit_timestamp ON audit_entries(timestamp);
    """
)
prev_sig = ""
for timestamp, payload, purged_at in ROWS:
    entry_sig = sig(timestamp, payload, prev_sig)
    stored = "PURGED" if purged_at else payload
    con.execute(
        "INSERT INTO audit_entries (timestamp, estimation_result_json, prev_sig, sig, purged_at) "
        "VALUES (?, ?, ?, ?, ?)",
        (timestamp, stored, prev_sig, entry_sig, purged_at),
    )
    prev_sig = entry_sig
con.commit()
con.close()
