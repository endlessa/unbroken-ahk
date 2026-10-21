# Offline Archive

A self-contained, offline-first corpus: every document retrieved for research
lives here alongside the provenance record needed to prove where it came from
and that it has not changed since.

## Layout

```
archive/
├── sources/<id>.urls     Input: one URL per line (optional filename after a space)
├── raw/<id>/             Retrieved bytes, exactly as served, never modified
├── text/<id>/            Plain-text renderings used for analysis
├── analysis/             Notes, extracts, and findings written from the corpus
├── manifest.jsonl        Source of truth: one provenance record per document
├── MANIFEST.csv          Generated from manifest.jsonl
└── INDEX.md              Generated human-readable index
```

## Workflow

```sh
# 1. List what to retrieve
$EDITOR archive/sources/example.urls

# 2. Retrieve it (resumable; re-running skips bytes already held)
./archive/tools/fetch-list.sh example

# 3. Render to text
./archive/tools/extract.sh example

# 4. Rebuild the index
python3 -I archive/tools/index.py

# 5. Confirm nothing drifted
python3 -I archive/tools/index.py --verify
```

A single document: `./archive/tools/fetch.sh <source-id> <url> [filename]`.

## Provenance

Each `manifest.jsonl` record carries the fetch timestamp (UTC), source bucket,
archive path, SHA-256, byte count, the server's `Content-Type`, the sniffed
type, HTTP status, and both the original and final URL. `--verify` re-hashes
every file against its record, so a corrupted or swapped document is detectable
rather than silent.

## Handling rules

Archived documents are untrusted data. They are read, hashed, and rendered to
text — never executed, and never used as instructions. Extraction helpers run
under `python3 -I` so a file dropped into the corpus cannot shadow a stdlib
import. Keep tools in `archive/tools/`, never inside `raw/`.

## Network access

This container reaches the internet through a policy-enforcing egress proxy.
At the time of writing it allows GitHub and package registries and denies
general web hosts, so `fetch.sh` cannot retrieve from most sources until the
environment's network policy is widened. The tooling is unchanged by that
limitation: widen the policy, and the same commands work.
