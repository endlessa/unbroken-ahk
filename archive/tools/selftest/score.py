import difflib, re, sys

def toks(path, strip_markers=False):
    text = open(path, encoding="utf-8").read()
    if strip_markers:
        text = re.sub(r"^\[vision-transcribed.*?\]$", "", text, flags=re.M)
        text = re.sub(r"^--- page \d+ ---$", "", text, flags=re.M)
        text = text.replace("_", " ")
    return re.findall(r"\S+", text)

truth = toks(sys.argv[1])
got = toks(sys.argv[2], strip_markers=True)
sm = difflib.SequenceMatcher(None, truth, got, autojunk=False)
match = sum(b.size for b in sm.get_matching_blocks())
print(f"ground truth tokens : {len(truth)}")
print(f"transcript tokens   : {len(got)}")
print(f"matching tokens     : {match}")
print(f"token accuracy      : {100*match/len(truth):.2f}%")
for tag, i1, i2, j1, j2 in sm.get_opcodes():
    if tag != "equal":
        print(f"  {tag:8} truth[{i1}:{i2}]={truth[i1:i2]!r:60.60} got[{j1}:{j2}]={got[j1:j2]!r:.60}")
