"""Build a synthetic scanned PDF for exercising the OCR path.

    python3 -I archive/tools/selftest/make-fixture.py archive/raw/_octest

Produces an image-only PDF — no text layer, slight page rotation, scan
speckle — whose contents match expected.txt. Used to confirm the pipeline
still reads pages correctly after changes to rasterization.
"""
import os
import random
import sys

from PIL import Image, ImageDraw, ImageFont

SERIF = "/usr/share/fonts/truetype/liberation/LiberationSerif-Regular.ttf"
BOLD = "/usr/share/fonts/truetype/liberation/LiberationSerif-Bold.ttf"
MONO = "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf"

PAGES = [
    ("UNITED STATES DISTRICT COURT", "SOUTHERN DISTRICT OF EXAMPLE",
     "Case No. 24-CV-01983-JSR", [
         "MEMORANDUM OPINION AND ORDER", "",
         "Before the Court is Defendant's motion to compel production of the",
         "internal audit workpapers identified in Request No. 14. The motion was",
         "filed on March 3, 2024, and fully briefed as of April 11, 2024.", "",
         "I.  BACKGROUND", "",
         "Plaintiff Ardent Holdings LLC commenced this action on November 8,",
         "2023, alleging that Defendant Calder Systems Inc. misrepresented the",
         "completeness of its reconciliation controls during the diligence period",
         "preceding the asset purchase agreement dated June 30, 2023.", "",
         "The disputed workpapers were prepared by Defendant's internal audit",
         "function between January and May of 2023. Defendant withheld eleven",
         "documents on the basis of attorney-client privilege and the work",
         "product doctrine, listing each on a privilege log served February 20.", "",
         "II. LEGAL STANDARD", "",
         "A party may obtain discovery regarding any nonprivileged matter that is",
         "relevant to any party's claim or defense and proportional to the needs",
         "of the case. Fed. R. Civ. P. 26(b)(1).",
     ]),
    ("", "", "Page 2 of 2", [
        "III. DISCUSSION", "",
        "Having reviewed the eleven documents in camera, the Court concludes",
        "that seven were prepared in the ordinary course of business and are",
        "not protected. Entries 3, 5, and 9 on the privilege log reflect",
        "communications with outside counsel and are properly withheld.", "",
        "Entry 7 presents a closer question. The memorandum dated April 2,",
        "2023 was circulated to both counsel and the operations team. Where a",
        "document serves a dual purpose, the privilege applies only if the",
        "primary purpose was to secure legal advice.", "",
        "The Court finds the primary purpose was operational. Entry 7 shall be",
        "produced, subject to redaction of the final paragraph.", "",
        "IV. CONCLUSION", "",
        "For the reasons stated above, the motion to compel is GRANTED IN PART",
        "and DENIED IN PART. Defendant shall produce documents corresponding to",
        "log entries 1, 2, 4, 6, 7, 8, 10, and 11 within fourteen days.", "",
        "SO ORDERED.", "",
        "Dated:  May 16, 2024              /s/ J. S. Reyes",
        "        Example City, EX          United States District Judge",
    ]),
]


def render(court, district, caseno, body):
    width, height = 1700, 2200
    img = Image.new("L", (width, height), 243)
    draw = ImageDraw.Draw(img)
    f_hdr = ImageFont.truetype(BOLD, 34)
    f_sub = ImageFont.truetype(SERIF, 30)
    f_body = ImageFont.truetype(SERIF, 31)
    f_case = ImageFont.truetype(MONO, 27)

    y = 150
    if court:
        for text, font in ((court, f_hdr), (district, f_sub)):
            draw.text(((width - draw.textlength(text, font=font)) / 2, y),
                      text, font=font, fill=25)
            y += 55
        y += 20
    draw.text((190, y), caseno, font=f_case, fill=35)
    y += 70
    draw.line((190, y, width - 190, y), fill=90, width=2)
    y += 50
    for line in body:
        draw.text((190, y), line, font=f_body, fill=22)
        y += 44

    img = img.rotate(random.uniform(-0.45, 0.45), resample=Image.BICUBIC, fillcolor=243)
    px = img.load()
    for _ in range(2600):
        x, yy = random.randrange(width), random.randrange(height)
        px[x, yy] = max(0, px[x, yy] - random.randrange(40, 120))
    return img.convert("RGB")


def main(outdir):
    os.makedirs(outdir, exist_ok=True)
    imgs = [render(*page) for page in PAGES]
    dest = os.path.join(outdir, "opinion-24cv01983.pdf")
    imgs[0].save(dest, save_all=True, append_images=imgs[1:], resolution=200)
    print(f"wrote {dest} ({len(imgs)} pages, no text layer)")


if __name__ == "__main__":
    main(sys.argv[1] if len(sys.argv) > 1 else "archive/raw/_octest")
