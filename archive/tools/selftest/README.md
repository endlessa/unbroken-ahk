# OCR self-test

Confirms the scanned-document path still works after changes to triage,
rasterization, or image sizing. Transcription is done by reading the rendered
pages, so the test is run interactively rather than by CI.

```sh
python3 -I archive/tools/selftest/make-fixture.py archive/raw/_octest
python3 -I archive/tools/ocr.py triage _octest        # expect: flagged, ~0 chars/page
python3 -I archive/tools/ocr.py rasterize _octest     # expect: 2 page images
#   read archive/ocr/_octest/opinion-24cv01983/page-*.png
#   write each transcript to page-N.txt beside the image
python3 -I archive/tools/ocr.py assemble _octest
python3 -I archive/tools/selftest/score.py \
    archive/tools/selftest/expected.txt \
    archive/text/_octest/opinion-24cv01983.txt
```

`score.py` reports token accuracy and prints every mismatch. Last recorded
run: **328/328 tokens, 100.00%**, at 200 dpi greyscale downscaled to 1600 px
wide. Clean up with `rm -rf archive/{raw,ocr,text}/_octest`.
