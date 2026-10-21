"""Vision-OCR pipeline for scanned documents in the archive.

Scanned PDFs carry no text layer, so pdftotext returns little or nothing for
them. This tool finds those documents, renders their pages to images sized for
reading, tracks which pages still need transcription, and assembles finished
per-page transcripts back into archive/text/.

    python3 -I archive/tools/ocr.py triage [source-id]
    python3 -I archive/tools/ocr.py rasterize <source-id> [--dpi 200]
    python3 -I archive/tools/ocr.py status [source-id]
    python3 -I archive/tools/ocr.py assemble <source-id>

Transcription itself happens between rasterize and assemble: the page images
are read and their text written to page-NNNN.txt beside each image.
"""
import argparse
import json
import os
import shutil
import subprocess
import sys

ARCHIVE = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
RAW = os.path.join(ARCHIVE, "raw")
TEXT = os.path.join(ARCHIVE, "text")
OCR = os.path.join(ARCHIVE, "ocr")
WORKLIST = os.path.join(OCR, "worklist.jsonl")

# A text layer thinner than this per page means the PDF is a scan, not text.
CHARS_PER_PAGE_FLOOR = 120
# Wide enough that small print and footnotes stay legible once rendered.
MAX_WIDTH = 1600


def run(cmd):
    return subprocess.run(cmd, capture_output=True, text=True, check=False)


def pdf_pages(path):
    out = run(["pdfinfo", path]).stdout
    for line in out.splitlines():
        if line.startswith("Pages:"):
            return int(line.split()[1])
    return 0


def pdf_text_chars(path):
    out = run(["pdftotext", "-enc", "UTF-8", path, "-"]).stdout
    return len("".join(out.split()))


def iter_pdfs(scope):
    root = os.path.join(RAW, scope) if scope else RAW
    for dirpath, _, names in os.walk(root):
        for name in sorted(names):
            full = os.path.join(dirpath, name)
            if run(["file", "-b", "--mime-type", full]).stdout.strip() == "application/pdf":
                yield full


def doc_key(pdf):
    rel = os.path.relpath(pdf, RAW)
    source = rel.split(os.sep)[0]
    stem = os.path.splitext(os.path.basename(rel))[0]
    return source, stem, rel


def load_worklist():
    if not os.path.exists(WORKLIST):
        return {}
    with open(WORKLIST, encoding="utf-8") as fh:
        return {r["raw_path"]: r for r in (json.loads(l) for l in fh if l.strip())}


def save_worklist(entries):
    os.makedirs(OCR, exist_ok=True)
    with open(WORKLIST, "w", encoding="utf-8") as fh:
        for key in sorted(entries):
            fh.write(json.dumps(entries[key], sort_keys=True) + "\n")


def cmd_triage(args):
    entries = load_worklist()
    found = 0
    for pdf in iter_pdfs(args.source):
        source, stem, rel = doc_key(pdf)
        pages = pdf_pages(pdf)
        chars = pdf_text_chars(pdf)
        density = chars / pages if pages else 0
        if density >= CHARS_PER_PAGE_FLOOR:
            continue
        found += 1
        entries[rel] = {
            "raw_path": rel, "source_id": source, "doc": stem,
            "pages": pages, "text_chars": chars,
            "chars_per_page": round(density, 1), "status": "needs-ocr",
        }
        print(f"SCAN  {pages:4d}p  {density:6.1f} chars/page  {rel}")
    save_worklist(entries)
    print(f"\n{found} scanned document(s) flagged; worklist at ocr/worklist.jsonl")


def cmd_rasterize(args):
    entries = load_worklist()
    todo = [e for e in entries.values()
            if e["source_id"] == args.source and e["status"] in ("needs-ocr", "rasterized")]
    if not todo:
        print(f"nothing flagged for {args.source}; run triage first")
        return
    try:
        from PIL import Image
    except ImportError:
        Image = None

    for entry in todo:
        pdf = os.path.join(RAW, entry["raw_path"])
        outdir = os.path.join(OCR, entry["source_id"], entry["doc"])
        os.makedirs(outdir, exist_ok=True)
        if not any(n.endswith(".png") for n in os.listdir(outdir)):
            run(["pdftoppm", "-png", "-r", str(args.dpi), "-gray", pdf,
                 os.path.join(outdir, "page")])
        pages = sorted(n for n in os.listdir(outdir) if n.endswith(".png"))
        if Image:
            for name in pages:
                path = os.path.join(outdir, name)
                with Image.open(path) as img:
                    if img.width > MAX_WIDTH:
                        height = round(img.height * MAX_WIDTH / img.width)
                        img.resize((MAX_WIDTH, height), Image.LANCZOS).save(path, optimize=True)
        entry["status"] = "rasterized"
        entry["page_images"] = len(pages)
        print(f"RENDER {len(pages):4d} images  {entry['source_id']}/{entry['doc']}")
    save_worklist(entries)
    print("\nread the page images and write each transcript to page-NNNN.txt beside it")


def cmd_status(args):
    entries = [e for e in load_worklist().values()
               if not args.source or e["source_id"] == args.source]
    if not entries:
        print("worklist empty")
        return
    total_pages = done_pages = 0
    for entry in sorted(entries, key=lambda e: (e["source_id"], e["doc"])):
        outdir = os.path.join(OCR, entry["source_id"], entry["doc"])
        imgs = len([n for n in os.listdir(outdir) if n.endswith(".png")]) if os.path.isdir(outdir) else 0
        txts = len([n for n in os.listdir(outdir) if n.endswith(".txt")]) if os.path.isdir(outdir) else 0
        total_pages += imgs
        done_pages += txts
        bar = "done" if imgs and txts >= imgs else f"{txts}/{imgs}"
        print(f"{entry['status']:11} {bar:>9}  {entry['source_id']}/{entry['doc']}")
    print(f"\n{done_pages}/{total_pages} pages transcribed across {len(entries)} document(s)")


def cmd_assemble(args):
    entries = load_worklist()
    written = 0
    for entry in entries.values():
        if entry["source_id"] != args.source:
            continue
        outdir = os.path.join(OCR, entry["source_id"], entry["doc"])
        if not os.path.isdir(outdir):
            continue
        parts = sorted(n for n in os.listdir(outdir) if n.endswith(".txt"))
        imgs = sorted(n for n in os.listdir(outdir) if n.endswith(".png"))
        if not parts or len(parts) < len(imgs):
            print(f"SKIP  {entry['doc']}: {len(parts)}/{len(imgs)} pages transcribed")
            continue
        dest = os.path.join(TEXT, entry["source_id"], entry["doc"] + ".txt")
        os.makedirs(os.path.dirname(dest), exist_ok=True)
        with open(dest, "w", encoding="utf-8") as out:
            out.write(f"[vision-transcribed from {entry['raw_path']}, "
                      f"{len(parts)} pages; no text layer in source]\n\n")
            for name in parts:
                page = os.path.splitext(name)[0].split("-")[-1]
                with open(os.path.join(outdir, name), encoding="utf-8") as fh:
                    out.write(f"--- page {page} ---\n{fh.read().rstrip()}\n\n")
        entry["status"] = "transcribed"
        written += 1
        print(f"WROTE text/{entry['source_id']}/{entry['doc']}.txt  ({len(parts)} pages)")
    save_worklist(entries)
    print(f"\nassembled {written} document(s)")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="cmd", required=True)
    p = sub.add_parser("triage"); p.add_argument("source", nargs="?"); p.set_defaults(fn=cmd_triage)
    p = sub.add_parser("rasterize"); p.add_argument("source"); p.add_argument("--dpi", type=int, default=200); p.set_defaults(fn=cmd_rasterize)
    p = sub.add_parser("status"); p.add_argument("source", nargs="?"); p.set_defaults(fn=cmd_status)
    p = sub.add_parser("assemble"); p.add_argument("source"); p.set_defaults(fn=cmd_assemble)
    args = parser.parse_args()
    if not shutil.which("pdftoppm"):
        sys.exit("pdftoppm not found; poppler-utils is required")
    args.fn(args)


if __name__ == "__main__":
    main()
