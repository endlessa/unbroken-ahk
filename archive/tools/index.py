"""Build INDEX.md and MANIFEST.csv from manifest.jsonl, and verify hashes.

    python3 -I archive/tools/index.py [--verify]

manifest.jsonl is the source of truth; both generated files are derived from it
and are safe to delete and rebuild.
"""
import csv
import hashlib
import json
import os
import sys

ARCHIVE = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
FIELDS = ["fetched_utc", "source_id", "path", "sha256", "bytes", "content_type",
          "sniffed_type", "http_status", "final_url", "original_url"]


def load():
    path = os.path.join(ARCHIVE, "manifest.jsonl")
    if not os.path.exists(path):
        return []
    with open(path, encoding="utf-8") as fh:
        return [json.loads(line) for line in fh if line.strip()]


def human(n):
    for unit in ("B", "KB", "MB", "GB"):
        if n < 1024 or unit == "GB":
            return f"{n:.0f} {unit}" if unit == "B" else f"{n:.1f} {unit}"
        n /= 1024


def verify(records):
    bad = []
    for rec in records:
        full = os.path.join(ARCHIVE, rec["path"])
        if not os.path.exists(full):
            bad.append((rec["path"], "missing"))
            continue
        digest = hashlib.sha256()
        with open(full, "rb") as fh:
            for chunk in iter(lambda: fh.read(1 << 20), b""):
                digest.update(chunk)
        if digest.hexdigest() != rec["sha256"]:
            bad.append((rec["path"], "hash mismatch"))
    for path, why in bad:
        print(f"BAD  {why:14} {path}")
    print(f"verified {len(records) - len(bad)}/{len(records)} documents")
    return 1 if bad else 0


def write_csv(records):
    with open(os.path.join(ARCHIVE, "MANIFEST.csv"), "w", newline="", encoding="utf-8") as fh:
        writer = csv.DictWriter(fh, fieldnames=FIELDS)
        writer.writeheader()
        for rec in sorted(records, key=lambda r: (r["source_id"], r["path"])):
            writer.writerow({k: rec.get(k, "") for k in FIELDS})


def write_index(records):
    by_source = {}
    for rec in records:
        by_source.setdefault(rec["source_id"], []).append(rec)
    total_bytes = sum(r["bytes"] for r in records)

    lines = ["# Archive Index", "",
             f"{len(records)} documents across {len(by_source)} sources, "
             f"{human(total_bytes)} on disk.", "",
             "| Source | Documents | Size | Earliest fetch | Latest fetch |",
             "|---|---:|---:|---|---|"]
    for source in sorted(by_source):
        recs = by_source[source]
        stamps = sorted(r["fetched_utc"] for r in recs)
        lines.append(f"| `{source}` | {len(recs)} | {human(sum(r['bytes'] for r in recs))} "
                     f"| {stamps[0]} | {stamps[-1]} |")

    for source in sorted(by_source):
        lines += ["", f"## {source}", "",
                  "| Document | Size | Type | Retrieved from |", "|---|---:|---|---|"]
        for rec in sorted(by_source[source], key=lambda r: r["path"]):
            name = os.path.basename(rec["path"])
            lines.append(f"| [`{name}`]({rec['path']}) | {human(rec['bytes'])} "
                         f"| {rec.get('sniffed_type', '')} | <{rec['original_url']}> |")

    with open(os.path.join(ARCHIVE, "INDEX.md"), "w", encoding="utf-8") as fh:
        fh.write("\n".join(lines) + "\n")


def main():
    records = load()
    if "--verify" in sys.argv:
        sys.exit(verify(records))
    write_csv(records)
    write_index(records)
    print(f"indexed {len(records)} documents")


if __name__ == "__main__":
    main()
