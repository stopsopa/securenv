# test010 — `##` comment encryption + ENCODEMASK filtering + round-trip

Tests the following requirements:

- `ENCODEMASK` is non-empty (`^SECRET_`) so only `SECRET_*` vars are encrypted.
- `PUBLIC_URL` does **not** match the mask → copied unchanged.
- Regular single-`#` comments are always copied unchanged.
- Every `##` comment variant is encrypted (payload only, prefix kept plaintext):
  - `##content` — no space between `##` and content
  - `##  two spaces` — two spaces
  - `##\tone tab` — one tab
  - `##\t\ttwo tabs` — two tabs
  - `##\t space` — tab then space
  - `## \ttab` — space then tab
  - `  ##` — two leading spaces before hashes
  - `\t##` — leading tab before hashes
  - `###` — triple hash (extra `#` is payload)
  - `####` — four hashes (extra `##` are payload)
- Lines that are `##` followed only by spaces/tabs are **ignored** (passed through).
- Round-trip: encrypt then decrypt must restore original plaintext exactly.
